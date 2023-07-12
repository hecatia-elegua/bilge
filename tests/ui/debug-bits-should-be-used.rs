use bilge::prelude::*;

// structs should use `DebugBits`
#[bitsize(4)]
#[derive(Debug)]
struct A(u4);

// might as well test deeper paths
#[bitsize(4)]
#[derive(fmt::Debug)]
struct A(u4);

#[bitsize(4)]
#[derive(core::fmt::Debug)]
struct A(u4);

#[bitsize(4)]
#[derive(std::fmt::Debug)]
struct A(u4);

// enums should use `Debug`
#[bitsize(1)]
#[derive(DebugBits)]
enum B { A, B }

fn main() {}
