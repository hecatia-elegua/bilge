use bilge::prelude::*;

#[bitsize(8)]
struct Duplicate {
    #[at(0)]
    #[at(1)]
    a: bool,
}

fn main() {}
