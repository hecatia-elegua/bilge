use bilge::prelude::*;

#[bitsize(16)]
struct RangeMismatch {
    #[at(8..=11)]
    nack: u3,
}

fn main() {}
