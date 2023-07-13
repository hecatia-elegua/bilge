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

// I just put this here since I noticed it here
#[bitsize(1)]
#[derive(FromBits)]
enum D {
    NineNine = 0,
    Sharp = 1,
    #[fallback]
    PlusPlus,
}

#[bitsize(1)]
#[derive(FromBits)]
enum E {
    NineNine = 0,
    Sharp = 1,
    PlusPlus = 2,
}

fn main() {}
