//! Integer niche types: [`NonValueU8`]…[`NonValueIsize`] and their
//! `NonMax*` / `NonMin*` specializations.

use crate::error::{ParseIntError, TryFromIntError};

/// Generate the six standard integer formatting impls, forwarding to `get()`.
macro_rules! int_fmt {
    ($nv:ident, $prim:ident, $($Trait:ident),+ $(,)?) => {
        $(
            impl<const N: $prim> core::fmt::$Trait for $nv<N> {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::$Trait::fmt(&self.get(), f)
                }
            }
        )+
    };
}

macro_rules! niche_int {
    (@common $nv:ident, $nonzero:ident, $prim:ident, $nonmax:ident, $nonmin:ident) => {
        #[doc = concat!("An [`", stringify!($prim), "`] that is known not to equal the const value `N`.")]
        ///
        /// `Option<Self>` is the same size as the primitive thanks to niche
        /// optimization. Construct with [`new`](Self::new).
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $nv<const N: $prim>(core::num::$nonzero);

        impl<const N: $prim> $nv<N> {
            /// Creates a niche integer if `value != N`, otherwise `None`.
            #[inline]
            pub const fn new(value: $prim) -> Option<Self> {
                match core::num::$nonzero::new(value ^ N) {
                    None => None,
                    Some(inner) => Some(Self(inner)),
                }
            }

            /// Creates a niche integer without checking `value`.
            ///
            /// # Safety
            ///
            /// `value` must not equal `N`.
            #[inline]
            pub const unsafe fn new_unchecked(value: $prim) -> Self {
                // SAFETY: the caller guarantees `value != N`, so `value ^ N != 0`.
                Self(unsafe { core::num::$nonzero::new_unchecked(value ^ N) })
            }

            /// Returns the value as a primitive.
            #[inline]
            pub const fn get(&self) -> $prim {
                self.0.get() ^ N
            }
        }

        impl<const N: $prim> core::cmp::Ord for $nv<N> {
            #[inline]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }
        impl<const N: $prim> core::cmp::PartialOrd for $nv<N> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<const N: $prim> From<$nv<N>> for $prim {
            #[inline]
            fn from(value: $nv<N>) -> Self {
                value.get()
            }
        }

        impl<const N: $prim> core::convert::TryFrom<$prim> for $nv<N> {
            type Error = TryFromIntError;
            #[inline]
            fn try_from(value: $prim) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(TryFromIntError(()))
            }
        }

        impl<const N: $prim> core::str::FromStr for $nv<N> {
            type Err = ParseIntError;
            #[inline]
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(<$prim as core::str::FromStr>::from_str(value)?).ok_or(ParseIntError(()))
            }
        }

        int_fmt!($nv, $prim, Debug, Display, Binary, Octal, LowerHex, UpperHex);

        #[cfg(feature = "serde")]
        impl<const N: $prim> serde::Serialize for $nv<N> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.get().serialize(serializer)
            }
        }
        #[cfg(feature = "serde")]
        impl<'de, const N: $prim> serde::Deserialize<'de> for $nv<N> {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <$prim as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).ok_or_else(|| serde::de::Error::custom("value is forbidden by niche type"))
            }
        }

        #[doc = concat!("A [`", stringify!($prim), "`] that is known not to equal its maximum. Drop-in for `nonmax::", stringify!($nonmax), "`.")]
        pub type $nonmax = $nv<{ $prim::MAX }>;

        #[doc = concat!("A [`", stringify!($prim), "`] that is known not to equal its minimum.")]
        pub type $nonmin = $nv<{ $prim::MIN }>;

        // ---- `MAX`-specific surface (nonmax parity). Attached to the concrete
        // ---- `<{MAX}>` instantiation so only the `NonMax*` alias sees it. ----
        impl $nv<{ $prim::MAX }> {
            /// The value `0`.
            pub const ZERO: Self = unsafe { Self::new_unchecked(0) };
            /// The value `1`.
            pub const ONE: Self = unsafe { Self::new_unchecked(1) };
            /// The largest representable value (`primitive::MAX - 1`).
            pub const MAX: Self = unsafe { Self::new_unchecked($prim::MAX - 1) };
        }

        impl Default for $nv<{ $prim::MAX }> {
            #[inline]
            fn default() -> Self {
                // SAFETY: 0 != MAX for every width.
                unsafe { Self::new_unchecked(0) }
            }
        }

        // `NonMax & NonMax` is closed: MAX is the only value whose low bits are
        // all ones, and AND cannot fabricate it from two non-MAX operands.
        impl core::ops::BitAnd for $nv<{ $prim::MAX }> {
            type Output = Self;
            #[inline]
            fn bitand(self, rhs: Self) -> Self {
                // SAFETY: result of AND of two non-MAX values is never MAX.
                unsafe { Self::new_unchecked(self.get() & rhs.get()) }
            }
        }
        impl core::ops::BitAndAssign for $nv<{ $prim::MAX }> {
            #[inline]
            fn bitand_assign(&mut self, rhs: Self) {
                *self = *self & rhs;
            }
        }

        const _: () = {
            assert!(core::mem::size_of::<$nonmax>() == core::mem::size_of::<$prim>());
            assert!(core::mem::size_of::<Option<$nonmax>>() == core::mem::size_of::<$prim>());
        };
    };

    (signed $nv:ident, $nonzero:ident, $prim:ident, $nonmax:ident, $nonmin:ident) => {
        niche_int!(@common $nv, $nonzero, $prim, $nonmax, $nonmin);
    };

    (unsigned $nv:ident, $nonzero:ident, $prim:ident, $nonmax:ident, $nonmin:ident) => {
        niche_int!(@common $nv, $nonzero, $prim, $nonmax, $nonmin);

        // For unsigned only, one operand may be a raw primitive: ANDing a
        // non-MAX (which has a zero bit) with anything keeps that zero bit.
        impl core::ops::BitAnd<$prim> for $nv<{ $prim::MAX }> {
            type Output = Self;
            #[inline]
            fn bitand(self, rhs: $prim) -> Self {
                // SAFETY: self is non-MAX (has a 0 bit); AND preserves it.
                unsafe { Self::new_unchecked(self.get() & rhs) }
            }
        }
        impl core::ops::BitAnd<$nv<{ $prim::MAX }>> for $prim {
            type Output = $nv<{ $prim::MAX }>;
            #[inline]
            fn bitand(self, rhs: $nv<{ $prim::MAX }>) -> Self::Output {
                // SAFETY: rhs is non-MAX (has a 0 bit); AND preserves it.
                unsafe { <$nv<{ $prim::MAX }>>::new_unchecked(self & rhs.get()) }
            }
        }
        impl core::ops::BitAndAssign<$prim> for $nv<{ $prim::MAX }> {
            #[inline]
            fn bitand_assign(&mut self, rhs: $prim) {
                *self = *self & rhs;
            }
        }
        impl core::ops::BitAndAssign<$nv<{ $prim::MAX }>> for $prim {
            #[inline]
            fn bitand_assign(&mut self, rhs: $nv<{ $prim::MAX }>) {
                *self = *self & rhs.get();
            }
        }

        // Interop with `core::num::NonZero*`: for unsigned types, forbidding the
        // minimum (0) is exactly "non-zero", so `NonMin* == NonValue*<0>` and
        // the standard library's `NonZero*` bridge both ways, infallibly.
        impl From<core::num::$nonzero> for $nv<0> {
            #[inline]
            fn from(value: core::num::$nonzero) -> Self {
                // A non-zero value is a valid `NonValue<0>`; the inner encoding
                // is `value ^ 0 == value`.
                Self(value)
            }
        }
        impl From<$nv<0>> for core::num::$nonzero {
            #[inline]
            fn from(value: $nv<0>) -> Self {
                value.0
            }
        }
    };
}

niche_int!(signed NonValueI8, NonZeroI8, i8, NonMaxI8, NonMinI8);
niche_int!(signed NonValueI16, NonZeroI16, i16, NonMaxI16, NonMinI16);
niche_int!(signed NonValueI32, NonZeroI32, i32, NonMaxI32, NonMinI32);
niche_int!(signed NonValueI64, NonZeroI64, i64, NonMaxI64, NonMinI64);
niche_int!(signed NonValueI128, NonZeroI128, i128, NonMaxI128, NonMinI128);
niche_int!(signed NonValueIsize, NonZeroIsize, isize, NonMaxIsize, NonMinIsize);

niche_int!(unsigned NonValueU8, NonZeroU8, u8, NonMaxU8, NonMinU8);
niche_int!(unsigned NonValueU16, NonZeroU16, u16, NonMaxU16, NonMinU16);
niche_int!(unsigned NonValueU32, NonZeroU32, u32, NonMaxU32, NonMinU32);
niche_int!(unsigned NonValueU64, NonZeroU64, u64, NonMaxU64, NonMinU64);
niche_int!(unsigned NonValueU128, NonZeroU128, u128, NonMaxU128, NonMinU128);
niche_int!(unsigned NonValueUsize, NonZeroUsize, usize, NonMaxUsize, NonMinUsize);

// ---- Widening `From` conversions between `NonMax*` types (nonmax parity) ----
macro_rules! widen_niche {
    ($small:ty, $large:ty) => {
        impl From<$small> for $large {
            #[inline]
            fn from(small: $small) -> Self {
                // SAFETY: the smaller type's non-max value widens to a value
                // that cannot be the larger type's max.
                unsafe { Self::new_unchecked(small.get().into()) }
            }
        }
    };
}
macro_rules! widen_prim {
    ($small:ty, $large:ty) => {
        impl From<$small> for $large {
            #[inline]
            fn from(small: $small) -> Self {
                // SAFETY: a smaller primitive widens to a value that cannot be
                // the larger type's max.
                unsafe { Self::new_unchecked(small.into()) }
            }
        }
    };
}

// NonMax unsigned -> NonMax unsigned
widen_niche!(NonMaxU8, NonMaxU16);
widen_niche!(NonMaxU8, NonMaxU32);
widen_niche!(NonMaxU8, NonMaxU64);
widen_niche!(NonMaxU8, NonMaxU128);
widen_niche!(NonMaxU8, NonMaxUsize);
widen_niche!(NonMaxU16, NonMaxU32);
widen_niche!(NonMaxU16, NonMaxU64);
widen_niche!(NonMaxU16, NonMaxU128);
widen_niche!(NonMaxU16, NonMaxUsize);
widen_niche!(NonMaxU32, NonMaxU64);
widen_niche!(NonMaxU32, NonMaxU128);
widen_niche!(NonMaxU64, NonMaxU128);

// NonMax signed -> NonMax signed
widen_niche!(NonMaxI8, NonMaxI16);
widen_niche!(NonMaxI8, NonMaxI32);
widen_niche!(NonMaxI8, NonMaxI64);
widen_niche!(NonMaxI8, NonMaxI128);
widen_niche!(NonMaxI8, NonMaxIsize);
widen_niche!(NonMaxI16, NonMaxI32);
widen_niche!(NonMaxI16, NonMaxI64);
widen_niche!(NonMaxI16, NonMaxI128);
widen_niche!(NonMaxI16, NonMaxIsize);
widen_niche!(NonMaxI32, NonMaxI64);
widen_niche!(NonMaxI32, NonMaxI128);
widen_niche!(NonMaxI64, NonMaxI128);

// NonMax unsigned -> NonMax signed
widen_niche!(NonMaxU8, NonMaxI16);
widen_niche!(NonMaxU8, NonMaxI32);
widen_niche!(NonMaxU8, NonMaxI64);
widen_niche!(NonMaxU8, NonMaxI128);
widen_niche!(NonMaxU8, NonMaxIsize);
widen_niche!(NonMaxU16, NonMaxI32);
widen_niche!(NonMaxU16, NonMaxI64);
widen_niche!(NonMaxU16, NonMaxI128);
widen_niche!(NonMaxU32, NonMaxI64);
widen_niche!(NonMaxU32, NonMaxI128);
widen_niche!(NonMaxU64, NonMaxI128);

// primitive -> NonMax (unsigned)
widen_prim!(u8, NonMaxU16);
widen_prim!(u8, NonMaxU32);
widen_prim!(u8, NonMaxU64);
widen_prim!(u8, NonMaxU128);
widen_prim!(u8, NonMaxUsize);
widen_prim!(u16, NonMaxU32);
widen_prim!(u16, NonMaxU64);
widen_prim!(u16, NonMaxU128);
widen_prim!(u16, NonMaxUsize);
widen_prim!(u32, NonMaxU64);
widen_prim!(u32, NonMaxU128);
widen_prim!(u64, NonMaxU128);

// primitive -> NonMax (signed)
widen_prim!(i8, NonMaxI16);
widen_prim!(i8, NonMaxI32);
widen_prim!(i8, NonMaxI64);
widen_prim!(i8, NonMaxI128);
widen_prim!(i8, NonMaxIsize);
widen_prim!(i16, NonMaxI32);
widen_prim!(i16, NonMaxI64);
widen_prim!(i16, NonMaxI128);
widen_prim!(i16, NonMaxIsize);
widen_prim!(i32, NonMaxI64);
widen_prim!(i32, NonMaxI128);
widen_prim!(i64, NonMaxI128);

// primitive unsigned -> NonMax signed
widen_prim!(u8, NonMaxI16);
widen_prim!(u8, NonMaxI32);
widen_prim!(u8, NonMaxI64);
widen_prim!(u8, NonMaxI128);
widen_prim!(u8, NonMaxIsize);
widen_prim!(u16, NonMaxI32);
widen_prim!(u16, NonMaxI64);
widen_prim!(u16, NonMaxI128);
widen_prim!(u32, NonMaxI64);
widen_prim!(u32, NonMaxI128);
widen_prim!(u64, NonMaxI128);

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn generic_forbidden_value() {
        // Forbid an arbitrary value in the middle of the range.
        let v = NonValueU8::<7>::new(3).unwrap();
        assert_eq!(v.get(), 3);
        assert!(NonValueU8::<7>::new(7).is_none());
        assert_eq!(NonValueU8::<7>::new(0).unwrap().get(), 0);
        assert_eq!(NonValueU8::<7>::new(255).unwrap().get(), 255);
    }

    #[test]
    fn exhaustive_u8_all_forbidden() {
        // For every forbidden value N, exactly N is rejected and all else round-trips.
        macro_rules! check_n {
            ($n:literal) => {{
                for value in 0..=u8::MAX {
                    match NonValueU8::<$n>::new(value) {
                        None => assert_eq!(value, $n),
                        Some(v) => {
                            assert_ne!(value, $n);
                            assert_eq!(v.get(), value);
                        }
                    }
                }
            }};
        }
        check_n!(0);
        check_n!(1);
        check_n!(42);
        check_n!(128);
        check_n!(254);
        check_n!(255);
    }

    #[test]
    fn exhaustive_i8_all_forbidden() {
        macro_rules! check_n {
            ($n:literal) => {{
                for value in i8::MIN..=i8::MAX {
                    match NonValueI8::<$n>::new(value) {
                        None => assert_eq!(value, $n),
                        Some(v) => assert_eq!(v.get(), value),
                    }
                }
            }};
        }
        check_n!(-128);
        check_n!(-1);
        check_n!(0);
        check_n!(1);
        check_n!(127);
    }

    #[test]
    fn nonmax_nonmin_aliases() {
        assert!(NonMaxU8::new(u8::MAX).is_none());
        assert_eq!(NonMaxU8::new(0).unwrap().get(), 0);
        assert!(NonMinU8::new(u8::MIN).is_none());
        assert_eq!(NonMinU8::new(255).unwrap().get(), 255);

        assert!(NonMaxI8::new(i8::MAX).is_none());
        assert!(NonMinI8::new(i8::MIN).is_none());
        assert_eq!(NonMinI8::new(-127).unwrap().get(), -127);
    }

    #[test]
    fn nonmax_constants_and_default() {
        assert_eq!(NonMaxU8::ZERO.get(), 0);
        assert_eq!(NonMaxU8::ONE.get(), 1);
        assert_eq!(NonMaxU8::MAX.get(), 254);
        assert_eq!(NonMaxU8::default().get(), 0);
        assert_eq!(NonMaxI16::MAX.get(), i16::MAX - 1);
    }

    #[test]
    fn ordering_is_by_value_not_bits() {
        let zero = NonMaxU8::new(0).unwrap();
        let one = NonMaxU8::new(1).unwrap();
        let big = NonMaxU8::new(200).unwrap();
        assert!(zero < one);
        assert!(one < big);
        // NonMax stores !value, which reverses bit order; ensure Ord ignores that.
        assert!(big > one);
    }

    #[test]
    fn bitand_unsigned_matches_primitive() {
        for left in 0..=u8::MAX {
            for right in 0..=u8::MAX {
                let vanilla = left & right;
                if let (Some(l), Some(r)) = (NonMaxU8::new(left), NonMaxU8::new(right)) {
                    assert_eq!((l & r).get(), vanilla);
                }
                if let Some(l) = NonMaxU8::new(left) {
                    assert_eq!((l & right).get(), vanilla);
                    assert_eq!((left & l).get(), left & left); // primitive & niche
                }
            }
        }
    }

    #[test]
    fn bitand_signed_closed() {
        for left in i8::MIN..=i8::MAX {
            for right in i8::MIN..=i8::MAX {
                if let (Some(l), Some(r)) = (NonMaxI8::new(left), NonMaxI8::new(right)) {
                    assert_eq!((l & r).get(), left & right);
                }
            }
        }
    }

    #[test]
    fn conversions() {
        use core::convert::TryFrom;
        let v = NonValueU8::<7>::try_from(3).unwrap();
        assert_eq!(u8::from(v), 3);
        NonValueU8::<7>::try_from(7).unwrap_err();

        // widening From (nonmax parity)
        let small = NonMaxU8::new(200).unwrap();
        let large: NonMaxU32 = small.into();
        assert_eq!(large.get(), 200);
        let from_prim: NonMaxU16 = 200u8.into();
        assert_eq!(from_prim.get(), 200);
    }

    #[test]
    fn nonzero_interop() {
        use core::num::NonZeroU16;
        let nz = NonZeroU16::new(42).unwrap();
        // NonMinU16 == NonValueU16<0> == "non-zero u16"
        let nm: NonMinU16 = nz.into();
        assert_eq!(nm.get(), 42);
        let back: NonZeroU16 = nm.into();
        assert_eq!(back.get(), 42);
        assert!(NonMinU16::new(0).is_none());
    }

    #[test]
    fn parse() {
        let v: NonValueU8<7> = "3".parse().unwrap();
        assert_eq!(v.get(), 3);
        "7".parse::<NonValueU8<7>>().unwrap_err();
        "300".parse::<NonMaxU8>().unwrap_err();
    }

    #[test]
    fn fmt_forwards_to_value() {
        let v = NonValueU8::<7>::new(200).unwrap();
        assert_eq!(format!("{v}"), "200");
        assert_eq!(format!("{v:?}"), "200");
        assert_eq!(format!("{v:b}"), format!("{:b}", 200u8));
        assert_eq!(format!("{v:x}"), format!("{:x}", 200u8));
    }

    #[test]
    fn sizes_niche_optimized() {
        assert_eq!(size_of::<Option<NonValueU8<7>>>(), size_of::<u8>());
        assert_eq!(size_of::<Option<NonMaxU32>>(), size_of::<u32>());
        assert_eq!(size_of::<Option<NonMinI64>>(), size_of::<i64>());
        assert_eq!(size_of::<Option<NonValueUsize<3>>>(), size_of::<usize>());
    }

    #[test]
    fn const_context() {
        const V: NonValueU8<7> = match NonValueU8::<7>::new(9) {
            Some(v) => v,
            None => panic!(),
        };
        const G: u8 = V.get();
        assert_eq!(G, 9);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip() {
        let v = NonValueU8::<7>::new(3).unwrap();
        let bytes = bincode::serialize(&v).unwrap();
        let back: NonValueU8<7> = bincode::deserialize(&bytes).unwrap();
        assert_eq!(v, back);
        // forbidden value fails to deserialize
        let bad = bincode::serialize(&7u8).unwrap();
        assert!(bincode::deserialize::<NonValueU8<7>>(&bad).is_err());
    }
}
