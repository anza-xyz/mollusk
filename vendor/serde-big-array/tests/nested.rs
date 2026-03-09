#![no_std]

use serde_big_array::Array;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct S {
    arr: Array<Array<u8, 234>, 65>,
}

#[test]
fn test() {
    let s = S {
        arr: Array([Array([1; 234]); 65]),
    };
    let j = serde_json::to_string(&s).unwrap();
    let s_back = serde_json::from_str::<S>(&j).unwrap();
    assert!(&s.arr[..] == &s_back.arr[..]);
    assert_eq!(s.arr.len(), 65);
}
