use bilge::prelude::*;

#[bitsize(8)]
#[discriminant_at(CrtcIndex)]
enum WrongAt {
    A(u8) = 0,
}

fn main() {}
