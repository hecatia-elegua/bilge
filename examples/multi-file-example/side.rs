use bilge::prelude::*;

#[bitsize(6)]
#[derive(DebugBits, FromBits)]
pub struct Sibling {
    field: u2,
    pub other: u4,
}
