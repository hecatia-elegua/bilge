use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(0..=1)]
#[derive(FromBits)]
enum TagOnly {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

#[bitsize(8)]
#[discriminant_at(0..=1)]
#[derive(FromBits)]
enum MixedUnit {
    A(u6) = 0,
    #[fallback]
    Unknown,
}

fn main() {}
