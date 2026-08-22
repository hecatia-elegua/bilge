//! Comparison benches: bilge vs bitbybit, modular-bitfield, deku, and a handwritten baseline.
//!
//! Each library implements the same GIC redistributor registers, including a 2-bit
//! `CommonLpiAffinity` enum. Timed paths do not `assert!` (that would dominate nanosecond measurements).
//! A one-shot `check` runs first so we still notice layout/API mistakes.
//!
//! Construction always starts from the same host integers. Libraries whose native API is
//! byte-oriented (`modular-bitfield`, `deku`) convert with `to_le_bytes` inside the timed function.
//!
//! `from_raw` / `getters` / `set_jep106` are allowed to inline into Criterion's loop
//! (with `black_box` so the work cannot be deleted).
//! `combined` is `#[inline(never)]` so the compiler treats a full register read-modify-write as one function, closer to a driver helper.
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

mod bilge;
mod bitbybit;
mod deku;
mod handmade;
mod modular;

/// Packed register values plus a 12-bit JEP106 id used by the setter.
pub type Input = (u32, u32, u64, u16);

const INPUT: Input = (
    // clear_enable_supported = true (bit 30)
    0b1111_0000_0000_1001_1111_0000_0000_1001,
    // implementer_jep106 = 2054
    0b1000_0000_0110_1001_1111_1111_0010_1001,
    // processor_number = 63872
    0b1000_1111_1111_1001_1000_0000_0110_1001_1111_1111_0010_1001_1111_1111_0010_1001,
    // setter payload (fits in 12 bits)
    0b0101_0111_1111,
);

fn bitfields_compared(c: &mut Criterion) {
    crate::bilge::check(INPUT);
    crate::bitbybit::check(INPUT);
    crate::modular::check(INPUT);
    crate::handmade::check(INPUT);
    crate::deku::check(INPUT);

    let mut from = c.benchmark_group("from_raw");
    from.bench_with_input(BenchmarkId::from_parameter("bilge"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::bilge::from_raw(black_box(*i))))
    });
    from.bench_with_input(BenchmarkId::from_parameter("bitbybit"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::bitbybit::from_raw(black_box(*i))))
    });
    from.bench_with_input(BenchmarkId::from_parameter("modular"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::modular::from_raw(black_box(*i))))
    });
    from.bench_with_input(BenchmarkId::from_parameter("handmade"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::handmade::from_raw(black_box(*i))))
    });
    from.bench_with_input(BenchmarkId::from_parameter("deku"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::deku::from_raw(black_box(*i))))
    });
    from.finish();

    let mut get = c.benchmark_group("getters");
    get.bench_function("bilge", |b| {
        let lpi = crate::bilge::from_raw(INPUT);
        b.iter(|| black_box(crate::bilge::getters(black_box(&lpi))))
    });
    get.bench_function("bitbybit", |b| {
        let lpi = crate::bitbybit::from_raw(INPUT);
        b.iter(|| black_box(crate::bitbybit::getters(black_box(&lpi))))
    });
    get.bench_function("modular", |b| {
        let lpi = crate::modular::from_raw(INPUT);
        b.iter(|| black_box(crate::modular::getters(black_box(&lpi))))
    });
    get.bench_function("handmade", |b| {
        let lpi = crate::handmade::from_raw(INPUT);
        b.iter(|| black_box(crate::handmade::getters(black_box(&lpi))))
    });
    get.bench_function("deku", |b| {
        let lpi = crate::deku::from_raw(INPUT);
        b.iter(|| black_box(crate::deku::getters(black_box(&lpi))))
    });
    get.finish();

    let mut set = c.benchmark_group("set_jep106");
    set.bench_function("bilge", |b| {
        let mut lpi = crate::bilge::from_raw(INPUT);
        b.iter(|| {
            crate::bilge::set_jep106(black_box(&mut lpi), black_box(INPUT.3));
            black_box(&lpi);
        })
    });
    set.bench_function("bitbybit", |b| {
        let mut lpi = crate::bitbybit::from_raw(INPUT);
        b.iter(|| {
            crate::bitbybit::set_jep106(black_box(&mut lpi), black_box(INPUT.3));
            black_box(&lpi);
        })
    });
    set.bench_function("modular", |b| {
        let mut lpi = crate::modular::from_raw(INPUT);
        b.iter(|| {
            crate::modular::set_jep106(black_box(&mut lpi), black_box(INPUT.3));
            black_box(&lpi);
        })
    });
    set.bench_function("handmade", |b| {
        let mut lpi = crate::handmade::from_raw(INPUT);
        b.iter(|| {
            crate::handmade::set_jep106(black_box(&mut lpi), black_box(INPUT.3));
            black_box(&lpi);
        })
    });
    set.bench_function("deku", |b| {
        let mut lpi = crate::deku::from_raw(INPUT);
        b.iter(|| {
            crate::deku::set_jep106(black_box(&mut lpi), black_box(INPUT.3));
            black_box(&lpi);
        })
    });
    set.finish();

    // Construct + get + set + write back to host integers (MMIO-style round trip).
    let mut combined = c.benchmark_group("combined");
    combined.bench_with_input(BenchmarkId::from_parameter("bilge"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::bilge::combined(black_box(*i))))
    });
    combined.bench_with_input(BenchmarkId::from_parameter("bitbybit"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::bitbybit::combined(black_box(*i))))
    });
    combined.bench_with_input(BenchmarkId::from_parameter("modular"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::modular::combined(black_box(*i))))
    });
    combined.bench_with_input(BenchmarkId::from_parameter("handmade"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::handmade::combined(black_box(*i))))
    });
    combined.bench_with_input(BenchmarkId::from_parameter("deku"), &INPUT, |b, i| {
        b.iter(|| black_box(crate::deku::combined(black_box(*i))))
    });
    combined.finish();
}

criterion_group!(benches, bitfields_compared);
criterion_main!(benches);
