use bilge::prelude::*;

#[bitsize(8)]
struct Exclusive {
    #[at(4..8)]
    a: u4,
}

fn main() {}
