use bilge::prelude::*;

#[bitsize(16)]
struct Overlap {
    a: u8,
    #[at(4)]
    b: u4,
}

fn main() {}
