use bilge::prelude::*;

#[bitsize(32)]
#[derive(TryFromBits, PartialEq, DebugBits)]
struct Wrapper {
    foo: FillsU32
}

#[bitsize(32)]
#[derive(TryFromBits, PartialEq, Debug)]
enum FillsU32 {
    Foo = 0xDEADBEEF
}

#[test]
fn single_filled_enum_works_issue_36() {
    let foo = Wrapper::try_from(0xDEADBEEF);
    assert_eq!(foo, Ok(Wrapper::new(FillsU32::Foo)));
}