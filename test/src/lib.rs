/*pub mod callbacks;
pub mod conv;
pub mod custom;
pub mod runtime;
pub mod types;*/

use ocaml::*;

#[ocaml::func]
pub fn test_func_1(x: Vec<i32>, i: OCamlInt) -> i32 {
    x[i as usize]
}
