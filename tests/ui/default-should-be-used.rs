#![feature(const_trait_impl)] use bilge::prelude::*;

#[bitsize(2)]
#[derive(DefaultBits)]
enum One {
    Small,
    #[default]
    Step,
}

#[bitsize(2)]
#[derive(DefaultBits)]
struct A {
    b: Inner,
}

#[bitsize(2)]
#[derive(FromBits)]
enum Inner {
    Zero,
    One,
    Two,
    Three,
}

fn main() {}
