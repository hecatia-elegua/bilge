use bilge::prelude::*;

#[bitsize(15)]
#[derive(FromBits)]
enum Testing {
    A,
    D,
    H,
    #[fallback]
    Dee { fallback: u15 },
}

#[bitsize(15)]
#[derive(FromBits)]
enum Windows {
    I,
    N,
    #[fallback]
    Tel(u8, u7),
}

#[bitsize(15)]
#[derive(FromBits)]
enum Alt {
    F,
    #[fallback]
    Four(Option<u8>),
}

#[bitsize(15)]
#[derive(FromBits)]
enum James {
    Atomic,
    #[fallback]
    Habits(u100),
}

#[bitsize(15)]
#[derive(FromBits)]
struct Dum {
    #[fallback]
    my: u15,
}

fn main() {}
