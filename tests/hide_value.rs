#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
use bilge::prelude::*;

#[bitsize(8, hide_value, new = pub)]
#[derive(BuilderBits, FromBits, PartialEq, DebugBits)]
struct HiddenBuilt {
    a: u4,
    #[default(u4::new(2))]
    b: u4,
}

#[test]
fn hide_value_builder() {
    let bits = HiddenBuilt::builder().a(u4::new(1)).build();
    assert_eq!(bits, HiddenBuilt::new(u4::new(1), u4::new(2)));
}

#[test]
fn hide_value_builder_type_keeps_its_name() {
    let builder: HiddenBuiltBuilder<(), ()> = HiddenBuilt::builder();
    let bits = builder.a(u4::new(1)).build();
    assert_eq!(bits.a(), u4::new(1));
}

#[bitsize(8, hide_value)]
#[derive(FromBits, PartialEq, DebugBits)]
struct Hidden {
    a: u4,
    b: u4,
}

#[test]
fn hide_value_from_and_accessors_still_work() {
    let mut bits = Hidden::new(u4::new(1), u4::new(2));
    assert_eq!(bits.a(), u4::new(1));
    assert_eq!(bits.b(), u4::new(2));
    bits.set_b(u4::new(3));
    assert_eq!(u8::from(bits), u8::from(Hidden::from(u8::new(0b0011_0001))));
}

#[bitsize(8, hide_value)]
#[derive(FromBits)]
pub struct NestedHidden {
    inner: u4,
    outer: u4,
}

impl NestedHidden {
    pub fn sum(&self) -> u8 {
        u8::from(self.inner()) + u8::from(self.outer())
    }
}

#[test]
fn hide_value_getters_work_in_defining_module() {
    let bits = NestedHidden::new(u4::new(1), u4::new(2));
    assert_eq!(bits.sum(), 3);
}

#[bitsize(8)]
#[derive(FromBits, PartialEq, DebugBits)]
pub struct PrivateNew {
    pub a: u4,
    pub b: u4,
}

impl PrivateNew {
    pub fn pair(a: u4, b: u4) -> Self {
        Self::new(a, b)
    }
}

#[test]
fn private_new_is_callable_in_defining_module() {
    let bits = PrivateNew::pair(u4::new(1), u4::new(2));
    assert_eq!(bits, PrivateNew::from(u8::new(0b0010_0001)));
}

#[bitsize(8, hide_value, new = pub(crate))]
#[derive(FromBits, PartialEq, DebugBits)]
struct HiddenCrateNew {
    pub a: u8,
}

#[test]
fn hide_value_and_new_vis_can_be_combined() {
    let bits = HiddenCrateNew::new(0);
    assert_eq!(bits.a(), 0);
    assert_eq!(u8::from(bits), 0);
}

mod super_new {
    use super::*;

    #[bitsize(8, hide_value, new = pub(super))]
    #[derive(FromBits)]
    pub struct HiddenSuperNew {
        pub a: u8,
    }
}

#[test]
fn hide_value_new_pub_super_is_visible_in_parent_module() {
    let bits = super_new::HiddenSuperNew::new(7);
    assert_eq!(bits.a(), 7);
}

#[bitsize(4)]
#[derive(FromBits)]
struct Child {
    inner: u4,
}

#[bitsize(8, hide_value)]
#[derive(FromBits)]
struct Parent {
    child: Child,
    extra: u4,
}

#[test]
fn hide_value_is_not_viral_for_nested_bitfields() {
    let parent = Parent::new(Child::new(u4::new(1)), u4::new(2));
    // Child did not opt into hide_value, so its raw field is still module-visible.
    assert_eq!(parent.child().value, u4::new(1));
    assert_eq!(parent.extra(), u4::new(2));
}
