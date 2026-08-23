use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(0..3)]
enum Exclusive {
    A(u5) = 0,
}

fn main() {}
