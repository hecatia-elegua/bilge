use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(2..=4)]
enum Middle {
    A(u5) = 0,
}

fn main() {}
