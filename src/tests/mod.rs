use crate as ocaml;

use crate::*;

/*#[test]
fn test_basic_array() -> Result<(), Error> {
    let mut rt = ocaml::OCamlRuntime::init();
    let mut a: ocaml::Array<String> = ocaml::Array::alloc(&mut rt, 2);
    a.set(&mut rt, 0, "testing".to_string())?;
    a.set(&mut rt, 1, "123".to_string())?;
    let b: ocaml::OCaml<Array<String>> = FromOCaml::from_ocaml(&ocaml_alloc!(a.to_ocaml(&mut rt)));
    assert!(b.into().as_slice() == &["testing", "123"]);
    Ok(())
}*/

/*#[ocaml::func]
pub fn make_tuple(a: OCaml<Value>, b: OCaml<Value>) -> (Value, Value) {
    (a, b)
}

#[test]
fn test_tuple_of_tuples() {
    ocaml::body!({
        let x = (1f64, 2f64, 3f64, 4f64, 5f64, 6f64, 7f64, 8f64, 9f64).to_ocaml(rt);
        let y = (9f64, 8f64, 7f64, 6f64, 5f64, 4f64, 3f64, 2f64, 1f64).to_value(rt);
        let r = make_tuple(x, y);
        let ((a, b, c, d, e, f, g, h, i), (j, k, l, m, n, o, p, q, r)): (
            (f64, f64, f64, f64, f64, f64, f64, f64, f64),
            (f64, f64, f64, f64, f64, f64, f64, f64, f64),
        ) = FromOCaml::from_ocaml(&ocaml_alloc!(r.to_ocaml(rt)));

        println!("a: {}, r: {}", a, r);
        assert!(a == r);
        assert!(b == q);
        assert!(c == p);
        assert!(d == o);
        assert!(e == n);
        assert!(f == m);
        assert!(g == l);
        assert!(h == k);
        assert!(i == j);
    })
}*/

#[test]
fn test_basic_list() {
    ocaml::body!({
        let mut list = ocaml::List::empty();
        list = list.add(rt, 3i64);
        list = list.add(rt, 2i64);
        list = list.add(rt, 1i64);

        assert!(list.len() == 3);

        let ll: OCaml<List<i64>> = FromOCaml::from_ocaml(&ocaml_alloc!(list.to_ocaml(rt)));

        for (i, x) in ll.iter().enumerate() {
            assert!((i + 1) as i64 == *x);
        }
    })
}

/*#[test]
fn test_int() {
    ocaml::body!({
        let a = (-123isize).to_value();
        let b = (-1isize).to_value();
        let c = 123isize.to_value();
        let d = 1isize.to_value();
        let e = 0isize.to_value();

        let a_: isize = FromValue::from_value(a);
        let b_: isize = FromValue::from_value(b);
        let c_: isize = FromValue::from_value(c);
        let d_: isize = FromValue::from_value(d);
        let e_: isize = FromValue::from_value(e);
        assert_eq!(a_, -123);
        assert_eq!(b_, -1);
        assert_eq!(c_, 123);
        assert_eq!(d_, 1);
        assert_eq!(e_, 0);
    })
}*/
