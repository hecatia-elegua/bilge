use bilge::prelude::*;

#[bitsize(4)]
#[derive(FromBits)]
#[non_exhaustive]
struct A(u4);

#[bitsize(4)]
#[derive(FromBits)]
#[non_exhaustive]
enum B {
    A,
    B,
    C,
}

#[bitsize(4)]
#[derive(TryFromBits)]
#[non_exhaustive]
struct C(u4);

#[bitsize(4)]
#[derive(TryFromBits)]
#[non_exhaustive]
enum D {
    A,
    B,
    C,
}

fn main() {}
