/*#[cfg(not(feature = "no-std"))]
static PANIC_HANDLER_INIT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);*/

#[doc(hidden)]
pub fn init_panic_handler() {
    #[cfg(not(feature = "no-std"))]
    ::std::panic::set_hook(Box::new(|info| unsafe {
        let err = info.payload();
        let msg = if err.is::<&str>() {
            err.downcast_ref::<&str>().unwrap()
        } else if err.is::<String>() {
            err.downcast_ref::<String>().unwrap().as_ref()
        } else {
            "rust panic"
        };

        let mut rt = crate::OCamlRuntime::recover_handle();

        if let Some(err) = crate::Value::named("Rust_exception") {
            crate::Error::raise_value(&mut rt, err, msg);
        }

        crate::Error::raise_failure(&mut rt, msg)
    }));
}

/// `body!` is needed to help the OCaml runtime to manage garbage collection, it should
/// be used to wrap the body of each function exported to OCaml. Panics from Rust code
/// will automatically be unwound/caught here (unless the `no-std` feature is enabled)
///
/// ```rust
/// #[no_mangle]
/// pub extern "C" fn example(a: ocaml::Value, b: ocaml::Value) -> ocaml::Value {
///     ocaml::body!((a, b) {
///         let a = a.int_val();
///         let b = b.int_val();
///         ocaml::Value::int(a + b)
///     })
/// }
/// ```
#[macro_export]
macro_rules! body {
    ($rt:ident, $code:block) => {{
        // Ensure panic handler is initialized
        $crate::init_panic_handler();

        // Initialize OCaml frame
        #[allow(unused_unsafe)]
        let mut $rt = $crate::OCamlRuntime::init();

        let res = {
            // Execute Rust function
            #[allow(unused_mut)]
            let mut res = |$rt: &mut $crate::OCamlRuntime| $code;
            let res = res(&mut $rt);

            let res = res.to_ocaml($rt.token()).mark(&mut $rt).eval(&mut $rt);
            let res: Result<_, OCaml<$crate::Error>> = res.to_result();
            res
        };

        match res {
            Ok(value) => {
                let value: Value = Value::of_ocaml(&value);
                if value.is_exception_result() {
                    let e = {
                        Error::Caml(CamlError::Exception(value.exception().unwrap()).into())
                            .to_ocaml($rt.token())
                            .mark(&mut $rt)
                            .eval(&mut $rt)
                    };
                    let e: Error = e.to_rust();
                    let e = e.to_ocaml($rt.token()).mark(&mut $rt).eval(&mut $rt);
                    return Value::of_ocaml(&e);
                }
                value
            }
            Err(e) => {
                let e: Error = e.to_rust();
                let e = e.to_ocaml($rt.token()).mark(&mut $rt).eval(&mut $rt);
                Value::of_ocaml(&e)
            }
        }

        //Error::result(&mut $rt, res)
    }};
}
