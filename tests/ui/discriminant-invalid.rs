use bilge::prelude::*;


const EXTERNAL: isize = 1;

#[bitsize(1)]
#[derive(FromBits)]
enum B {
    A = EXTERNAL,
    B,
}

#[bitsize(1)]
#[derive(FromBits)]
enum C {
    NineNine = 1,
    PlusPlus = 2,
}

fn main() {}
