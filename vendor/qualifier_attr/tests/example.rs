#![allow(dead_code)]

#[macro_use]
extern crate qualifier_attr;

// We can add a qualifier to a function
// with an attribute.
#[qualifiers(const)]
fn const_fn() -> u32 {
    42
}

const CONST_RES: u32 = const_fn();

// It's not so impressive on its own,
// but with `cfg_attr`, it can be conditional.
#[cfg_attr(feature = "extern_c", no_mangle, qualifiers(pub, extern "C"))]
fn extern_c_fn() -> u32 {
    42
}

// It even works with types, imports, and more!
mod foo {
    #[qualifiers(pub)]
    struct Foo {
        x: i32,
        y: i32,
    }
}

#[qualifiers(pub)]
use foo::Foo;

// Traits and implementations too!?
#[cfg_attr(feature = "unsafe_quux", qualifiers(unsafe))]
trait Quux {
    fn quux_the_thing();
}

#[cfg_attr(feature = "unsafe_quux", qualifiers(unsafe))]
impl Quux for Foo {
    fn quux_the_thing() {
        println!("The thing was quuxed.");
    }
}

// You can add qualifiers to the fields of a
// struct as well with this special attribute.
#[field_qualifiers(x(pub), y(pub))]
struct Point2 {
    x: i32,
    y: i32,
}

#[field_qualifiers(_0(pub), _1(pub), _2(pub))]
struct Point3(i32, i32, i32);
