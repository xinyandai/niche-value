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

        forward_conversions!([const BITS: $bits] $nv<BITS>, $prim, TryFromFloatError, ParseFloatError);
        forward_fmt!([const BITS: $bits] $nv<BITS> => Debug, Display, LowerExp, UpperExp);
        forward_serde!([const BITS: $bits] $nv<BITS>, $prim, "bit pattern is forbidden by niche type");

        #[doc = concat!("A [`", stringify!($prim), "`] known not to be `", stringify!($prim), "::MAX` (bit-exact).")]
        pub type $nonmax = $nv<{ $prim::MAX.to_bits() }>;
        #[doc = concat!("A [`", stringify!($prim), "`] known not to be `", stringify!($prim), "::MIN` (bit-exact).")]
        pub type $nonmin = $nv<{ $prim::MIN.to_bits() }>;

        assert_niche_layout!($nonmax, $prim);
    };
}

niche_float!(NonValueF32, f32, u32, NonZeroU32, NonMaxF32, NonMinF32);
niche_float!(NonValueF64, f64, u64, NonZeroU64, NonMaxF64, NonMinF64);

// ============================ class-based family ============================

/// Defines one class-based float type. `reject = |v| …` is spliced into the
/// constructor body (not called as a closure) so that `new` can be `const fn`.
macro_rules! niche_float_class {
    (
        $ty:ident, $prim:ident, $bits:ident, $nonzero:ident,
        anchor = $anchor:expr, reject = |$v:ident| $reject:expr, what = $what:literal
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
            pub const fn new(value: $prim) -> Option<Self> {
                let $v = value;
                if $reject {
                    return None;
                }
                // The predicate guarantees `value.to_bits() != ANCHOR` (the
                // anchor is itself a member of the forbidden class), so the XOR
                // is never zero and `NonZero::new` always returns `Some`.
                match core::num::$nonzero::new(value.to_bits() ^ Self::ANCHOR) {
                    None => None,
                    Some(inner) => Some(Self(inner)),
                }
            }

            #[doc = concat!("Creates a value without checking that it is not ", $what, ".")]
            ///
            /// # Safety
            #[doc = concat!("`value` must not be ", $what, ".")]
            #[inline]
            pub const unsafe fn new_unchecked(value: $prim) -> Self {
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

        forward_conversions!([] $ty, $prim, TryFromFloatError, ParseFloatError);
        forward_fmt!([] $ty => Debug, Display, LowerExp, UpperExp);
        forward_serde!([] $ty, $prim, concat!("value is ", $what));

        assert_niche_layout!($ty, $prim);
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
                // Without NaN, `<`/`>` are exhaustive: exactly one of `<`, `>`,
                // `==` holds, so this is a total order with no panic path.
                // (`f32::total_cmp` is deliberately not used: it distinguishes
                // `-0.0` from `+0.0`, which would disagree with `PartialEq`.)
                let (a, b) = (self.get(), other.get());
                if a < b {
                    core::cmp::Ordering::Less
                } else if a > b {
                    core::cmp::Ordering::Greater
                } else {
                    core::cmp::Ordering::Equal
                }
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
    reject = |v| v.is_nan(),
    what = "`NaN`"
);
niche_float_class!(
    NonNanF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x7FF8_0000_0000_0000,
    reject = |v| v.is_nan(),
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
    reject = |v| v.is_infinite(),
    what = "infinite"
);
niche_float_class!(
    NonInfF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x7FF0_0000_0000_0000,
    reject = |v| v.is_infinite(),
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
    reject = |v| v == 0.0,
    what = "zero"
);
niche_float_class!(
    NonZeroF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x0000_0000_0000_0000,
    reject = |v| v == 0.0,
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
    reject = |v| !v.is_finite(),
    what = "non-finite (`NaN` or infinite)"
);
niche_float_class!(
    FiniteF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x7FF8_0000_0000_0000,
    reject = |v| !v.is_finite(),
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
    reject = |v| v.is_subnormal(),
    what = "subnormal"
);
niche_float_class!(
    NonSubnormalF64,
    f64,
    u64,
    NonZeroU64,
    anchor = 0x0000_0000_0000_0001,
    reject = |v| v.is_subnormal(),
    what = "subnormal"
);
impl_partial_ord!(NonSubnormalF32);
impl_partial_ord!(NonSubnormalF64);

// Every anchor must be a member of the class its type rejects; otherwise the
// anchor would be constructible and `new` could build a `NonZero(0)`.
const _: () = {
    assert!(f32::from_bits(NonNanF32::ANCHOR).is_nan());
    assert!(f64::from_bits(NonNanF64::ANCHOR).is_nan());
    assert!(f32::from_bits(NonInfF32::ANCHOR).is_infinite());
    assert!(f64::from_bits(NonInfF64::ANCHOR).is_infinite());
    assert!(f32::from_bits(NonZeroF32::ANCHOR) == 0.0);
    assert!(f64::from_bits(NonZeroF64::ANCHOR) == 0.0);
    assert!(!f32::from_bits(FiniteF32::ANCHOR).is_finite());
    assert!(!f64::from_bits(FiniteF64::ANCHOR).is_finite());
    assert!(f32::from_bits(NonSubnormalF32::ANCHOR).is_subnormal());
    assert!(f64::from_bits(NonSubnormalF64::ANCHOR).is_subnormal());
};

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;
    use std::collections::{BTreeSet, HashSet};
    use std::{format, vec, vec::Vec};

    // Shared contract checks for the NaN-free float types: a total `Ord` and the
    // `-0.0`-normalized `Hash`. Both `NonNan*` and `Finite*` must satisfy them,
    // so exercising them through one helper stops a future contract change from
    // leaving either type under-tested.
    fn assert_total_order_f64<T: Ord + Copy>(new: impl Fn(f64) -> T, get: impl Fn(&T) -> f64) {
        let a = new(-1.0);
        let b = new(0.0);
        let c = new(2.5);
        assert!(a < b && b < c);
        assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal);
        assert_eq!(c.cmp(&a), core::cmp::Ordering::Greater);

        let mut set: BTreeSet<T> = BTreeSet::new();
        set.insert(c);
        set.insert(a);
        set.insert(b);
        let sorted: Vec<f64> = set.iter().map(get).collect();
        assert_eq!(sorted, vec![-1.0, 0.0, 2.5]);
    }

    fn assert_signed_zero_eq_hash_f32<T: Eq + core::hash::Hash + Copy + core::fmt::Debug>(
        new: impl Fn(f32) -> T,
    ) {
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
        assert!(NonMaxF64::new(f64::MAX).is_none());
        assert!(NonMinF64::new(f64::MIN).is_none());
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
        assert!(NonNanF64::new(f64::NAN).is_none());
        assert!(NonNanF64::new(f64::from_bits(0xFFFF_FFFF_FFFF_FFFF)).is_none());
        // infinities are NOT NaN, so allowed
        assert_eq!(NonNanF32::new(f32::INFINITY).unwrap().get(), f32::INFINITY);
        assert_eq!(NonNanF32::new(-2.5).unwrap().get(), -2.5);
    }

    #[test]
    fn noninf_rejects_both_infinities_but_keeps_nan() {
        assert!(NonInfF32::new(f32::INFINITY).is_none());
        assert!(NonInfF32::new(f32::NEG_INFINITY).is_none());
        assert!(NonInfF64::new(f64::INFINITY).is_none());
        assert!(NonInfF64::new(f64::NEG_INFINITY).is_none());
        // NaN is not infinite, so allowed (and thus NonInf is NOT Eq/Ord)
        assert!(NonInfF64::new(f64::NAN).unwrap().get().is_nan());
        assert_eq!(NonInfF32::new(3.0).unwrap().get(), 3.0);
    }

    #[test]
    fn nonnan_is_totally_ordered_and_hashable() {
        assert_total_order_f64(|v| NonNanF64::new(v).unwrap(), |x| x.get());
        assert_signed_zero_eq_hash_f32(|v| NonNanF32::new(v).unwrap());
        // Infinities are allowed and sit at the ends of the order.
        let lo = NonNanF64::new(f64::NEG_INFINITY).unwrap();
        let hi = NonNanF64::new(f64::INFINITY).unwrap();
        assert!(lo < NonNanF64::new(f64::MIN).unwrap());
        assert!(hi > NonNanF64::new(f64::MAX).unwrap());
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
        // the smallest subnormal is nonzero and must round-trip bit-exactly
        assert_eq!(
            NonZeroF32::new(f32::from_bits(1)).unwrap().get().to_bits(),
            1
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
        assert_eq!(FiniteF32::new(f32::MAX).unwrap().get(), f32::MAX);
        assert_eq!(size_of::<Option<FiniteF32>>(), size_of::<f32>());
        assert_eq!(size_of::<Option<FiniteF64>>(), size_of::<f64>());
    }

    #[test]
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
    fn const_context() {
        const V: NonNanF32 = match NonNanF32::new(2.5) {
            Some(v) => v,
            None => panic!(),
        };
        const G: f32 = V.get();
        assert_eq!(G, 2.5);
        const REJECTED: Option<FiniteF64> = FiniteF64::new(f64::INFINITY);
        assert!(REJECTED.is_none());
        const BIT_EXACT: Option<NonMaxF32> = NonMaxF32::new(f32::MAX);
        assert!(BIT_EXACT.is_none());
    }

    #[test]
    fn fmt_and_parse() {
        let v = NonNanF64::new(1.5).unwrap();
        assert_eq!(format!("{v}"), "1.5");
        assert_eq!(format!("{v:?}"), "1.5");
        assert_eq!(format!("{v:e}"), format!("{:e}", 1.5f64));
        assert_eq!(format!("{v:E}"), format!("{:E}", 1.5f64));
        let p: NonNanF32 = "2.5".parse().unwrap();
        assert_eq!(p.get(), 2.5);
        assert!("NaN".parse::<NonNanF32>().is_err());
        assert!("inf".parse::<FiniteF32>().is_err());
        assert!("abc".parse::<NonMaxF32>().is_err());
        let b: NonMaxF32 = "1e3".parse().unwrap();
        assert_eq!(b.get(), 1000.0);
    }

    #[test]
    fn conversions() {
        use core::convert::TryFrom;
        assert_eq!(f32::from(NonNanF32::new(1.5).unwrap()), 1.5);
        assert_eq!(NonNanF32::try_from(1.5).unwrap().get(), 1.5);
        NonNanF32::try_from(f32::NAN).unwrap_err();
        NonMaxF64::try_from(f64::MAX).unwrap_err();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip() {
        let v = NonNanF64::new(1.25).unwrap();
        let bytes = bincode::serialize(&v).unwrap();
        let back: NonNanF64 = bincode::deserialize(&bytes).unwrap();
        assert_eq!(v, back);
        // a forbidden value fails to deserialize
        let bad = bincode::serialize(&f64::NAN).unwrap();
        assert!(bincode::deserialize::<NonNanF64>(&bad).is_err());
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
