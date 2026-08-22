use bilge::prelude::*;

#[bitsize(2, hide_value)]
#[derive(FromBits)]
enum Nope {
    A,
    B,
    C,
    D,
}

fn main() {}
