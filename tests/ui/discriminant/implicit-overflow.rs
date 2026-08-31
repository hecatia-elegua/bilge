use bilge::prelude::*;

#[bitsize(4)]
#[derive(TryFromBits)]
enum Overflow {
    A = 15,
    B,
}

fn main() {
    // Would panic in `u4::from(Overflow::B)` / `as_int` if this compiled.
    let _ = Overflow::B;
}
