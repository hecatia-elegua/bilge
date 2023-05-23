//! Just a crude compile test to see that using a field across files works.
use bilge::prelude::*;

mod side;
use side::Sibling;

#[bitsize(8)]
#[derive(DebugBits, FromBits)]
pub struct Example {
    sibling: Sibling,
    field: u2,
}

fn main() {}
