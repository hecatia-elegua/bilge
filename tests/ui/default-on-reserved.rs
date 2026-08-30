use bilge::prelude::*;

#[bitsize(8)]
#[derive(DefaultBits)]
struct Reg {
    a: u4,
    #[default(u4::new(0))]
    reserved: u4,
}

fn main() {}
