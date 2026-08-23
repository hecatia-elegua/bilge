use bilge::prelude::*;

#[bitsize(8)]
#[discriminant(u8)]
#[derive(FromBits)]
enum Unfilled {
    A(u8) = 0,
    B(u8) = 1,
}

fn main() {}
