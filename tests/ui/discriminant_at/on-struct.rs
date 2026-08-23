use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(0..=1)]
struct OnStruct {
    a: u8,
}

fn main() {}
