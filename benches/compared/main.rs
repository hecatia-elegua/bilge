#![feature(const_trait_impl, const_convert, const_mut_refs)]
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

mod bilge;
mod bitbybit;
mod modular;
mod handmade;
// mod deku;

/// This is a benchmark testing basic construction from bytes and getters, setters.
/// For modular and deku, the input is converted inside their respective function,
/// since that conversion is const.
/// 
/// on my random hardware config, these are all running at ~2.4ns, besides deku at ~15µs
pub fn bitfields_compared(c: &mut Criterion) {
    let input1: (u32, u32, u64, u16) = (
        // clear_enable_supported = true
        0b1111_0000_0000_1001_1111_0000_0000_1001,
        // implementer_jep106 = 2054
        0b1000_0000_0110_1001_1111_1111_0010_1001,
        // processor_number = 63872
        0b1000_1111_1111_1001_1000_0000_0110_1001_1111_1111_0010_1001_1111_1111_0010_1001,
        // for setting implementer_jep106
        0b0101_0111_1111
    );

    let mut group = c.benchmark_group("compared");
    // for i in [input1, ...].iter() {
        group.bench_with_input(
            BenchmarkId::new("bilge", "input1"), &input1, |b, i| b.iter(|| crate::bilge::bilge(*i))
        );
        group.bench_with_input(
            BenchmarkId::new("bitbybit", "input1"), &input1, |b, i| b.iter(|| crate::bitbybit::bitbybit(*i))
        );
        group.bench_with_input(
            BenchmarkId::new("modular", "input1"), &input1, |b, i| b.iter(|| crate::modular::modular(*i))
        );
        group.bench_with_input(
            BenchmarkId::new("handmade", "input1"), &input1, |b, i| b.iter(|| crate::handmade::handmade(*i))
        );
        // uncomment once deku/bitvec is more `const`
        // group.bench_with_input(
        //     BenchmarkId::new("deku", "input1"), &input1, |b, i| b.iter(|| crate::deku::deku(*i))
        // );
    // }
    group.finish();
}

criterion_group!(benches, bitfields_compared);
criterion_main!(benches);
