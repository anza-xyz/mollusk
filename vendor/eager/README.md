# eager
[![Build Status](https://travis-ci.org/Emoun/eager.svg?branch=master)](https://travis-ci.org/Emoun/eager)
[![](http://meritbadge.herokuapp.com/eager)](https://crates.io/crates/eager)
[![](https://docs.rs/eager/badge.svg)](https://docs.rs/eager)


Rust crate for simulating eager macro expansion.

### Example

```Rust
#[macro_use]
extern crate eager;

//Declare an eager macro
eager_macro_rules!{ $eager_1
    macro_rules! plus_1{
        ()=>{+ 1};
    }
}

fn main(){
	// Use the macro inside an eager! call to expand it eagerly
	assert_eq!(4, eager!{2 plus_1!() plus_1!()});
}
```

### License 

Licensed under the MIT license.

