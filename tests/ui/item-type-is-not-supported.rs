use bilge::bitsize;

#[bitsize(8)]
union Test {
    a: u8,
    b: (u4, u4),
}

fn main() {}
