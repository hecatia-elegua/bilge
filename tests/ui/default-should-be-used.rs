use bilge::prelude::*;

#[bitsize(2)]
#[derive(DefaultBits)]
enum One {
    Small,
    #[default]
    Step,
}

fn main() {}
