use bilge::prelude::*;

#[bitsize(2, new = pub)]
#[derive(FromBits)]
enum Nope {
    A,
    B,
    C,
    D,
}

fn main() {}
