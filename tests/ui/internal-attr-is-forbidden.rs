#![feature(const_trait_impl)]
use bilge::bitsize_internal;
use bilge::prelude::*;

// TODO?: validating `bitsize_internal` is not used alone, like:
// #[bitsize_internal] struct A;
// would be possible by generating a marker trait or sth in `bitsize`

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
