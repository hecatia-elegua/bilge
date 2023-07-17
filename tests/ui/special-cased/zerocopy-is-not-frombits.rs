use bilge::prelude::*;

#[bitsize(32)]
#[derive(zerocopy::FromBytes)]
struct Group([bool; 32]);

#[bitsize(8)]
#[derive(zerocopy::FromBytes)]
enum Packet {
    A,
    B,
}

fn main() {}
