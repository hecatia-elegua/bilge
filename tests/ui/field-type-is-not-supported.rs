use bilge::prelude::*;

// NOTE: this only spits out the first field error,
// but I checked the others + it doesn't matter too much
#[bitsize(4)]
struct A {
    // BareFn(_)
    a: fn(),

    // Group(_)
    // b: ???,

    // ImplTrait(_)
    c: impl Debug,

    // Infer(_)
    d: _,

    // Macro(_)
    e: field!(),

    // Never(_)
    f: !,

    // Ptr(_)
    g: *const u8,
    h: *mut u8,

    // Reference(_)
    i: &u8,
    j: &mut u8,

    // Slice(_)
    k: &[u8],

    // TraitObject(_)
    l: dyn Debug,

    // Verbatim(_)
    // ---

    // Paren(_)
    m: (u8),
}

fn main() {}
