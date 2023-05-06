# bilge: the most readable bitfields

_Y e s_, this is yet another bitfield crate, but hear me out:

This is a _**bit**_ better than what we had before.

I wanted a design fitting rust:

- **safe**
    - types model as much of the functionality as possible and don't allow false usage
- **fast**
    - like handwritten bit fiddling code
- **simple to complex**
    - obvious and readable basic frontend, like normal structs
    - only minimally and gradually introduce advanced concepts

## WARNING

Our current version is still pre 1.0, which means nothing is completely stable.
The lib is **no-std** and fully `const`, but still nightly-only (needed for several `const` features).

However, constructors, getters, setters and From/TryFrom should stay the same, since their semantics are very clear.

[//]: # (keep this fixed to the version in .github/workflows/ci.yml, rust-toolchain.toml)

The current version is tested on `nightly-2022-11-03`.

## Usage

To make your life easier:
```rust
use bilge::prelude::*;
```

### Infallible (From)
You can just specify bitsized fields like normal fields:
```rust
#[bitsize(14)]
struct Register {
    header: u4,
    body: u7,
    footer: Footer,
}
```
The attribute `bitsize` generates the bitfield, while `14` works as a failsafe, emitting a compile error if your struct definition doesn't declare 14 bits.
Let's define the nested struct `Footer` as well:
```rust
#[bitsize(3)]
#[derive(FromBits)]
struct Footer {
    is_last: bool,
    code: Code,
}
```
As you can see, we added `#[derive(FromBits)]`, which is needed for `Register`'s getters and setters.
Due to how rust macros work (outside-in), it needs to be below `#[bitsize]`.
Also, `bool` can be used as one bit.

`Code` is another nesting, this time an enum:
```rust
#[bitsize(2)]
#[derive(FromBits)]
enum Code { Success, Error, IoError, GoodExample }
```
Now we can construct `Register`:
```rust
let reg1 = Register::new(
    u4::new(0b1010),
    u7::new(0b010_1010),
    Footer::new(true, Code::GoodExample)
);
```
Or, if we add `#[derive(FromBits)]` to `Register` and want to parse a raw register value:
```rust
let mut reg2 = Register::from(u14::new(0b11_1_0101010_1010));
```
And getting and setting fields is done like this:
```rust
let header = reg2.header();
reg2.set_footer(Footer::new(false, Code::Success));
```
Any kinds of tuple and array are also supported.

### Fallible (TryFrom)
In contrast to structs, enums don't have to declare all of their bits:
```rust
#[bitsize(2)]
#[derive(TryFromBits)]
enum Class {
    Mobile, Semimobile, /* 0x2 undefined */ Stationary = 0x3
}
```
meaning this will work:
```rust
let class = Class::try_from(u2::new(2));
assert_eq!(class, Err(u2::new(2)));
```
except we first need to `#[derive(Debug, PartialEq)]` on `Class`, since `assert_eq!` needs those.

Let's do that, and use `Class` as a field:
```rust
#[bitsize(8)]
#[derive(TryFromBits)]
struct Device {
    reserved: u2,
    class: Class,
    reserved: u4,
}
```
This shows `TryFrom` being propagated upward. There's also another small help: `reserved` fields (which are often used in registers) can all have the same name.

Again, let's try to print this:
```rust
println!("{:?}", Device::try_from(0b0000_11_00));
```
And again, `Device` doesn't implement `Debug`:

### DebugBits

For structs, you need to add `#[derive(DebugBits)]` to get an output like this:
```rust
Ok(Device { reserved_i: 0, class: Stationary, reserved_ii: 0 })
```

## Back- and Forwards Compatibility

The syntax is kept very similar to usual rust structs for a simple reason:

The endgoal of this library is to support the adoption of LLVM's arbitrary bitwidth integers into rust,
thereby allowing rust-native bitfields.
Until then, bilge is using the wonderful [`arbitrary-int` crate by danlehmann](https://github.com/danlehmann/arbitrary-int).

After all attribute expansions, our generated bitfield contains a single field, somewhat like:
```rust
struct Register { value: u14 }
```
This means you _could_ modify the inner value directly, but it breaks type safety guarantees (e.g. unfilled or read-only fields).
So if you need to modify the whole field, instead use the type-safe conversions `u14::from(register)` and `Register::from(u14)`.
It is possible that this inner type will be made private.

You also mustn't depend on the generated `const FILLED`.

More `/tests` will follow. Right now the `/examples` directory is used for some testing and as a functionality overview.

## Alternatives

### benchmarks, performance, asm line count
First of all, [basic benchmarking](https://github.com/hecatia-elegua/bilge/blob/main/benches/compared/main.rs) reveals that all alternatives mentioned here (besides deku) have about the same performance and line count. This includes a handwritten version.

### handwritten implementation
The common handwritten implementation pattern for bitfields in rust looks [somewhat like benches/compared/handmade.rs](https://github.com/hecatia-elegua/bilge/blob/main/benches/compared/handmade.rs), sometimes also throwing around a lot of consts for field offsets. The problems with this approach are:
- readability suffers
- offset, cast or masking errors could go unnoticed
- bit fiddling, shifting and masking is done all over the place, in contrast to bitfields
- beginners suffer, although I would argue even seniors, since it's more like: "Why do we need to learn and debug bit fiddling if we can get most of it done by using structs?"
- reimplementing different kinds of _fallible nested-struct enum-tuple array field access_ might not be so fun

### modular-bitfield
The often used and very inspiring [`modular-bitfield`](https://github.com/robbepop/modular-bitfield) has a few
problems:
- it is unmaintained and has a quirky structure
- constructors use the builder pattern
    - makes user code unreadable if you have many fields
    - can accidentally leave things uninitialized
- `from_bytes` can easily take invalid arguments, which turns verification inside-out:
    - modular-bitfield flow: `u16` -> `PackedData::from_bytes([u16])` -> `PackedData::status_or_err()?`
        - needs to check for `Err` on every single access
        - adds duplicate getters and setters with postfix `_or_err`
        - reinvents `From<u16>`/`TryFrom<u16>` as a kind of hybrid
    - bilge: usual type-system centric flow: `u16` -> `PackedData::try_from(u16)?` -> `PackedData::status()`
        - just works, needs to check nothing on access
        - some more general info on this: [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- big god-macro
    - powerful, but less readable to the devs of modular-bitfield
    - needs to cover many derives in itself, like `impl Debug` (other bitfield crates do this as well)
        - bilge: solves this by providing a kind of scope for `-Bits`-derives

and implementation differences:
- underlying type is a byte array
    - can be useful for bitfields larger than u128
        - bilge: if your bitfields get larger than u128, you can most often split them into multiple bitfields of a primitive size (like u64) and put those in a parent struct which is not a bitfield

Still, modular-bitfield is pretty good and I had set out to build something equal or hopefully better than it.
Tell me where I can do better, I will try.

### bitbybit
One of the libs inspired by the same crate is [`bitbybit`](https://github.com/danlehmann/bitfield), which is much more readable and up-to-date. Actually, I even helped and am still helping on that one as well. After experimenting and hacking around in their code though, I realized it would need to be severely changed for the features and structure I had in mind.

implementation differences (as of 26.04.23):
- it can do read/write-only, array strides and repeat the same bits for multiple fields
    - bilge: these will be added the moment someone needs it
- redundant bit-offset specification, which can help or annoy, the same way bilge's `reserved` fields can help or annoy

### deku
After looking at a ton of bitfield libs on crates.io, I _didn't_ find [`deku`](https://github.com/sharksforarms/deku).
I will still mention it here because it uses a very interesting crate underneath (bitvec).
Currently (as of 26.04.23), it generates far more assembly and takes longer to run, since parts of the API are not `const`.
I've opened an issue on their repo about that.

### most others
Besides that, many bitfield libs try to imitate or look like C bitfields, even though these are hated by many.
I argue most beginners would have the idea to specify bits with basic primitives like u1, u2, ...
This also opens up some possibilities for calculation and conversion on those primitives.

Something similar can be said about `bitflags`, which, under this model, can be turned into simple structs with bools and enums.

Basically, `bilge` tries to convert bit fiddling, shifting and masking into more widely known concepts like struct access.

About the name: a bilge is one of the "lowest" parts of a ship, nothing else to it :)
