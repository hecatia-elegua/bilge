use bilge::prelude::*;

#[bitsize(16)]
struct Reorder {
    #[at(8)]
    a: u8,
    #[at(0)]
    b: u4,
}

fn main() {}
