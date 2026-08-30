use bilge::prelude::*;

#[bitsize(8)]
#[derive(BuilderBits)]
struct Reg {
    a: u4,
    b: u4,
}

fn main() {
    let _ = Reg::builder().a(u4::new(1)).build();
}
