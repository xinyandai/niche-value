# niche-value

Numeric types that are known **not** to hold one chosen value — so `Option<T>`
(and other niches) are no larger than `T`.

This is a generalization of [`nonmax`](https://docs.rs/nonmax): where `nonmax`
forbids the *maximum* value, `niche-value` forbids an *arbitrary* value chosen
with a const generic, and additionally supports `f32`/`f64`, including
niche-optimized "not NaN", "not infinite", "finite", "nonzero", and
"non-subnormal" types.

```rust
use niche_value::{NonValueU8, NonMaxU8, NonNanF32};
use core::mem::size_of;

// Forbid an arbitrary value:
let v = NonValueU8::<7>::new(3).unwrap();
assert_eq!(v.get(), 3);
assert!(NonValueU8::<7>::new(7).is_none());

// nonmax-compatible specialization:
assert!(NonMaxU8::new(u8::MAX).is_none());

// Niche optimization holds everywhere — even for floats:
assert_eq!(size_of::<Option<NonValueU8<7>>>(), size_of::<u8>());
assert_eq!(size_of::<Option<NonNanF32>>(), size_of::<f32>());
```

## How it works

Every type stores a [`core::num::NonZero`] of `value_bits ^ forbidden_bits`.
Because the stored value can never equal the forbidden bit pattern, the XOR is
never `0`, so the `NonZero` niche survives and `Option<T>` reuses it. This is
exactly `nonmax`'s trick with the forbidden value made generic. The checked
constructor compiles to a couple of branchless instructions, and the layout
guarantee is enforced at **compile time** with `const` size assertions — a
future layout regression fails the build rather than silently doubling your size.

## Types

### Integers (all 12 widths `u8`…`i128`, `usize`/`isize`)

| Type | Forbids |
|------|---------|
| `NonValueU8<const N: u8>`, … | the value `N` |
| `NonMaxU8`, … (alias of `NonValue*<{MAX}>`) | `T::MAX` — drop-in for `nonmax` |
| `NonMinU8`, … (alias of `NonValue*<{MIN}>`) | `T::MIN` |

`NonMax*` carries the full `nonmax` surface (`ZERO`/`ONE`/`MAX` consts,
`Default`, `BitAnd`, widening `From`, `TryFromIntError`/`ParseIntError`), so it
is a drop-in superset: `use nonmax::NonMaxU8` → `use niche_value::NonMaxU8`.

### Floats (`f32`/`f64`)

| Type | Forbids | `Ord`/`Eq`/`Hash`? |
|------|---------|:-:|
| `NonValueF32<const BITS: u32>`, `NonValueF64<const BITS: u64>` | one bit pattern | no¹ |
| `NonMaxF32`, `NonMinF32`, … | `T::MAX` / `T::MIN` (bit-exact) | no¹ |
| `NonNanF32`, `NonNanF64` | **every** `NaN` bit pattern | **yes** |
| `NonInfF32`, `NonInfF64` | both infinities | no¹ |
| `NonZeroF32`, `NonZeroF64` | **both** zeros (`+0.0` *and* `-0.0`) | no¹ |
| `FiniteF32`, `FiniteF64` | `NaN` **and** both infinities | **yes** |
| `NonSubnormalF32`, `NonSubnormalF64` | subnormals | no¹ |

¹ These can still hold `NaN`, so only `PartialEq`/`PartialOrd` (by value,
matching the primitive) are provided.

`FiniteF*` (= `NonNan*` ∩ `NonInf*`) can never hold `NaN`, so like `NonNan*` it
gets a total `Ord`/`Eq`/`Hash` (with the same `-0.0`-normalized `Hash`).
`NonZeroF*` rejects zero as a **class** — both `+0.0` and `-0.0` — which is
distinct from the bit-exact `NonValueF32<0x0000_0000>` that forbids only `+0.0`
and leaves `-0.0` valid.

`NonNan*` is the highlight: because it can never hold `NaN`, it implements a
total `Ord`/`Eq`/`Hash` — a **niche-optimized `NotNan`**, something
`ordered-float`/`decorum` don't give you. `+0.0`/`-0.0` compare and hash equal
(`-0.0` is normalized only inside `Hash`), while `get()` round-trips the sign.

## Float semantics: bit-pattern, not value

Floats reject by **bit pattern**, never by mathematical value. This is forced by
soundness — a value check would let a `NaN` equal to the forbidden pattern slip
through (`NaN != NaN`) and form an unsound `NonZero(0)`. Two consequences:

- `+0.0` and `-0.0` are **distinct** patterns; forbidding one permits the other.
- `NonValueF32<BITS>` forbids exactly **one** bit pattern. To reject *all* `NaN`
  or *all* infinities, use the class-based `NonNan*` / `NonInf*` types.

### Why there is no niche-optimized "reject all NaN except at construction" surprise

`NonNan*` rejects all ~16.7M `NaN` patterns at construction but anchors its niche
on a single representative `NaN`. Reclaiming *one* niche (for `Option`) only
requires *one* guaranteed-absent pattern, and "no NaN" guarantees that. Fatter
niches (reclaiming a whole class for multi-variant enums) would require unstable
compiler features and are out of scope; this crate is stable-only.

## Features

- `std` (default) — implements `std::error::Error` for the error types.
- `serde` — `Serialize`/`Deserialize` for every type.

Disable default features for `#![no_std]`.

## MSRV

Rust 1.83.

## License

MIT OR Apache-2.0.
