use bilge::prelude::*;

#[bitsize(8)]
#[discriminant(u128)]
#[derive(FromBits)]
enum TooWide {
    A(u8) = 0,
}

#[bitsize(65)]
#[derive(FromBits)]
struct WideTag {
    bits: u65,
}

#[bitsize(8)]
#[discriminant(WideTag)]
#[derive(TryFromBits)]
enum TooWideCustom {
    A(u8) = 0,
}

fn main() {}
