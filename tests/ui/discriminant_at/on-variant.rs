use bilge::prelude::*;

#[bitsize(8)]
enum OnVariant {
    #[discriminant_at(0)]
    A(u7),
}

fn main() {}
