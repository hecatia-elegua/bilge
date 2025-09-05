// We're only testing ui on nightly-2022-11-03, until newer
// nightly has some notion of const_convert again.
//
// TODO(upstream): currently just fails
// #[rustversion::attr(not(nightly(2022-11-03)), ignore)]
//
//  so,
// `cargo +nightly test` will fail
// `cargo +nightly-2022-11-03 test` should not fail
#[allow(unused_attributes, clippy::duplicated_attributes)]
#[rustversion::attr(not(nightly), ignore)]
#[cfg_attr(not(feature = "nightly"), ignore)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    t.compile_fail("tests/ui/*/*.rs");
}
