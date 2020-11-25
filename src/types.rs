//! OCaml types represented in Rust, these are zero-copy and incur no additional overhead

use crate::*;

use core::{iter::Iterator, marker::PhantomData, mem, slice};

use crate::value::{Size, Value};

/// A handle to a Rust value/reference owned by the OCaml heap.
///
/// This should only be used with values allocated with `alloc_final` or `alloc_custom`,
/// for abstract pointers see `Value::alloc_abstract_ptr` and `Value::abstract_ptr_val`
#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Pointer<T>(pub Value, PhantomData<T>);

unsafe impl<T> ToOCaml<Pointer<T>> for Pointer<T> {
    fn to_ocaml(&self, _token: AllocToken) -> AllocResult<Pointer<T>> {
        AllocResult::of((self.0).0)
    }
}

unsafe impl<T> FromOCaml<Pointer<T>> for Pointer<T> {
    fn from_ocaml(value: &OCaml<Pointer<T>>) -> Self {
        unsafe { Pointer(Value(value.raw()), PhantomData) }
    }
}

unsafe extern "C" fn ignore(_: Value) {}

impl<T> Pointer<T> {
    /// Allocate a new value with an optional custom finalizer and used/max
    ///
    /// This calls `caml_alloc_final` under-the-hood, which can has less than ideal performance
    /// behavior. In most cases you should prefer `Poiner::alloc_custom` when possible.
    pub unsafe fn alloc_final(
        rt: &mut Runtime,
        x: T,
        finalizer: Option<unsafe extern "C" fn(Value)>,
        used_max: Option<(usize, usize)>,
    ) -> Pointer<T> {
        let mut ptr = Pointer(
            match finalizer {
                Some(f) => Value::alloc_final::<T>(rt, f, used_max),
                None => Value::alloc_final::<T>(rt, ignore, used_max),
            },
            PhantomData,
        );
        ptr.set(x);
        ptr
    }

    /// Allocate a `Custom` value
    pub unsafe fn alloc_custom(rt: &mut Runtime, x: T) -> Pointer<T>
    where
        T: crate::Custom,
    {
        let mut ptr = Pointer(Value::alloc_custom::<T>(rt), PhantomData);
        ptr.set(x);
        ptr
    }

    /// Drop pointer in place
    ///
    /// # Safety
    /// This should only be used when you're in control of the underlying value and want to drop
    /// it. It should only be called once.
    pub unsafe fn drop_in_place(mut self) {
        core::ptr::drop_in_place(self.as_mut_ptr())
    }

    /// Replace the inner value with the provided argument
    pub unsafe fn set(&mut self, x: T) {
        core::ptr::write_unaligned(self.as_mut_ptr(), x);
    }

    /// Access the underlying pointer
    pub unsafe fn as_ptr(&self) -> *const T {
        self.0.custom_ptr_val()
    }

    /// Access the underlying mutable pointer
    pub unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self.0.custom_ptr_val_mut()
    }
}

impl<T> AsRef<T> for Pointer<T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.as_ptr() }
    }
}

impl<T> AsMut<T> for Pointer<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.as_mut_ptr() }
    }
}

/// `Array<A>` wraps an OCaml `'a array` without converting it to Rust
#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Array<T>(Value, PhantomData<T>);

unsafe impl<T: ToOCaml<T> + FromOCaml<T>> ToOCaml<Array<T>> for Array<T> {
    fn to_ocaml(&self, _token: AllocToken) -> AllocResult<Array<T>> {
        AllocResult::of((self.0).0)
    }
}

unsafe impl<T: ToOCaml<T> + FromOCaml<T>> FromOCaml<Array<T>> for Array<T> {
    fn from_ocaml(value: &OCaml<Array<T>>) -> Self {
        unsafe { Array(Value(value.raw()), PhantomData) }
    }
}

impl<'a> Array<OCamlFloat> {
    /// Set value to double array
    pub unsafe fn set_double(&mut self, i: usize, f: f64) -> Result<(), Error> {
        if i >= self.len() {
            return Err(CamlError::ArrayBoundError.into());
        }

        if !self.is_double_array() {
            return Err(Error::NotDoubleArray);
        }

        self.set_double_unchecked(i, f);

        Ok(())
    }

    /// Set value to double array without bounds checking
    ///
    /// # Safety
    /// This function performs no bounds checking
    #[inline]
    pub unsafe fn set_double_unchecked(&mut self, i: usize, f: f64) {
        let ptr = ((self.0).0 as *mut f64).add(i);
        *ptr = f;
    }

    /// Get a value from a double array
    pub unsafe fn get_double(self, i: usize) -> Result<f64, Error> {
        if i >= self.len() {
            return Err(CamlError::ArrayBoundError.into());
        }
        if !self.is_double_array() {
            return Err(Error::NotDoubleArray);
        }

        Ok(self.get_double_unchecked(i))
    }

    /// Get a value from a double array without checking if the array is actually a double array
    ///
    /// # Safety
    ///
    /// This function does not perform bounds checking
    #[inline]
    pub unsafe fn get_double_unchecked(self, i: usize) -> f64 {
        *((self.0).0 as *mut f64).add(i)
    }
}

impl<T> Array<T> {
    /// Allocate a new Array
    pub unsafe fn alloc(rt: &mut Runtime, n: usize) -> Array<T> {
        let x = Value(sys::caml_alloc(n, 0)).register(rt);
        Array(x, PhantomData)
    }

    /// Check if Array contains only doubles, if so `get_double` and `set_double` should be used
    /// to access values
    pub fn is_double_array(&self) -> bool {
        unsafe { sys::caml_is_double_array((self.0).0) == 1 }
    }

    /// Array length
    pub fn len(&self) -> usize {
        unsafe { sys::caml_array_length((self.0).0) }
    }

    /// Returns true when the array is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Set array index
    pub fn set(&mut self, rt: &mut Runtime, i: usize, v: T) -> Result<(), Error>
    where
        T: ToOCaml<T>,
    {
        if i >= self.len() {
            return Err(CamlError::ArrayBoundError.into());
        }
        unsafe { self.set_unchecked(rt, i, v) }
        Ok(())
    }

    /// Set array index without bounds checking
    ///
    /// # Safety
    ///
    /// This function does not perform bounds checking
    #[inline]
    pub unsafe fn set_unchecked(&mut self, rt: &mut Runtime, i: usize, v: T)
    where
        T: ToOCaml<T>,
    {
        self.0
            .store_field(i, Value(ocaml_alloc!(v.to_ocaml(rt)).raw()));
    }

    /// Get array index
    pub fn get(&self, rt: &mut Runtime, i: usize) -> Result<T, Error>
    where
        T: FromOCaml<T>,
    {
        if i >= self.len() {
            return Err(CamlError::ArrayBoundError.into());
        }
        Ok(unsafe { self.get_unchecked(rt, i) })
    }

    /// Get array index without bounds checking
    ///
    /// # Safety
    ///
    /// This function does not perform bounds checking
    #[inline]
    pub unsafe fn get_unchecked(&self, rt: &mut Runtime, i: usize) -> T
    where
        T: FromOCaml<T>,
    {
        T::from_ocaml(&OCaml::new(rt, self.0.field(i).0))
    }

    /// Array as slice
    pub fn as_slice(&self) -> &[Value] {
        unsafe { crate::value::slice(self.0) }
    }

    /// Array as mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [Value] {
        unsafe { crate::value::mut_slice(self.0) }
    }

    /// Array as `Vec`
    #[cfg(not(feature = "no-std"))]
    pub fn to_vec(&self, rt: &mut Runtime) -> Vec<T>
    where
        T: FromOCaml<T>,
    {
        self.as_slice()
            .iter()
            .map(|x| unsafe { T::from_ocaml(&OCaml::new(rt, x.0)) })
            .collect()
    }
}

/// `List<A>` wraps an OCaml `'a list` without converting it to Rust, this introduces no
/// additional overhead compared to a `Value` type
#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct List<T: ToOCaml<T> + FromOCaml<T>>(Value, PhantomData<T>);

unsafe impl<T: ToOCaml<T> + FromOCaml<T>> ToOCaml<List<T>> for List<T> {
    fn to_ocaml(&self, _token: AllocToken) -> AllocResult<List<T>> {
        AllocResult::of((self.0).0)
    }
}

unsafe impl<T: ToOCaml<T> + FromOCaml<T>> FromOCaml<List<T>> for List<T> {
    fn from_ocaml(value: &OCaml<List<T>>) -> Self {
        unsafe { List(Value(value.raw()), PhantomData) }
    }
}

impl<T: ToOCaml<T> + FromOCaml<T>> List<T> {
    /// An empty list
    #[inline(always)]
    pub unsafe fn empty() -> List<T> {
        List(Value::unit(), PhantomData)
    }

    /// Returns the number of items in `self`
    pub unsafe fn len(&self) -> usize {
        let mut length = 0;
        let mut tmp = self.0;
        while tmp.0 != sys::EMPTY_LIST {
            tmp = tmp.field(1);
            length += 1;
        }
        length
    }

    /// Returns true when the list is empty
    pub unsafe fn is_empty(&self) -> bool {
        self.0 == Self::empty().0
    }

    /// Add an element to the front of the list returning the new list
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, rt: &mut Runtime, v: T) -> List<T> {
        let tmp = unsafe {
            let mut tmp = Value(sys::caml_alloc(2, 0));
            tmp.store_field(0, Value(ocaml_alloc!(v.to_ocaml(rt)).raw()));
            tmp.store_field(1, self.0);
            tmp
        };
        List(tmp, PhantomData)
    }

    /// List head
    pub unsafe fn hd(&self, rt: &mut Runtime) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        Some(T::from_ocaml(&OCaml::new(rt, self.0.field(0).0)))
    }

    /// List tail
    pub unsafe fn tl(&self, rt: &Runtime) -> List<T> {
        if self.is_empty() {
            return Self::empty();
        }

        List::from_ocaml(&OCaml::new(rt, self.0.field(1).0))
    }

    #[cfg(not(feature = "no-std"))]
    /// List as `Vec`
    pub unsafe fn to_vec(&self, rt: &mut Runtime) -> Vec<T> {
        self.iter(rt).collect()
    }

    /*#[cfg(not(feature = "no-std"))]
    /// List as `LinkedList`
    pub fn to_linked_list(&self) -> std::collections::LinkedList<T> {
        FromValue::from_value(self.0)
    }*/

    /// List iterator
    pub unsafe fn iter<'a>(&self, rt: &'a mut Runtime) -> ListIterator<'a, T> {
        ListIterator {
            rt,
            inner: self.0,
            _marker: PhantomData,
        }
    }
}

/// List iterator.
pub struct ListIterator<'a, T: ToOCaml<T> + FromOCaml<T>> {
    inner: Value,
    rt: &'a mut Runtime,
    _marker: PhantomData<T>,
}

impl<'a, T: ToOCaml<T> + FromOCaml<T>> Iterator for ListIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner != Value::unit() {
            self.inner = unsafe { self.inner.field(1) };
            unsafe { List(self.inner, PhantomData).hd(self.rt) }
        } else {
            None
        }
    }
}

/// `bigarray` contains wrappers for OCaml `Bigarray` values. These types can be used to transfer arrays of numbers between Rust
/// and OCaml directly without the allocation overhead of an `array` or `list`
pub mod bigarray {
    use super::*;
    use crate::sys::bigarray;

    /// Bigarray kind
    pub trait Kind {
        /// Array item type
        type T: Clone + Copy;

        /// OCaml bigarray type identifier
        fn kind() -> i32;
    }

    macro_rules! make_kind {
        ($t:ty, $k:ident) => {
            impl Kind for $t {
                type T = $t;

                fn kind() -> i32 {
                    bigarray::Kind::$k as i32
                }
            }
        };
    }

    make_kind!(u8, UINT8);
    make_kind!(i8, SINT8);
    make_kind!(u16, UINT16);
    make_kind!(i16, SINT16);
    make_kind!(f32, FLOAT32);
    make_kind!(f64, FLOAT64);
    make_kind!(i64, INT64);
    make_kind!(i32, INT32);
    make_kind!(char, CHAR);

    /// OCaml Bigarray.Array1 type, this introduces no
    /// additional overhead compared to a `Value` type
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq)]
    pub struct Array1<'a, T>(Value, PhantomData<&'a T>);

    unsafe impl<'a, T> FromOCaml<Array1<'a, T>> for Array1<'a, T> {
        fn from_ocaml(value: &OCaml<'_, Array1<'a, T>>) -> Array1<'a, T> {
            unsafe { Array1(Value(value.raw()), PhantomData) }
        }
    }

    unsafe impl<'a, T> ToOCaml<Array1<'a, T>> for Array1<'a, T> {
        fn to_ocaml(&self, _token: AllocToken) -> AllocResult<Array1<'a, T>> {
            AllocResult::of((self.0).0)
        }
    }

    impl<'a, T: Copy + Kind> Array1<'a, T> {
        /// Create new 1-dimensional array from an existing Vec
        pub unsafe fn from_vec(rt: &mut Runtime, x: Vec<T>) -> Array1<'a, T> {
            let mut arr = Array1::<T>::create(rt, x.len());
            let data = arr.data_mut();
            data.copy_from_slice(x.as_slice());
            arr
        }

        /// Array1::of_slice is used to convert from a slice to OCaml Bigarray,
        /// the `data` parameter must outlive the resulting bigarray or there is
        /// no guarantee the data will be valid. Use `Array1::from_slice` to clone the
        /// contents of a slice.
        pub unsafe fn of_slice(rt: &mut Runtime, data: &'a mut [T]) -> Array1<'a, T> {
            let x = {
                Value(bigarray::caml_ba_alloc_dims(
                    T::kind() | bigarray::Managed::EXTERNAL as i32,
                    1,
                    data.as_mut_ptr() as bigarray::Data,
                    data.len() as sys::Intnat,
                ))
                .register(rt)
            };
            Array1(x, PhantomData)
        }

        /// Convert from a slice to OCaml Bigarray, copying the array. This is the implemtation
        /// used by `Array1::from` for slices to avoid any potential lifetime issues
        #[cfg(not(feature = "no-std"))]
        pub unsafe fn from_slice(rt: &'a mut Runtime, data: &'a [T]) -> Array1<'a, T> {
            Array1::from_vec(rt, data.to_vec())
        }

        /// Create a new OCaml `Bigarray.Array1` with the given type and size
        pub unsafe fn create(rt: &mut Runtime, n: Size) -> Array1<'a, T> {
            let x = {
                let data = bigarray::malloc(n * mem::size_of::<T>());
                Value(bigarray::caml_ba_alloc_dims(
                    T::kind() | bigarray::Managed::MANAGED as i32,
                    1,
                    data,
                    n as sys::Intnat,
                ))
                .register(rt)
            };
            Array1(x, PhantomData)
        }

        /// Returns the number of items in `self`
        pub unsafe fn len(self) -> Size {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            let dim = { slice::from_raw_parts((*ba).dim.as_ptr() as *const usize, 1) };
            dim[0]
        }

        /// Returns true when `self.len() == 0`
        pub unsafe fn is_empty(self) -> bool {
            self.len() == 0
        }

        /// Get underlying data as Rust slice
        pub unsafe fn data(&self) -> &[T] {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                slice::from_raw_parts((*ba).data as *const T, self.len())
            }
        }

        /// Get underlying data as mutable Rust slice
        pub unsafe fn data_mut(&mut self) -> &mut [T] {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                slice::from_raw_parts_mut((*ba).data as *mut T, self.len())
            }
        }
    }

    #[cfg(all(feature = "bigarray-ext", not(feature = "no-std")))]
    pub use super::bigarray_ext::*;
}

#[cfg(all(feature = "bigarray-ext", not(feature = "no-std")))]
pub(crate) mod bigarray_ext {
    use ndarray::{ArrayView2, ArrayView3, ArrayViewMut2, ArrayViewMut3, Dimension};

    use core::{marker::PhantomData, mem, ptr, slice};

    use crate::{
        bigarray::Kind,
        sys::{self, bigarray},
        AllocResult, AllocToken, FromOCaml, OCaml, Runtime, ToOCaml, Value,
    };

    /// OCaml Bigarray.Array2 type, this introduces no
    /// additional overhead compared to a `Value` type
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq)]
    pub struct Array2<T>(Value, PhantomData<T>);

    impl<T: Copy + Kind> Array2<T> {
        /// Returns array view
        pub unsafe fn view(&self) -> ArrayView2<T> {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                ArrayView2::from_shape_ptr(self.shape(), (*ba).data as *const T)
            }
        }

        /// Returns mutable array view
        pub unsafe fn view_mut(&mut self) -> ArrayViewMut2<T> {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                ArrayViewMut2::from_shape_ptr(self.shape(), (*ba).data as *mut T)
            }
        }

        /// Returns the shape of `self`
        pub unsafe fn shape(&self) -> (usize, usize) {
            let dim = self.dim();
            (dim[0], dim[1])
        }

        /// Returns the number of items in `self`
        pub unsafe fn len(&self) -> usize {
            let dim = self.dim();
            dim[0] * dim[1]
        }

        /// Returns true when the list is empty
        pub unsafe fn is_empty(&self) -> bool {
            self.len() == 0
        }

        unsafe fn dim(&self) -> &[usize] {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                slice::from_raw_parts((*ba).dim.as_ptr() as *const usize, 2)
            }
        }
    }

    unsafe impl<T> FromOCaml<Array2<T>> for Array2<T> {
        fn from_ocaml(value: &OCaml<Array2<T>>) -> Array2<T> {
            unsafe { Array2(Value(value.raw()), PhantomData) }
        }
    }

    unsafe impl<T> ToOCaml<Array2<T>> for Array2<T> {
        fn to_ocaml(&self, _token: AllocToken) -> AllocResult<Array2<T>> {
            AllocResult::of((self.0).0)
        }
    }

    impl<T: Copy + Kind> Array2<T> {
        /// Create a new OCaml `Bigarray.Array2` with the given type and shape
        pub unsafe fn create(rt: &mut Runtime, dim: ndarray::Ix2) -> Array2<T> {
            let x = {
                let data = bigarray::malloc(dim.size() * mem::size_of::<T>());
                Value(bigarray::caml_ba_alloc_dims(
                    T::kind() | bigarray::Managed::MANAGED as i32,
                    2,
                    data,
                    dim[0] as sys::Intnat,
                    dim[1] as sys::Intnat,
                ))
                .register(rt)
            };
            Array2(x, PhantomData)
        }

        /// Create `Bigarray.Array2` from an existing `ndarray::Array2`
        pub unsafe fn from_array(rt: &mut Runtime, data: ndarray::Array2<T>) -> Array2<T> {
            let dim = data.raw_dim();
            let array = Array2::create(rt, dim);
            let ba = array.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                ptr::copy_nonoverlapping(data.as_ptr(), (*ba).data as *mut T, dim.size());
            }
            array
        }
    }

    /// OCaml Bigarray.Array3 type, this introduces no
    /// additional overhead compared to a `Value` type
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq)]
    pub struct Array3<T>(Value, PhantomData<T>);

    impl<T: Copy + Kind> Array3<T> {
        /// Returns array view
        pub unsafe fn view(&self) -> ArrayView3<T> {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                ArrayView3::from_shape_ptr(self.shape(), (*ba).data as *const T)
            }
        }

        /// Returns mutable array view
        pub unsafe fn view_mut(&mut self) -> ArrayViewMut3<T> {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                ArrayViewMut3::from_shape_ptr(self.shape(), (*ba).data as *mut T)
            }
        }

        /// Returns the shape of `self`
        pub unsafe fn shape(&self) -> (usize, usize, usize) {
            let dim = self.dim();
            (dim[0], dim[1], dim[2])
        }

        /// Returns the number of items in `self`
        pub unsafe fn len(&self) -> usize {
            let dim = self.dim();
            dim[0] * dim[1] * dim[2]
        }

        /// Returns true when the list is empty
        pub unsafe fn is_empty(&self) -> bool {
            self.len() == 0
        }

        unsafe fn dim(&self) -> &[usize] {
            let ba = self.0.custom_ptr_val::<bigarray::Bigarray>();
            unsafe { slice::from_raw_parts((*ba).dim.as_ptr() as *const usize, 3) }
        }
    }

    unsafe impl<T> FromOCaml<Array3<T>> for Array3<T> {
        fn from_ocaml(value: &OCaml<Array3<T>>) -> Array3<T> {
            unsafe { Array3(Value(value.raw()), PhantomData) }
        }
    }

    unsafe impl<T> ToOCaml<Array3<T>> for Array3<T> {
        fn to_ocaml(&self, _token: AllocToken) -> AllocResult<Array3<T>> {
            AllocResult::of((self.0).0)
        }
    }

    impl<T: Copy + Kind> Array3<T> {
        /// Create a new OCaml `Bigarray.Array3` with the given type and shape
        pub fn create(rt: &mut Runtime, dim: ndarray::Ix3) -> Array3<T> {
            let x = unsafe {
                let data = bigarray::malloc(dim.size() * mem::size_of::<T>());
                Value(bigarray::caml_ba_alloc_dims(
                    T::kind() | bigarray::Managed::MANAGED as i32,
                    3,
                    data,
                    dim[0] as sys::Intnat,
                    dim[1] as sys::Intnat,
                    dim[2] as sys::Intnat,
                ))
                .register(rt)
            };
            Array3(x, PhantomData)
        }

        /// Create `Bigarray.Array3` from an existing `ndarray::Array3`
        pub unsafe fn from_array(rt: &mut Runtime, data: ndarray::Array3<T>) -> Array3<T> {
            let dim = data.raw_dim();
            let array = Array3::create(rt, dim);
            let ba = array.0.custom_ptr_val::<bigarray::Bigarray>();
            {
                ptr::copy_nonoverlapping(data.as_ptr(), (*ba).data as *mut T, dim.size());
            }
            array
        }
    }
}
