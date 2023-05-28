pub trait CustomBits {
    fn fields(&self) -> usize;
}
pub use custom_bits_derive::CustomBits;
