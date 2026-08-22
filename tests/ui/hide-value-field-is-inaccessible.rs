use bilge::prelude::*;

#[bitsize(8, hide_value)]
#[derive(FromBits)]
struct Hidden {
    a: u4,
    b: u4,
}

fn main() {
    let mut hidden = Hidden::from(u8::new(0));
    hidden.value = u8::new(1);
}
