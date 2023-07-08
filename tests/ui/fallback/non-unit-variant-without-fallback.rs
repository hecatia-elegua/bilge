use bilge::prelude::*;

#[bitsize(2)]
#[derive(FromBits)]
enum IJustMetYou {
    And,
    This,
    Is(u2),
    Crazy,
}

fn main() {}