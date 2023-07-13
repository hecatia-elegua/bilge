use bilge::prelude::*;

#[bitsize(4)]
#[derive(FromBits)]
enum A {
    B, C
}

#[bitsize(2)]
#[derive(FromBits)]
enum F {
    B,
    C,
    D,
    #[fallback]
    E
}

#[bitsize(2)]
#[derive(TryFromBits)]
enum X {
    M,
    A,
    S(u6),
}

#[bitsize(2)]
#[derive(TryFromBits)]
enum L {
    I,
    K,
    #[fallback]
    E(u2),
}

fn main() {}
