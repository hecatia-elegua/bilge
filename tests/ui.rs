// We're only testing ui on stable,
// since the errors are pretty much the same,
// but change the position of help in the error message.
//
// #[rustversion::attr(not(stable), ignore)]
// #[cfg_attr(feature = "nightly", ignore)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    t.compile_fail("tests/ui/*/*.rs");
}
