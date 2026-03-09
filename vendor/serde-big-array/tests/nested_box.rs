#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use serde_big_array::Array;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct S {
    arr: Box<Array<u8, 1234>>,
}

#[test]
fn test() {
    let s = S {
        arr: Box::new(Array([1; 1234])),
    };
    let j = serde_json::to_string(&s).unwrap();
    let s_back = serde_json::from_str::<S>(&j).unwrap();
    assert!(&s.arr[..] == &s_back.arr[..]);
}
