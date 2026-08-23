use bilge::prelude::*;

#[bitsize(8)]
struct StartsPastEnd {
    #[at(8)]
    a: bool,
}

#[bitsize(8)]
struct RangePastEnd {
    #[at(6..=8)]
    a: u3,
}

#[bitsize(8)]
struct ExtendsPastEnd {
    #[at(6)]
    a: u4,
}

fn main() {}
