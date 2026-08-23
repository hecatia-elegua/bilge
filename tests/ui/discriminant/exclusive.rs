use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(0..=1)]
#[discriminant(u8)]
enum Both {
    A(u6) = 0,
}

fn main() {}
