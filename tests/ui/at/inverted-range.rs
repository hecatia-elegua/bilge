use bilge::prelude::*;

#[bitsize(8)]
struct Inverted {
    #[at(4..=1)]
    a: u4,
}

fn main() {}
