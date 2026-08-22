use bilge::bitsize_internal;
use bilge::prelude::*;

#[bitsize(4)]
#[bitsize_internal]
struct A;

#[bitsize(1)]
#[bitsize_internal]
enum R {
    U,
    OK,
}

#[bitsize(1)]
#[derive(FromBits, bitsize_internal)]
enum X {
    A1,
    A2,
}

fn main() {}
