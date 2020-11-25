/*pub mod callbacks;
pub mod conv;
pub mod custom;
pub mod runtime;
pub mod types;*/

use ocaml::*;

#[no_mangle]
pub unsafe extern "C" fn test_func_1(x: Value, i: Value) -> Value {
    body!(rt, {
        let x: Array<i32> = x.rust::<Array<i32>, Array<i32>>(rt);
        let i: i32 = i.rust::<OCamlInt, i32>(rt);

        x.get(i as usize)
    })
}

/*#[ocaml::func]
pub fn test_func_1(x: Vec<i32>, i: OCamlInt) -> i32 {
    x[i as usize]
}*/
