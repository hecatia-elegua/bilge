use bilge::prelude::*;

#[bitsize(3)]
#[derive(FromBits)]
enum ButHeresMyNumber {
    So(u3),
    Call,
    Me,
    #[fallback]
    Maybe(u3),
}

fn main() {}