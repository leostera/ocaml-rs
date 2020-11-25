use crate::*;

/// Errors that are translated directly into OCaml exceptions
#[derive(Debug)]
pub enum CamlError {
    /// Not_found
    NotFound,

    /// Failure
    Failure(&'static str),

    /// Invalid_argument
    InvalidArgument(&'static str),

    /// Out_of_memory
    OutOfMemory,

    /// Stack_overflow
    StackOverflow,

    /// Sys_error
    SysError(&'static str),

    /// End_of_file
    EndOfFile,

    /// Zero_divide
    ZeroDivide,

    /// Array bound error
    ArrayBoundError,

    /// Sys_blocked_io
    SysBlockedIo,

    /// A pre-allocated OCaml exception
    Exception(Value),

    /// An exception type and argument
    WithArg(Value, Value),
}

/// Error returned by `ocaml-rs` functions
#[derive(Debug)]
pub enum Error {
    /// A value cannot be called using callback functions
    NotCallable,

    /// Array is not a double array
    NotDoubleArray,

    /// Error message
    Message(&'static str),

    /// General error
    #[cfg(not(feature = "no-std"))]
    Error(Box<dyn std::error::Error>),

    /// OCaml exceptions
    Caml(CamlError),
}

#[cfg(not(feature = "no-std"))]
impl<T: 'static + std::error::Error> From<T> for Error {
    fn from(x: T) -> Error {
        Error::Error(Box::new(x))
    }
}

impl From<CamlError> for Error {
    fn from(x: CamlError) -> Error {
        Error::Caml(x)
    }
}

impl Error {
    /// Re-raise an existing exception value
    pub unsafe fn reraise(exc: Value) -> Result<(), Error> {
        Err(CamlError::Exception(exc).into())
    }

    /// Raise an exception that has been registered using `Callback.register_exception` with no
    /// arguments
    pub unsafe fn raise<S: AsRef<str>>(exc: S) -> Result<(), Error> {
        let value = match Value::named(exc.as_ref()) {
            Some(v) => v,
            None => {
                return Err(Error::Message(
                    "Value has not been registered with the OCaml runtime",
                ))
            }
        };
        Err(CamlError::Exception(value).into())
    }

    /// Raise an exception that has been registered using `Callback.register_exception` with an
    /// argument
    pub unsafe fn raise_with_arg<S: AsRef<str>, T: ToOCaml<T>>(
        rt: &mut OCamlRuntime,
        exc: S,
        arg: T,
    ) -> Result<(), Error> {
        let value = match Value::named(exc.as_ref()) {
            Some(v) => v,
            None => {
                return Err(Error::Message(
                    "Value has not been registered with the OCaml runtime",
                ))
            }
        };

        let arg = ocaml_alloc!(arg.to_ocaml(rt));
        Err(CamlError::WithArg(value, Value(arg.raw())).into())
    }

    /// Raise `Not_found`
    pub fn not_found() -> Result<(), Error> {
        Err(CamlError::NotFound.into())
    }

    /// Raise `Out_of_memory`
    pub fn out_of_memory() -> Result<(), Error> {
        Err(CamlError::OutOfMemory.into())
    }

    /// Raise `Failure`
    pub fn failwith(s: &'static str) -> Result<(), Error> {
        Err(CamlError::Failure(s).into())
    }

    /// Raise `Invalid_argument`
    pub fn invalid_argument(s: &'static str) -> Result<(), Error> {
        Err(CamlError::Failure(s).into())
    }

    #[doc(hidden)]
    pub unsafe fn raise_failure(rt: &mut OCamlRuntime, s: &str) -> ! {
        let s: OCaml<String> = ocaml_alloc!(s.to_ocaml(rt));
        {
            crate::sys::caml_failwith_value(s.raw());
        }
        #[allow(clippy::empty_loop)]
        loop {}
    }

    #[doc(hidden)]
    pub unsafe fn raise_value(rt: &mut OCamlRuntime, v: Value, s: &str) -> ! {
        let s: OCaml<String> = ocaml_alloc!(s.to_ocaml(rt));
        {
            crate::sys::caml_raise_with_arg(v.0, s.raw());
        }
        #[allow(clippy::empty_loop)]
        loop {}
    }

    /// Get named error registered using `Callback.register_exception`
    pub unsafe fn named<S: AsRef<str>>(s: S) -> Option<Value> {
        Value::named(s.as_ref())
    }

    /// Wrap std::error::Error value
    pub fn wrap<E: 'static + std::error::Error>(
        rt: &mut OCamlRuntime,
        error: E,
    ) -> OCamlAllocResult<Error> {
        let e = Error::Error(Box::new(error));
        OCamlAllocResult::of_ocaml(ocaml_alloc!(e.to_ocaml(rt)))
    }
}

unsafe impl ToOCaml<Error> for Error {
    fn to_ocaml(&self, token: OCamlAllocToken) -> OCamlAllocResult<Error> {
        match self {
            Error::Caml(CamlError::Exception(e)) => unsafe {
                crate::sys::caml_raise(e.0);
            },
            Error::Caml(CamlError::NotFound) => unsafe {
                crate::sys::caml_raise_not_found();
            },
            Error::Caml(CamlError::ArrayBoundError) => unsafe {
                crate::sys::caml_array_bound_error();
            },
            Error::Caml(CamlError::OutOfMemory) => unsafe {
                crate::sys::caml_array_bound_error();
            },
            Error::Caml(CamlError::EndOfFile) => unsafe { crate::sys::caml_raise_end_of_file() },
            Error::Caml(CamlError::StackOverflow) => unsafe {
                crate::sys::caml_raise_stack_overflow()
            },
            Error::Caml(CamlError::ZeroDivide) => unsafe { crate::sys::caml_raise_zero_divide() },
            Error::Caml(CamlError::SysBlockedIo) => unsafe {
                crate::sys::caml_raise_sys_blocked_io()
            },
            Error::Caml(CamlError::InvalidArgument(s)) => {
                unsafe {
                    let s = crate::util::CString::new(*s).expect("Invalid C string");
                    crate::sys::caml_invalid_argument(s.as_ptr() as *const ocaml_sys::Char)
                };
            }
            Error::Caml(CamlError::WithArg(a, b)) => unsafe {
                crate::sys::caml_raise_with_arg(a.0, b.0)
            },
            Error::Caml(CamlError::SysError(s)) => {
                unsafe {
                    let rt = &mut token.recover_runtime_handle();
                    let s: OCaml<String> = ocaml_alloc!(s.to_ocaml(rt));
                    crate::sys::caml_raise_sys_error(s.raw())
                };
            }
            Error::Message(s) => {
                unsafe {
                    let s = crate::util::CString::new(*s).expect("Invalid C string");
                    crate::sys::caml_failwith(s.as_ptr() as *const ocaml_sys::Char)
                };
            }
            Error::Caml(CamlError::Failure(s)) => {
                unsafe {
                    let s = crate::util::CString::new(*s).expect("Invalid C string");
                    crate::sys::caml_failwith(s.as_ptr() as *const ocaml_sys::Char)
                };
            }
            #[cfg(not(feature = "no-std"))]
            Error::Error(e) => {
                let s = format!("{:?}\0", e);
                unsafe { crate::sys::caml_failwith(s.as_ptr() as *const ocaml_sys::Char) };
            }
            Error::NotDoubleArray => {
                let s = "invalid double array\0";
                unsafe { crate::sys::caml_failwith(s.as_ptr() as *const ocaml_sys::Char) };
            }
            Error::NotCallable => {
                let s = "value is not callable\0";
                unsafe { crate::sys::caml_failwith(s.as_ptr() as *const ocaml_sys::Char) };
            }
        };

        OCamlAllocResult::of(sys::UNIT)
    }
}

/*impl<T: FromOCaml<T>> FromOCaml<Result<T, crate::Error>> for Result<T, crate::Error> {
    fn from_ocaml(value: OCaml<Result<T, crate::Error>>) -> Self {
        if value.is_exception_result() {
            return Err(CamlError::Exception(value.exception().unwrap()).into());
        }

        Ok(T::from_ocaml(value))
    }
}*/
