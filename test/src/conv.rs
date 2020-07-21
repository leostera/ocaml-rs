use ocaml::{FromValue, ToValue};

#[derive(ToValue, FromValue)]
enum Enum1 {
    Empty,
    First(ocaml::Int),
    Second(ocaml::Array<&'static str>),
}

#[ocaml::func]
pub fn enum1_empty() -> Enum1 {
    Enum1::Empty
}

#[ocaml::func]
pub fn enum1_first(i: ocaml::Int) -> Enum1 {
    Enum1::First(i)
}

#[ocaml::func]
pub fn enum1_make_second(s: &'static str) -> Enum1 {
    let mut arr = ocaml::Array::alloc(&root, 1);
    let _ = arr.set(&root, 0, s);
    Enum1::Second(arr)
}

#[ocaml::func]
pub fn enum1_get_second_value(e: Enum1) -> Option<ocaml::Array<&'static str>> {
    match e {
        Enum1::Second(x) => Some(x),
        Enum1::Empty | Enum1::First(_) => None,
    }
}

#[ocaml::func]
pub fn enum1_is_empty(e: Enum1) -> bool {
    match e {
        Enum1::Empty => true,
        _ => false,
    }
}

#[derive(ToValue, FromValue, Default)]
struct Struct1 {
    a: ocaml::Int,
    b: ocaml::Float,
    c: Option<String>,
    d: Option<ocaml::Array<&'static str>>,
}

#[ocaml::func]
pub fn struct1_empty() -> Struct1 {
    Struct1::default()
}

#[ocaml::func]
pub fn struct1_get_c(s: Struct1) -> Option<String> {
    s.c
}

#[ocaml::func]
pub fn struct1_get_d(s: Struct1) -> Option<ocaml::Array<&'static str>> {
    s.d
}

#[ocaml::func]
pub fn struct1_set_c(mut s: Struct1, v: String) {
    s.c = Some(v);
}

#[ocaml::func]
pub unsafe fn make_struct1(
    a: ocaml::Int,
    b: ocaml::Float,
    c: Option<String>,
    d: Option<ocaml::Array<&'static str>>,
) -> Result<Struct1, ocaml::Error> {
    Ok(Struct1 { a, b, c, d })
}

#[ocaml::func]
pub unsafe fn string_non_copying(s: &str) -> ocaml::Value {
    ocaml::Value::of_str(s)
}

#[ocaml::func]
pub unsafe fn direct_slice(data: &[ocaml::Value]) -> i64 {
    let mut total = 0;
    for i in data {
        let x: i64 = ocaml::FromValue::from_value(*i);
        total += x;
    }
    total
}

#[ocaml::func]
pub unsafe fn deep_clone(a: ocaml::Value) -> ocaml::Value {
    let b = a.deep_clone_to_rust();
    b.deep_clone_to_ocaml(&root)
}
