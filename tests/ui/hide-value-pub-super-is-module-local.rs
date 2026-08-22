use bilge::prelude::*;

mod inner {
    use super::*;

    #[bitsize(8, hide_value, new = pub(super))]
    #[derive(FromBits)]
    pub struct Foo {
        a: u4,
        pub(super) b: u4,
    }

    fn _in_defining_module() {
        let mut foo = Foo::new(u4::new(0), u4::new(1));
        let _ = foo.a();
        foo.set_a(u4::new(2));
        let _ = foo.b();
    }
}

fn main() {
    let mut foo = inner::Foo::from(u8::new(0));
    let _ = foo.b();
    let _ = inner::Foo::new(u4::new(0), u4::new(0));
    let _ = foo.a();
}
