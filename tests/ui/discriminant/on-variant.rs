use bilge::prelude::*;

#[bitsize(8)]
enum OnVariant {
    #[discriminant(u8)]
    A(u8),
}

fn main() {}
