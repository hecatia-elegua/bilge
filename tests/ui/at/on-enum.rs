use bilge::prelude::*;

#[bitsize(2)]
#[at(0)]
enum ItemLevel {
    A,
    B,
    C,
    D,
}

#[bitsize(2)]
enum VariantLevel {
    #[at(0)]
    A,
    B,
    C,
    D,
}

fn main() {}
