use bilge::prelude::*;

// negative value
#[bitsize(-1)]
enum Test {}

// one below lowest value
#[bitsize(0)]
enum Test {}

// one above highest (struct) value
#[bitsize(129)]
struct Test {}

// one above highest enum value
#[bitsize(65)]
enum Test {}

// one above highest enum value, this compiles
#[bitsize(65)]
struct Test { field: u65 }

fn main() {}
