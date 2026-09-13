//! Crate-private helper macros shared by the integer and float families.
//!
//! Every niche type forwards the same surface to its primitive: formatting,
//! `From`/`TryFrom`/`FromStr`, serde, and a compile-time layout check. Each
//! helper takes the impl generics in square brackets (`[const N: u8]`, or `[]`
//! for a concrete type) so one macro serves both generic and concrete types.

/// Implements the listed `core::fmt` traits by forwarding to `self.get()`.
macro_rules! forward_fmt {
    // The generics stay one bracketed token tree here so they can be repeated
    // once per trait; the `@one` arm splices them into the `impl` header.
    ($gen:tt $ty:ty => $($Trait:ident),+ $(,)?) => {
        $( forward_fmt!(@one $gen $ty, $Trait); )+
    };
    (@one [$($gen:tt)*] $ty:ty, $Trait:ident) => {
        impl<$($gen)*> core::fmt::$Trait for $ty {
            #[inline]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::$Trait::fmt(&self.get(), f)
            }
        }
    };
}

/// Implements `From<Self> for $prim`, `TryFrom<$prim> for Self` and `FromStr`
/// for a niche type whose checked constructor is `Self::new`. The two error
/// types are the crate's unit-payload error structs.
macro_rules! forward_conversions {
    ([$($gen:tt)*] $ty:ty, $prim:ty, $try_from_error:ident, $parse_error:ident $(,)?) => {
        impl<$($gen)*> From<$ty> for $prim {
            #[inline]
            fn from(value: $ty) -> Self {
                value.get()
            }
        }
        impl<$($gen)*> core::convert::TryFrom<$prim> for $ty {
            type Error = $try_from_error;
            #[inline]
            fn try_from(value: $prim) -> Result<Self, Self::Error> {
                <$ty>::new(value).ok_or($try_from_error(()))
            }
        }
        impl<$($gen)*> core::str::FromStr for $ty {
            type Err = $parse_error;
            #[inline]
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                <$ty>::new(<$prim as core::str::FromStr>::from_str(value)?).ok_or($parse_error(()))
            }
        }
    };
}

/// Implements `serde::Serialize`/`Deserialize` by the primitive's own
/// representation, re-checking the niche invariant on the way in.
macro_rules! forward_serde {
    ([$($gen:tt)*] $ty:ty, $prim:ty, $reject_msg:expr $(,)?) => {
        #[cfg(feature = "serde")]
        impl<$($gen)*> serde::Serialize for $ty {
            #[inline]
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.get().serialize(serializer)
            }
        }
        #[cfg(feature = "serde")]
        impl<'de, $($gen)*> serde::Deserialize<'de> for $ty {
            #[inline]
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <$prim as serde::Deserialize>::deserialize(deserializer)?;
                <$ty>::new(value).ok_or_else(|| serde::de::Error::custom($reject_msg))
            }
        }
    };
}

/// Asserts at compile time that `$ty` and `Option<$ty>` are both exactly the
/// size of `$prim`, i.e. that the `NonZero` niche survived.
macro_rules! assert_niche_layout {
    ($ty:ty, $prim:ty) => {
        const _: () = {
            assert!(core::mem::size_of::<$ty>() == core::mem::size_of::<$prim>());
            assert!(core::mem::size_of::<Option<$ty>>() == core::mem::size_of::<$prim>());
        };
    };
}
