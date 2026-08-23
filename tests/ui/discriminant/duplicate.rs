use bilge::prelude::*;

#[bitsize(8)]
#[discriminant(u8)]
#[discriminant(u8)]
enum Duplicate {
    A(u8) = 0,
}

fn main() {}
