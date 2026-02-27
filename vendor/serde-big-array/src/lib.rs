/*!
Big array helper for serde.
The purpose of this crate is to make (de-)serializing arrays of sizes > 32 easy.
This solution is needed until [serde adopts const generics support](https://github.com/serde-rs/serde/issues/1937).

This crates provides you with two tools to use big arrays in your crate:

* The first tool is the [`BigArray`] trait. You can use it together with the
  `serde_derive` macro and an `#[serde(with = "BigArray")]` next to your data declaration.
* The second tool is the [`Array`] struct. It requires you to change your datastructures,
  and some of the code accessing your array, but it allows for nested use cases,
  which [`BigArray`] struggles with.

We recommended using the [`BigArray`] trait in most cases, and using the
[`Array`] struct only if [`BigArray`] doesn't work.

[`BigArray`]: self::BigArray
[`Array`]: self::Array

## Example
```
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_big_array;

use serde_big_array::BigArray;

#[derive(Serialize, Deserialize)]
struct S {
    #[serde(with = "BigArray")]
    arr: [u8; 64],
}

#[test]
fn test() {
    let s = S { arr: [1; 64] };
    let j = serde_json::to_string(&s).unwrap();
    let s_back = serde_json::from_str::<S>(&j).unwrap();
    assert!(&s.arr[..] == &s_back.arr[..]);
    assert!(false);
}

# fn main() {}
```
*/
#![no_std]

mod array;
pub(crate) mod const_generics;
pub use array::Array;
pub use const_generics::BigArray;
