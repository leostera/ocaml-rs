use crate::*;

/// Size is an alias for the platform specific integer type used to store size values
pub type Size = sys::Size;

/// Value wraps the native OCaml `value` type transparently, this means it has the
/// same representation as an `ocaml_sys::Value`
#[derive(Debug, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Value(pub RawOCaml);

impl Clone for Value {
    fn clone(&self) -> Value {
        Value(self.0)
    }
}

unsafe impl ToOCaml<Value> for Value {
    fn to_ocaml(&self, _: AllocToken) -> AllocResult<Value> {
        AllocResult::of(self.0)
    }
}

unsafe impl FromOCaml<Value> for Value {
    fn from_ocaml(v: &OCaml<Value>) -> Value {
        unsafe { Value(v.raw()) }
    }
}

impl<'a, T> From<OCaml<'a, T>> for Value {
    fn from(v: OCaml<'a, T>) -> Value {
        unsafe { Value(v.raw()) }
    }
}

const NONE: Value = Value(sys::NONE);
const UNIT: Value = Value(sys::UNIT);

macro_rules! alloc_value {
    ($rt:expr, $v:expr) => {{
        let v = $v;
        let rt = $rt;
        let t = ocaml_alloc!(v.to_ocaml(rt));
        Value(t.raw())
    }};
}

impl Value {
    #[doc(hidden)]
    pub unsafe fn register(self, rt: &mut Runtime) -> Value {
        let r = alloc_value!(rt, self);
        r
    }

    /// Get OCaml value
    pub unsafe fn ocaml<'a, T>(&'a self, rt: &'a mut Runtime) -> OCaml<'a, T> {
        OCaml::new(rt, self.0)
    }

    /// Get generic OCaml value
    pub unsafe fn of_ocaml<'a, T>(value: &OCaml<'a, T>) -> Value {
        Value(value.raw())
    }

    /// Get Rust value
    pub unsafe fn rust<'a, X, T: FromOCaml<X>>(&'a self, rt: &'a mut Runtime) -> T {
        self.ocaml::<X>(rt).to_rust()
    }

    /// Returns a named value registered by OCaml
    pub unsafe fn named(name: &str) -> Option<Value> {
        let s = match crate::util::CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };
        let named = sys::caml_named_value(s.as_ptr());
        if named.is_null() {
            return None;
        }

        Some(Value(*named))
    }

    /// Allocate a new value with the given size and tag.
    pub unsafe fn alloc(rt: &mut Runtime, n: usize, tag: Tag) -> Value {
        Value(sys::caml_alloc(n, tag.into())).register(rt)
    }

    /// Allocate a new tuple value
    pub unsafe fn alloc_tuple(rt: &mut Runtime, n: usize) -> Value {
        Value(sys::caml_alloc_tuple(n)).register(rt)
    }

    /// Allocate a new small value with the given size and tag
    pub unsafe fn alloc_small(rt: &mut Runtime, n: usize, tag: Tag) -> Value {
        Value(sys::caml_alloc_small(n, tag.into())).register(rt)
    }

    /// Allocate a new value with a finalizer
    ///
    /// This calls `caml_alloc_final` under-the-hood, which can has less than ideal performance
    /// behavior. In most cases you should prefer `Pointer::alloc_custom` when possible.
    pub unsafe fn alloc_final<T>(
        rt: &mut Runtime,
        finalizer: unsafe extern "C" fn(Value),
        cfg: Option<(usize, usize)>,
    ) -> Value {
        let (used, max) = cfg.unwrap_or((0, 1));

        Value(sys::caml_alloc_final(
            core::mem::size_of::<T>(),
            core::mem::transmute(finalizer),
            used,
            max,
        ))
        .register(rt)
    }

    /// Allocate custom value
    pub unsafe fn alloc_custom<T: crate::Custom>(rt: &mut Runtime) -> Value {
        let size = core::mem::size_of::<T>();
        Value(sys::caml_alloc_custom(
            T::ops() as *const _ as *const sys::custom_operations,
            size,
            T::USED,
            T::MAX,
        ))
        .register(rt)
    }

    /// Allocate an abstract pointer value, it is best to ensure the value is
    /// on the heap using `Box::into_raw(Box::from(...))` to create the pointer
    /// and `Box::from_raw` to free it
    pub unsafe fn alloc_abstract_ptr<T>(rt: &mut Runtime, ptr: *mut T) -> Value {
        let x = Self::alloc(rt, 1, Tag::ABSTRACT);
        let dest = x.0 as *mut *mut T;
        {
            *dest = ptr;
        }
        x
    }

    /// Create a new Value from an existing OCaml `value`
    #[inline]
    pub const unsafe fn new(v: sys::Value) -> Value {
        Value(v)
    }

    /// Get array length
    pub unsafe fn array_length(self) -> usize {
        {
            sys::caml_array_length(self.0)
        }
    }

    /// See caml_register_global_root
    pub unsafe fn register_global_root(&mut self) {
        {
            sys::caml_register_global_root(&mut self.0)
        }
    }

    /// Set caml_remove_global_root
    pub unsafe fn remove_global_root(&mut self) {
        {
            sys::caml_remove_global_root(&mut self.0)
        }
    }

    /// Get the tag for the underlying OCaml `value`
    pub unsafe fn tag(self) -> Tag {
        {
            sys::tag_val(self.0).into()
        }
    }

    /// Convert a boolean to OCaml value
    pub const unsafe fn bool(b: bool) -> Value {
        Value::int(b as crate::Int)
    }

    /// Allocate and copy a string value
    pub unsafe fn string<S: AsRef<str>>(rt: &mut Runtime, s: S) -> Value {
        let x = s.as_ref();
        let s: OCaml<String> = ocaml_alloc!(x.to_ocaml(rt));
        Value(s.raw())
    }

    /// Convert from a pointer to an OCaml string back to an OCaml value
    ///
    /// # Safety
    /// This function assumes that the `str` argument has been allocated by OCaml
    pub unsafe fn of_str(s: &str) -> Value {
        Value(s.as_ptr() as isize)
    }

    /// Convert from a pointer to an OCaml string back to an OCaml value
    ///
    /// # Safety
    /// This function assumes that the `&[u8]` argument has been allocated by OCaml
    pub unsafe fn of_bytes(s: &[u8]) -> Value {
        Value(s.as_ptr() as isize)
    }

    /// OCaml Some value
    pub unsafe fn some(self, rt: &mut Runtime) -> Value {
        let mut x = Self::alloc(rt, 1, 0.into());
        x.store_field(0, self);
        x
    }

    /// OCaml None value
    #[inline(always)]
    pub const fn none() -> Value {
        NONE
    }

    /// OCaml Unit value
    #[inline(always)]
    pub const fn unit() -> Value {
        UNIT
    }

    /// Create a variant value
    pub unsafe fn variant(rt: &mut Runtime, tag: u8, value: Option<Value>) -> Value {
        match value {
            Some(v) => {
                let mut x = Self::alloc(rt, 1, tag.into());
                x.store_field(0, v);
                x
            }
            None => Self::alloc(rt, 0, tag.into()),
        }
    }

    /// Result.Ok value
    pub unsafe fn result_ok(rt: &mut Runtime, value: impl Into<Value>) -> Value {
        Self::variant(rt, 0, Some(value.into()))
    }

    /// Result.Error value
    pub unsafe fn result_error(rt: &mut Runtime, value: impl Into<Value>) -> Value {
        Self::variant(rt, 1, Some(value.into()))
    }

    /// Create an OCaml `int`
    pub const unsafe fn int(i: crate::Int) -> Value {
        Value(sys::val_int(i))
    }

    /// Create an OCaml `int`
    pub const unsafe fn uint(i: crate::Uint) -> Value {
        Value(sys::val_int(i as crate::Int))
    }

    /// Create an OCaml `Int64` from `i64`
    pub unsafe fn int64(rt: &mut Runtime, i: i64) -> Value {
        Value(sys::caml_copy_int64(i)).register(rt)
    }

    /// Create an OCaml `Int32` from `i32`
    pub unsafe fn int32(rt: &mut Runtime, i: i32) -> Value {
        Value(sys::caml_copy_int32(i)).register(rt)
    }

    /// Create an OCaml `Nativeint` from `isize`
    pub unsafe fn nativeint(rt: &mut Runtime, i: isize) -> Value {
        Value(sys::caml_copy_nativeint(i)).register(rt)
    }

    /// Create an OCaml `Float` from `f64`
    pub unsafe fn float(rt: &mut Runtime, d: f64) -> Value {
        Value(sys::caml_copy_double(d)).register(rt)
    }

    /// Check if a Value is an integer or block, returning true if
    /// the underlying value is a block
    pub unsafe fn is_block(self) -> bool {
        sys::is_block(self.0)
    }

    /// Check if a Value is an integer or block, returning true if
    /// the underlying value is an integer
    pub unsafe fn is_long(self) -> bool {
        sys::is_long(self.0)
    }

    /// Get index of underlying OCaml block value
    pub unsafe fn field(self, i: Size) -> Value {
        {
            Value(*sys::field(self.0, i))
        }
    }

    /// Set index of underlying OCaml block value
    pub unsafe fn store_field(&mut self, i: Size, val: Value) {
        {
            sys::store_field(self.0, i, val.0)
        }
    }

    /// Convert an OCaml `int` to `isize`
    pub const unsafe fn int_val(self) -> isize {
        {
            sys::int_val(self.0)
        }
    }

    /// Convert an OCaml `Float` to `f64`
    pub unsafe fn float_val(self) -> f64 {
        {
            *(self.0 as *const f64)
        }
    }

    /// Convert an OCaml `Int32` to `i32`
    pub unsafe fn int32_val(self) -> i32 {
        {
            *self.custom_ptr_val::<i32>()
        }
    }

    /// Convert an OCaml `Int64` to `i64`
    pub unsafe fn int64_val(self) -> i64 {
        {
            *self.custom_ptr_val::<i64>()
        }
    }

    /// Convert an OCaml `Nativeint` to `isize`
    pub unsafe fn nativeint_val(self) -> isize {
        {
            *self.custom_ptr_val::<isize>()
        }
    }

    /// Get pointer to data stored in an OCaml custom value
    pub unsafe fn custom_ptr_val<T>(self) -> *const T {
        {
            sys::field(self.0, 1) as *const T
        }
    }

    /// Get mutable pointer to data stored in an OCaml custom value
    pub unsafe fn custom_ptr_val_mut<T>(self) -> *mut T {
        {
            sys::field(self.0, 1) as *mut T
        }
    }

    /// Get pointer to the pointer contained by Value
    pub unsafe fn abstract_ptr_val<T>(self) -> *const T {
        {
            *(self.0 as *const *const T)
        }
    }

    /// Get mutable pointer to the pointer contained by Value
    pub unsafe fn abstract_ptr_val_mut<T>(self) -> *mut T {
        {
            *(self.0 as *mut *mut T)
        }
    }

    /// Extract OCaml exception
    pub unsafe fn exception(self) -> Option<Value> {
        if !self.is_exception_result() {
            return None;
        }

        Some(Value(crate::sys::extract_exception(self.0)))
    }

    /// Call a closure with a single argument, returning an exception value
    pub unsafe fn call(self, rt: &mut Runtime, arg: Value) -> Result<Value, Error> {
        if self.tag() != Tag::CLOSURE {
            return Err(Error::NotCallable);
        }

        let mut v = Value(sys::caml_callback_exn(self.0, arg.0)).register(rt);
        if v.is_exception_result() {
            v = v.exception().unwrap();
            Err(CamlError::Exception(v).into())
        } else {
            Ok(v)
        }
    }

    /// Call a closure with two arguments, returning an exception value
    pub unsafe fn call2(self, rt: &mut Runtime, arg1: Value, arg2: Value) -> Result<Value, Error> {
        if self.tag() != Tag::CLOSURE {
            return Err(Error::NotCallable);
        }

        let mut v = Value(sys::caml_callback2_exn(self.0, arg1.0, arg2.0)).register(rt);

        if v.is_exception_result() {
            v = v.exception().unwrap();
            Err(CamlError::Exception(v).into())
        } else {
            Ok(v)
        }
    }

    /// Call a closure with three arguments, returning an exception value
    pub unsafe fn call3(
        self,
        rt: &mut Runtime,
        arg1: Value,
        arg2: Value,
        arg3: Value,
    ) -> Result<Value, Error> {
        if self.tag() != Tag::CLOSURE {
            return Err(Error::NotCallable);
        }

        let mut v = Value(sys::caml_callback3_exn(self.0, arg1.0, arg2.0, arg3.0)).register(rt);

        if v.is_exception_result() {
            v = v.exception().unwrap();
            Err(CamlError::Exception(v).into())
        } else {
            Ok(v)
        }
    }

    /// Call a closure with `n` arguments, returning an exception value
    pub unsafe fn call_n<A: AsRef<[Value]>>(
        self,
        rt: &mut Runtime,
        args: A,
    ) -> Result<Value, Error> {
        if self.tag() != Tag::CLOSURE {
            return Err(Error::NotCallable);
        }

        let n = args.as_ref().len();
        let x = args.as_ref();

        let mut v = Value(sys::caml_callbackN_exn(
            self.0,
            n,
            x.as_ptr() as *mut sys::Value,
        ))
        .register(rt);

        if v.is_exception_result() {
            v = v.exception().unwrap();
            Err(CamlError::Exception(v).into())
        } else {
            Ok(v)
        }
    }

    /// Modify an OCaml value in place
    pub unsafe fn modify(&mut self, v: Value) {
        {
            sys::caml_modify(&mut self.0, v.0)
        }
    }

    /// Determines if the current value is an exception
    pub unsafe fn is_exception_result(self) -> bool {
        crate::sys::is_exception_result(self.0)
    }

    /// Get hash variant as OCaml value
    pub unsafe fn hash_variant<S: AsRef<str>>(
        rt: &mut Runtime,
        name: S,
        a: Option<Value>,
    ) -> Value {
        let s = crate::util::CString::new(name.as_ref()).expect("Invalid C string");
        let hash = { Value(sys::caml_hash_variant(s.as_ptr() as *const u8)) };
        match a {
            Some(x) => {
                let mut output = Value::alloc_small(rt, 2, Tag(0));
                output.store_field(0, hash);
                output.store_field(1, x);
                output
            }
            None => hash,
        }
    }

    /// Get object method
    pub unsafe fn method<S: AsRef<str>>(self, rt: &mut Runtime, name: S) -> Option<Value> {
        if self.tag() != Tag::OBJECT {
            return None;
        }

        let v = { sys::caml_get_public_method(self.0, Self::hash_variant(rt, name, None).0) };

        if v == 0 {
            return None;
        }

        Some(Value(v))
    }

    /// Initialize OCaml value using `caml_initialize`
    pub unsafe fn initialize(&mut self, value: Value) {
        {
            sys::caml_initialize(&mut self.0, value.0)
        }
    }

    /// This will recursively clone any OCaml value
    /// The new value is allocated inside the OCaml heap,
    /// and may end up being moved or garbage collected.
    pub unsafe fn deep_clone_to_ocaml(self, rt: &mut Runtime) -> Self {
        if self.is_long() {
            return self;
        }
        {
            let wosize = sys::wosize_val(self.0);
            let val1 = Self::alloc(rt, wosize, self.tag());
            let ptr0 = self.0 as *const sys::Value;
            let ptr1 = val1.0 as *mut sys::Value;
            if self.tag() >= Tag::NO_SCAN {
                ptr0.copy_to_nonoverlapping(ptr1, wosize);
                return val1;
            }
            for i in 0..(wosize as isize) {
                sys::caml_initialize(
                    ptr1.offset(i),
                    Value(ptr0.offset(i).read()).deep_clone_to_ocaml(rt).0,
                );
            }
            val1
        }
    }

    /// This will recursively clone any OCaml value
    /// The new value is allocated outside of the OCaml heap, and should
    /// only be used for storage inside Rust structures.
    #[cfg(not(feature = "no-std"))]
    pub unsafe fn deep_clone_to_rust(self) -> Self {
        if self.is_long() {
            return self;
        }
        {
            if self.tag() >= Tag::NO_SCAN {
                let slice0 = slice(self);
                let vec1 = slice0.to_vec();
                let ptr1 = vec1.as_ptr();
                core::mem::forget(vec1);
                return Value(ptr1.offset(1) as isize);
            }
            let slice0 = slice(self);
            let vec1: Vec<Value> = slice0
                .iter()
                .enumerate()
                .map(|(i, v)| if i == 0 { *v } else { v.deep_clone_to_rust() })
                .collect();
            let ptr1 = vec1.as_ptr();
            core::mem::forget(vec1);
            Value(ptr1.offset(1) as isize)
        }
    }
}

#[cfg(not(feature = "no-std"))]
pub(crate) unsafe fn slice<'a>(value: Value) -> &'a [Value] {
    ::core::slice::from_raw_parts(
        (value.0 as *const Value).offset(-1),
        sys::wosize_val(value.0) + 1,
    )
}

#[cfg(not(feature = "no-std"))]
pub(crate) unsafe fn mut_slice<'a>(value: Value) -> &'a mut [Value] {
    ::core::slice::from_raw_parts_mut(
        (value.0 as *mut Value).offset(-1),
        sys::wosize_val(value.0) + 1,
    )
}
