use bilge::prelude::*;

#[bitsize(15)]
#[derive(FromBits)]
enum Testing {
    #[fallback]
    A(u3),
    B,
    #[fallback]
    C,
    D
}

fn main() {}