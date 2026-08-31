use bilge::prelude::*;

// Payload-width fallback is rejected; it must be the full enum width (`u8` here).
#[bitsize(8)]
#[discriminant_at(0..=1)]
#[derive(FromBits)]
enum Mixed {
    A(u6) = 0,
    B(u6) = 1,
    #[fallback]
    Other(u6),
}

fn main() {}
