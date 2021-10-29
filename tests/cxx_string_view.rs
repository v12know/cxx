#![cfg(any(feature="c++17", feature="c++20"))]

use std::assert_eq;
use cxx::let_cxx_string;

#[test]
fn test_cxx_string_view() {
    let_cxx_string!(s = "A string from C++");
    let sv = s.to_string_view();

    assert_eq!(&sv, "A string from C++");
}
