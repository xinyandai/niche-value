//! Float niche types.
//!
//! Two families:
//! * **bit-exact** — [`NonValueF32`]/[`NonValueF64`], forbidding one bit pattern
//!   (const generic `BITS`), plus the `NonMax*`/`NonMin*` aliases.
//! * **class-based** — [`NonNanF32`], [`NonInfF32`], [`NonZeroF32`],
//!   [`FiniteF32`], [`NonSubnormalF32`], … which reject a whole semantic class
//!   at construction while anchoring their niche on one representative pattern.
//!   [`NonNanF32`]/[`FiniteF32`] can never hold `NaN`, so they alone get a total
//!   [`Ord`]/[`Eq`]/[`Hash`].
//!
//! # Anchors and the soundness rule
//!
//! Every type stores `value.to_bits() ^ ANCHOR` in a [`core::num::NonZero`], so
//! the niche survives exactly as long as no *constructible* value can have
//! `to_bits() == ANCHOR`. The two families guarantee that differently:
//!
//! * **bit-exact** types reject by **bit pattern** (`to_bits() == BITS`): the one
//!   forbidden pattern is the anchor and nothing else is touched. A consequence
//!   is that `+0.0` and `-0.0` are distinct patterns — forbidding one leaves the
//!   other valid — and a bit-exact type forbids exactly one of them.
//! * **class-based** types reject by **value / semantics** (`is_nan()`,
//!   `== 0.0`, `is_subnormal()`, …) and anchor on one representative pattern
//!   *drawn from the rejected class*, so the anchor is itself rejected and hence
//!   never constructible.
//!
//! A value predicate is sound only when the anchor compares **equal to itself**.
//! `+0.0 == +0.0`, so [`NonZeroF32`] may reject with `value == 0.0` (its anchor
//! is `+0.0`). `NaN != NaN`, so a `value == NaN` check would let the anchor slip
//! through and form an unsound `NonZero(0)`; [`NonNanF32`] must instead reject
//! with `is_nan()`, which classifies *every* `NaN` pattern, the anchor included.
//!
//! # Serde
//!
//! Float (de)serialization uses the primitive's own representation, so a round
//! trip reproduces the mathematical **value**, matching `f32`/`f64`. Exact bit
//! identity — a specific `NaN` payload, or the sign of a zero — survives only on
//! formats that preserve IEEE-754 bits (e.g. `bincode`). Under a format that
//! canonicalizes `NaN` or flushes signed zero, a bit-exact `NonValueF*` whose
//! forbidden pattern is such a value can even *fail* to deserialize, since the
//! checked constructor rejects the mangled bits. Serialize `to_bits()` yourself
//! if you need format-independent bit fidelity.

use crate::error::{ParseFloatError, TryFromFloatError};

// ============================ bit-exact family ============================

macro_rules! niche_float {
    ($nv:ident, $prim:ident, $bits:ident, $nonzero:ident, $nonmax:ident, $nonmin:ident) => {
        #[doc = concat!("An [`", stringify!($prim), "`] whose bit pattern is known not to equal `BITS`.")]
        ///
        /// `Option<Self>` is niche-optimized to the size of the primitive.
        /// Rejection is bit-exact (see the module docs). Because it can still
        /// hold `NaN`, it implements only [`PartialEq`]/[`PartialOrd`] (by
        /// value, matching the primitive) — not [`Eq`]/[`Ord`]/[`Hash`].
        #[derive(Clone, Copy)]
        #[repr(transparent)]
        pub struct $nv<const BITS: $bits>(core::num::$nonzero);

        impl<const BITS: $bits> $nv<BITS> {
            /// Creates a value if `value.to_bits() != BITS`, otherwise `None`.
            #[inline]
            pub const fn new(value: $prim) -> Option<Self> {
                match core::num::$nonzero::new(value.to_bits() ^ BITS) {
                    None => None,
                    Some(inner) => Some(Self(inner)),
                }
            }

            /// Creates a value without checking its bit pattern.
            ///
            /// # Safety
            ///
            /// `value.to_bits()` must not equal `BITS`.
            #[inline]
            pub const unsafe fn new_unchecked(value: $prim) -> Self {
                // SAFETY: caller guarantees `value.to_bits() != BITS`.
                Self(unsafe { core::num::$nonzero::new_unchecked(value.to_bits() ^ BITS) })
            }

            /// Returns the value as a primitive.
            #[inline]
            pub const fn get(&self) -> $prim {
                $prim::from_bits(self.0.get() ^ BITS)
            }
        }

        impl<const BITS: $bits> PartialEq for $nv<BITS> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.get() == other.get()
            }
        }
        impl<const BITS: $bits> PartialOrd for $nv<BITS> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }
        impl<const BITS: $bits> From<$nv<BITS>> for $prim {
            #[inline]
            fn from(value: $nv<BITS>) -> Self {
                value.get()
            }
        }
        impl<const BITS: $bits> core::convert::TryFrom<$prim> for $nv<BITS> {
            type Error = TryFromFloatError;
            #[inline]
            fn try_from(value: $prim) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(TryFromFloatError(()))
            }
        }
        impl<const BITS: $bits> core::str::FromStr for $nv<BITS> {
            type Err = ParseFloatError;
            #[inline]
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(<$prim as core::str::FromStr>::from_str(value)?).ok_or(ParseFloatError(()))
            }
        }

        impl<const BITS: $bits> core::fmt::Debug for $nv<BITS> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&self.get(), f)
            }
        }
        impl<const BITS: $bits> core::fmt::Display for $nv<BITS> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.get(), f)
            }
        }
        impl<const BITS: $bits> core::fmt::LowerExp for $nv<BITS> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerExp::fmt(&self.get(), f)
            }
        }
        impl<const BITS: $bits> core::fmt::UpperExp for $nv<BITS> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperExp::fmt(&self.get(), f)
            }
        }

        #[cfg(feature = "serde")]
        impl<const BITS: $bits> serde::Serialize for $nv<BITS> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.get().serialize(serializer)
            }
        }
        #[cfg(feature = "serde")]
        impl<'de, const BITS: $bits> serde::Deserialize<'de> for $nv<BITS> {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <$prim as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).ok_or_else(|| serde::de::Error::custom("bit pattern is forbidden by niche type"))
            }
        }

        #[doc = concat!("A [`", stringify!($prim), "`] known not to be `", stringify!($prim), "::MAX` (bit-exact).")]
        pub type $nonmax = $nv<{ $prim::MAX.to_bits() }>;
        #[doc = concat!("A [`", stringify!($prim), "`] known not to be `", stringify!($prim), "::MIN` (bit-exact).")]
        pub type $nonmin = $nv<{ $prim::MIN.to_bits() }>;

        const _: () = {
            assert!(core::mem::size_of::<$nonmax>() == core::mem::size_of::<$prim>());
            assert!(core::mem::size_of::<Option<$nonmax>>() == core::mem::size_of::<$prim>());
        };
    };
}

niche_float!(NonValueF32, f32, u32, NonZeroU32, NonMaxF32, NonMinF32);
niche_float!(NonValueF64, f64, u64, NonZeroU64, NonMaxF64, NonMinF64);

// ============================ class-based family ============================

macro_rules! niche_float_class {
    (
        $ty:ident, $prim:ident, $bits:ident, $nonzero:ident,
        anchor = $anchor:expr, reject = $reject:expr, what = $what:literal
    ) => {
        #[doc = concat!("An [`", stringify!($prim), "`] guaranteed not to be ", $what, ".")]
        ///
        /// `Option<Self>` is niche-optimized to the size of the primitive.
        #[derive(Clone, Copy)]
        #[repr(transparent)]
        pub struct $ty(core::num::$nonzero);

        impl $ty {
            /// The niche anchor: a representative bit pattern from the forbidden
            /// class. Never stored, so it is free to serve as `Option::None`.
            const ANCHOR: $bits = $anchor;

            #[doc = concat!("Creates a value if it is not ", $what, ", otherwise `None`.")]
            #[inline]
            pub fn new(value: $prim) -> Option<Self> {
                #[allow(clippy::redundant_closure_call)]
                if ($reject)(value) {
                    return None;
                }
                // The predicate guarantees `value.to_bits() != ANCHOR` (the
                // anchor is itself a member of the forbidden class), so the XOR
                // is never zero and `NonZero::new` always returns `Some`.
                core::num::$nonzero::new(value.to_bits() ^ Self::ANCHOR).map(Self)
            }

            #[doc = concat!("Creates a value without checking that it is not ", $what, ".")]
            ///
            /// # Safety
            #[doc = concat!("`value` must not be ", $what, ".")]
            #[inline]
            pub unsafe fn new_unchecked(value: $prim) -> Self {
                // SAFETY: caller guarantees the value is outside the forbidden
                // class, hence `value.to_bits() != ANCHOR`.
                Self(unsafe { core::num::$nonzero::new_unchecked(value.to_bits() ^ Self::ANCHOR) })
            }

            /// Returns the value as a primitive.
            #[inline]
            pub const fn get(&self) -> $prim {
                $prim::from_bits(self.0.get() ^ Self::ANCHOR)
            }
        }

        impl PartialEq for $ty {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.get() == other.get()
            }
        }
        impl From<$ty> for $prim {
            #[inline]
            fn from(value: $ty) -> Self {
                value.get()
            }
        }
        impl core::convert::TryFrom<$prim> for $ty {
            type Error = TryFromFloatError;
            #[inline]
            fn try_from(value: $prim) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(TryFromFloatError(()))
            }
        }
        impl core::str::FromStr for $ty {
            type Err = ParseFloatError;
            #[inline]
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(<$prim as core::str::FromStr>::from_str(value)?)
                    .ok_or(ParseFloatError(()))
            }
        }
        impl core::fmt::Debug for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&self.get(), f)
            }
        }
        impl core::fmt::Display for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.get(), f)
            }
        }
        impl core::fmt::LowerExp for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerExp::fmt(&self.get(), f)
            }
        }
        impl core::fmt::UpperExp for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperExp::fmt(&self.get(), f)
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $ty {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.get().serialize(serializer)
            }
        }
        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <$prim as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value)
                    .ok_or_else(|| serde::de::Error::custom(concat!("value is ", $what)))
            }
        }

        const _: () = {
            assert!(core::mem::size_of::<$ty>() == core::mem::size_of::<$prim>());
            assert!(core::mem::size_of::<Option<$ty>>() == core::mem::size_of::<$prim>());
        };
    };
}

/// `PartialOrd` for a class-based float that may still hold `NaN` (so no total
/// order): comparison is by value and can be `None`.
macro_rules! impl_partial_ord {
    ($ty:ident) => {
        impl PartialOrd for $ty {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }
    };
}

/// Adds total `Eq`/`Ord`/`Hash` (and a canonical `PartialOrd`) to a float type
/// that can never hold `NaN`. `-0.0` is normalized to `+0.0` in `Hash` to stay
/// consistent with `Eq` (`+0.0 == -0.0`), while `get()` round-trips the sign.
macro_rules! impl_total_ord {
    ($ty:ident, $prim:ident) => {
        impl PartialOrd for $ty {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Eq for $ty {}
        impl Ord for $ty {
            #[inline]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                // SAFETY-of-logic: this type never holds NaN, so partial_cmp is total.
                self.get()
                    .partial_cmp(&other.get())
                    .expect("type invariant guarantees no NaN")
            }
        }
        impl core::hash::Hash for $ty {
            #[inline]
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                let value = self.get();
                let normalized = if value == 0.0 { 0.0 } else { value };
                normalized.to_bits().hash(state);
            }
        }
    };
}

niche_float_class!(
    NonNanF32,
    f32,
    u32,
    NonZeroU32,
    anchor = 0x7FC0_0000,
    reject = |v: f32| v.is_nan(),
    what = "`NaN`"
);
niche_float_class!(
    NonNanF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x7FF8_0000_0000_0000,
    reject = |v: f64| v.is_nan(),
    what = "`NaN`"
);
impl_total_ord!(NonNanF32, f32);
impl_total_ord!(NonNanF64, f64);

niche_float_class!(
    NonInfF32,
    f32,
    u32,
    NonZeroU32,
    anchor = 0x7F80_0000,
    reject = |v: f32| v.is_infinite(),
    what = "infinite"
);
niche_float_class!(
    NonInfF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x7FF0_0000_0000_0000,
    reject = |v: f64| v.is_infinite(),
    what = "infinite"
);
impl_partial_ord!(NonInfF32);
impl_partial_ord!(NonInfF64);

niche_float_class!(
    NonZeroF32,
    f32,
    u32,
    NonZeroU32,
    anchor = 0x0000_0000,
    reject = |v: f32| v == 0.0,
    what = "zero"
);
niche_float_class!(
    NonZeroF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x0000_0000_0000_0000,
    reject = |v: f64| v == 0.0,
    what = "zero"
);
impl_partial_ord!(NonZeroF32);
impl_partial_ord!(NonZeroF64);

niche_float_class!(
    FiniteF32,
    f32,
    u32,
    NonZeroU32,
    anchor = 0x7FC0_0000,
    reject = |v: f32| !v.is_finite(),
    what = "non-finite (`NaN` or infinite)"
);
niche_float_class!(
    FiniteF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x7FF8_0000_0000_0000,
    reject = |v: f64| !v.is_finite(),
    what = "non-finite (`NaN` or infinite)"
);
impl_total_ord!(FiniteF32, f32);
impl_total_ord!(FiniteF64, f64);

niche_float_class!(
    NonSubnormalF32,
    f32,
    u32,
    NonZeroU32,
    anchor = 0x0000_0001,
    reject = |v: f32| v.is_subnormal(),
    what = "subnormal"
);
niche_float_class!(
    NonSubnormalF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x0000_0000_0000_0001,
    reject = |v: f64| v.is_subnormal(),
    what = "subnormal"
);
impl_partial_ord!(NonSubnormalF32);
impl_partial_ord!(NonSubnormalF64);

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    // Shared contract checks for the NaN-free float types: a total `Ord` and the
    // `-0.0`-normalized `Hash`. Both `NonNan*` and `Finite*` must satisfy them,
    // so exercising them through one helper stops a future contract change from
    // leaving either type under-tested.
    #[cfg(feature = "std")]
    fn assert_total_order_f64<T: Ord + Copy>(new: impl Fn(f64) -> T, get: impl Fn(&T) -> f64) {
        use std::collections::BTreeSet;
        let a = new(-1.0);
        let b = new(0.0);
        let c = new(2.5);
        assert!(a < b && b < c);

        let mut set: BTreeSet<T> = BTreeSet::new();
        set.insert(c);
        set.insert(a);
        set.insert(b);
        let sorted: Vec<f64> = set.iter().map(get).collect();
        assert_eq!(sorted, vec![-1.0, 0.0, 2.5]);
    }

    #[cfg(feature = "std")]
    fn assert_signed_zero_eq_hash_f32<T: Eq + core::hash::Hash + Copy + core::fmt::Debug>(
        new: impl Fn(f32) -> T,
    ) {
        use std::collections::HashSet;
        // +0.0 and -0.0 compare and hash equal, so -0.0 must collide in the set.
        let mut hs: HashSet<T> = HashSet::new();
        assert!(hs.insert(new(0.0)));
        assert!(!hs.insert(new(-0.0)));
        assert_eq!(new(0.0), new(-0.0));
    }

    #[test]
    fn bit_exact_rejection_and_roundtrip() {
        assert_eq!(NonMaxF32::new(1.5).unwrap().get(), 1.5);
        assert!(NonMaxF32::new(f32::MAX).is_none());
        assert!(NonMinF32::new(f32::MIN).is_none());
        assert_eq!(NonMaxF32::new(f32::INFINITY).unwrap().get(), f32::INFINITY);
    }

    #[test]
    fn signed_zero_is_bit_distinct() {
        // Forbid +0.0's bit pattern; -0.0 is a *different* pattern and allowed.
        type NoPosZero = NonValueF32<0x0000_0000>;
        assert!(NoPosZero::new(0.0).is_none());
        let neg = NoPosZero::new(-0.0).unwrap();
        assert!(neg.get().is_sign_negative());
        assert_eq!(neg.get(), 0.0); // value-equal to +0.0
    }

    #[test]
    fn nonnan_rejects_every_nan_but_keeps_infinity() {
        assert!(NonNanF32::new(f32::NAN).is_none());
        // a different (signaling-ish) NaN bit pattern is also rejected
        assert!(NonNanF32::new(f32::from_bits(0x7F80_0001)).is_none());
        assert!(NonNanF32::new(f32::from_bits(0xFFFF_FFFF)).is_none());
        // infinities are NOT NaN, so allowed
        assert_eq!(NonNanF32::new(f32::INFINITY).unwrap().get(), f32::INFINITY);
        assert_eq!(NonNanF32::new(-2.5).unwrap().get(), -2.5);
    }

    #[test]
    fn noninf_rejects_both_infinities_but_keeps_nan() {
        assert!(NonInfF32::new(f32::INFINITY).is_none());
        assert!(NonInfF32::new(f32::NEG_INFINITY).is_none());
        // NaN is not infinite, so allowed (and thus NonInf is NOT Eq/Ord)
        assert!(NonInfF64::new(f64::NAN).unwrap().get().is_nan());
        assert_eq!(NonInfF32::new(3.0).unwrap().get(), 3.0);
    }

    #[test]
    #[cfg(feature = "std")] // ordered/hashed containers
    fn nonnan_is_totally_ordered_and_hashable() {
        assert_total_order_f64(|v| NonNanF64::new(v).unwrap(), |x| x.get());
        assert_signed_zero_eq_hash_f32(|v| NonNanF32::new(v).unwrap());
    }

    #[test]
    fn nonnan_signed_zero_roundtrips_but_compares_equal() {
        let pos = NonNanF32::new(0.0).unwrap();
        let neg = NonNanF32::new(-0.0).unwrap();
        assert_eq!(pos, neg); // value equality
        assert_eq!(pos.cmp(&neg), core::cmp::Ordering::Equal);
        // but get() is lossless on the sign bit
        assert!(pos.get().is_sign_positive());
        assert!(neg.get().is_sign_negative());
    }

    #[test]
    fn nonzero_rejects_both_signed_zeros_but_keeps_nan() {
        // Both +0.0 and -0.0 are rejected as a *class* (unlike bit-exact
        // NonValueF32<0x0000_0000>, which forbids only +0.0).
        assert!(NonZeroF32::new(0.0).is_none());
        assert!(NonZeroF32::new(-0.0).is_none());
        assert!(NonZeroF64::new(0.0).is_none());
        assert!(NonZeroF64::new(-0.0).is_none());
        // nonzero values (including NaN and infinities) round-trip
        assert_eq!(NonZeroF32::new(1.5).unwrap().get(), 1.5);
        assert_eq!(NonZeroF32::new(-2.0).unwrap().get(), -2.0);
        assert_eq!(
            NonZeroF32::new(f32::MIN_POSITIVE).unwrap().get(),
            f32::MIN_POSITIVE
        );
        assert!(NonZeroF64::new(f64::NAN).unwrap().get().is_nan());
        assert_eq!(NonZeroF32::new(f32::INFINITY).unwrap().get(), f32::INFINITY);
        assert_eq!(size_of::<Option<NonZeroF32>>(), size_of::<f32>());
        assert_eq!(size_of::<Option<NonZeroF64>>(), size_of::<f64>());
    }

    #[test]
    fn finite_rejects_nan_and_both_infinities() {
        // Finite = NonNan ∩ NonInf: rejects the whole non-finite class.
        assert!(FiniteF32::new(f32::NAN).is_none());
        assert!(FiniteF32::new(f32::from_bits(0x7F80_0001)).is_none()); // another NaN
        assert!(FiniteF32::new(f32::INFINITY).is_none());
        assert!(FiniteF32::new(f32::NEG_INFINITY).is_none());
        assert!(FiniteF64::new(f64::NAN).is_none());
        assert!(FiniteF64::new(f64::INFINITY).is_none());
        assert!(FiniteF64::new(f64::NEG_INFINITY).is_none());
        // finite values (including subnormals and zeros) round-trip
        assert_eq!(FiniteF32::new(1.5).unwrap().get(), 1.5);
        assert_eq!(FiniteF64::new(-2.5).unwrap().get(), -2.5);
        assert_eq!(FiniteF32::new(0.0).unwrap().get(), 0.0);
        assert_eq!(size_of::<Option<FiniteF32>>(), size_of::<f32>());
        assert_eq!(size_of::<Option<FiniteF64>>(), size_of::<f64>());
    }

    #[test]
    #[cfg(feature = "std")] // ordered/hashed containers
    fn finite_is_totally_ordered_and_hashable() {
        assert_total_order_f64(|v| FiniteF64::new(v).unwrap(), |x| x.get());
        assert_signed_zero_eq_hash_f32(|v| FiniteF32::new(v).unwrap());
        // +0.0 and -0.0 compare and hash equal, but get() keeps the sign.
        assert!(FiniteF32::new(-0.0).unwrap().get().is_sign_negative());
    }

    #[test]
    fn nonsubnormal_rejects_subnormals_but_keeps_nan_zero_and_normals() {
        // Smallest positive subnormal and a mid-range subnormal are rejected.
        assert!(NonSubnormalF32::new(f32::from_bits(0x0000_0001)).is_none());
        assert!(NonSubnormalF32::new(f32::from_bits(0x0040_0000)).is_none()); // larger subnormal
        assert!(NonSubnormalF64::new(f64::from_bits(0x0000_0000_0000_0001)).is_none());
        // Negative subnormals (sign bit set) are subnormal too, so also rejected.
        assert!(NonSubnormalF32::new(f32::from_bits(0x8000_0001)).is_none());
        assert!(NonSubnormalF64::new(f64::from_bits(0x8000_0000_0000_0001)).is_none());
        // -0.0/+0.0 are NOT subnormal, so allowed; NaN allowed; normals allowed.
        assert_eq!(NonSubnormalF32::new(0.0).unwrap().get(), 0.0);
        assert!(NonSubnormalF32::new(-0.0).unwrap().get().is_sign_negative());
        assert!(NonSubnormalF64::new(f64::NAN).unwrap().get().is_nan());
        assert_eq!(
            NonSubnormalF32::new(f32::MIN_POSITIVE).unwrap().get(),
            f32::MIN_POSITIVE
        );
        assert_eq!(NonSubnormalF64::new(1.5).unwrap().get(), 1.5);
        assert_eq!(size_of::<Option<NonSubnormalF32>>(), size_of::<f32>());
        assert_eq!(size_of::<Option<NonSubnormalF64>>(), size_of::<f64>());
    }

    #[test]
    fn sizes_niche_optimized() {
        assert_eq!(size_of::<Option<NonValueF32<7>>>(), size_of::<f32>());
        assert_eq!(size_of::<Option<NonMaxF64>>(), size_of::<f64>());
        assert_eq!(size_of::<Option<NonNanF32>>(), size_of::<f32>());
        assert_eq!(size_of::<Option<NonNanF64>>(), size_of::<f64>());
        assert_eq!(size_of::<Option<NonInfF32>>(), size_of::<f32>());
    }

    #[test]
    #[cfg(feature = "std")] // uses `format!`
    fn fmt_and_parse() {
        let v = NonNanF64::new(1.5).unwrap();
        assert_eq!(format!("{v}"), "1.5");
        assert_eq!(format!("{v:e}"), format!("{:e}", 1.5f64));
        let p: NonNanF32 = "2.5".parse().unwrap();
        assert_eq!(p.get(), 2.5);
        assert!("NaN".parse::<NonNanF32>().is_err());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip() {
        let v = NonNanF64::new(1.25).unwrap();
        let bytes = bincode::serialize(&v).unwrap();
        let back: NonNanF64 = bincode::deserialize(&bytes).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_signed_zero_and_nan_payload() {
        // bincode preserves IEEE-754 bits, so the value-based serde impls keep
        // signed zero and specific NaN payloads exact across a round trip.
        let neg_zero = NonValueF32::<0x0000_0000>::new(-0.0).unwrap();
        let bytes = bincode::serialize(&neg_zero).unwrap();
        let back: NonValueF32<0x0000_0000> = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.get().to_bits(), 0x8000_0000);
        assert!(back.get().is_sign_negative());

        // A payloaded NaN (not `f32::MAX`) survives a bit-preserving round trip.
        let payload_nan = NonMaxF32::new(f32::from_bits(0x7F80_0001)).unwrap();
        let bytes = bincode::serialize(&payload_nan).unwrap();
        let back: NonMaxF32 = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.get().to_bits(), 0x7F80_0001);
    }
}
