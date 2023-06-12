pub trait FieldsInBits {
    fn field_count() -> usize;
}
pub use custom_bits_derive::FieldsInBits;
