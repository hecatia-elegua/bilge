use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(0..=1)]
#[discriminant_at(0..=1)]
enum Duplicate {
    A(u6) = 0,
}

fn main() {}
