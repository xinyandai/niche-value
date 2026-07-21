//! `niche_value` provides numeric types that are known **not** to hold one
//! chosen value, so that `Option<T>` (and other niches) are no larger than `T`.
//!
//! It generalizes [`nonmax`](https://docs.rs/nonmax): where `nonmax` forbids the
//! *maximum* value, this crate forbids an *arbitrary* value chosen with a const
//! generic — [`NonValueU8<N>`], [`NonValueI32<N>`], … — and additionally
//! supports floats ([`NonValueF32<BITS>`], [`NonValueF64<BITS>`]) plus
//! semantic-class float types ([`NonNanF32`], [`NonInfF32`], …).
//!
//! # Mechanism
//!
//! Every type stores a [`core::num::NonZero`] of `value_bits ^ forbidden_bits`.
//! Because the stored value can never equal the forbidden bit pattern, the XOR
//! is never `0`, so the `NonZero` niche survives and `Option<T>` reuses it.
//! This is exactly `nonmax`'s trick, with the forbidden value made generic.
//!
//! ```
//! use niche_value::{NonValueU8, NonMaxU8, NonNanF32};
//! use core::mem::size_of;
//!
//! // Forbid an arbitrary value:
//! let v = NonValueU8::<7>::new(3).unwrap();
//! assert_eq!(v.get(), 3);
//! assert!(NonValueU8::<7>::new(7).is_none());
//!
//! // nonmax-compatible specialization:
//! assert!(NonMaxU8::new(u8::MAX).is_none());
//!
//! // Niche optimization holds everywhere:
//! assert_eq!(size_of::<Option<NonValueU8<7>>>(), size_of::<u8>());
//! assert_eq!(size_of::<Option<NonNanF32>>(), size_of::<f32>());
//! ```
//!
//! # Float semantics
//!
//! Float types reject by **bit pattern**, not by mathematical value (this is
//! forced by soundness — see the crate README). Consequences: `+0.0` and `-0.0`
//! are distinct, and [`NonValueF32<BITS>`] forbids exactly one bit pattern.
//! The class-based [`NonNanF32`]/[`NonInfF32`] reject the whole `NaN` / infinite
//! class at construction while anchoring their niche on one representative
//! pattern. Because [`NonNanF32`] can never hold `NaN`, it is the only float
//! family that implements total [`Ord`]/[`Eq`]/[`Hash`] — a niche-optimized
//! `NotNan`.
//!
//! # Features
//!
//! * `std` (default): implements [`std::error::Error`] for the error types.
//!   Disable for `#![no_std]`.
//! * `serde`: `Serialize`/`Deserialize` for every type.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

mod error;
mod float;
mod int;

pub use error::{ParseFloatError, ParseIntError, TryFromFloatError, TryFromIntError};
pub use float::*;
pub use int::*;
