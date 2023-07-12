use bilge::prelude::*;

#[bitsize(32)]
#[derive(zerocopy::FromBytes)]
struct Group([bool; 32]);

fn main() {}
