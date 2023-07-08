use bilge::prelude::*;

#[bitsize(9)]
#[derive(FromBits)]
enum Testing {
    #[fallback]
    A(u9),
    B,
}

fn main() {}