use bilge::prelude::*;

#[bitsize(2)]
#[derive(BuilderBits)]
enum Nope {
    A,
    B,
    C,
    D,
}

fn main() {}
