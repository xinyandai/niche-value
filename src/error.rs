//! Error types returned by the checked conversions.
//!
//! The integer error types mirror [`nonmax`](https://docs.rs/nonmax)'s
//! `TryFromIntError` / `ParseIntError` (which themselves mirror the
//! `core::num` types) so that code written against `nonmax` keeps compiling.
//! The float error types mirror `core::num::ParseFloatError` naming.

/// Error returned when a checked integral conversion fails (mirrors
/// [`core::num::TryFromIntError`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryFromIntError(pub(crate) ());

impl core::fmt::Display for TryFromIntError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        "out of range integral type conversion attempted".fmt(f)
    }
}

impl From<core::num::TryFromIntError> for TryFromIntError {
    fn from(_: core::num::TryFromIntError) -> Self {
        Self(())
    }
}

impl From<core::convert::Infallible> for TryFromIntError {
    fn from(never: core::convert::Infallible) -> Self {
        match never {}
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TryFromIntError {}

/// Error returned when an integer string cannot be parsed into a niche integer
/// (mirrors [`core::num::ParseIntError`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIntError(pub(crate) ());

impl core::fmt::Display for ParseIntError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        "unable to parse integer".fmt(f)
    }
}

impl From<core::num::ParseIntError> for ParseIntError {
    fn from(_: core::num::ParseIntError) -> Self {
        Self(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseIntError {}

/// Error returned when a checked `f32`/`f64` conversion fails (the value was the
/// forbidden bit pattern, or `NaN`/`±inf` for the class-based types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryFromFloatError(pub(crate) ());

impl core::fmt::Display for TryFromFloatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        "float value is forbidden by this niche type".fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TryFromFloatError {}

/// Error returned when a float string cannot be parsed into a niche float
/// (mirrors [`core::num::ParseFloatError`] naming).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFloatError(pub(crate) ());

impl core::fmt::Display for ParseFloatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        "unable to parse float".fmt(f)
    }
}

impl From<core::num::ParseFloatError> for ParseFloatError {
    fn from(_: core::num::ParseFloatError) -> Self {
        Self(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseFloatError {}
