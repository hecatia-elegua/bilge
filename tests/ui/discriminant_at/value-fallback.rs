use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(0..=1)]
#[derive(FromBits)]
enum Mixed {
    A(u6) = 0,
    B(u6) = 1,
    #[fallback]
    Other(u8),
}

fn main() {}
