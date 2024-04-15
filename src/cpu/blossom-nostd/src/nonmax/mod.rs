/*!

The public crate [nonmax](https://crates.io/crates/nonmax) offers a good starting point, but it has two drawbacks:

1. it uses XOR operation every time of accessing the value; indeed this would simply the implementation but it also
    incurs runtime costs
2. it doesn't have a C repr, which means that any value passed from C would have to go through type conversion

This implementation is a modified version of `nonmax`, which solves the above drawbacks.


nonmax provides types similar to the std `NonZero*` types, but instead requires
that their values are not the maximum for their type. This ensures that
`Option<NonMax*>` is no larger than `NonMax*`.

nonmax supports every type that has a corresponding non-zero variant in the
standard library:

* `NonMaxU8`
* `NonMaxU16`
* `NonMaxU32`
* `NonMaxU64`
* `NonMaxU128`
* `NonMaxUsize`

## Example

```
use micro_blossom_nostd::nonmax::NonMaxU8;

let value = NonMaxU8::new(16).option().expect("16 should definitely fit in a u8");
assert_eq!(value.get(), 16);
assert_eq!(std::mem::size_of_val(&value), 1);

let oops = NonMaxU8::new(255);
assert_eq!(oops.option(), None);
```

## Features

* `std` (default): implements [`std::error::Error`] for [`ParseIntError`] and
[`TryFromIntError`]. Disable this feature for
[`#![no_std]`](https://rust-embedded.github.io/book/intro/no-std.html) support.

## Minimum Supported Rust Version (MSRV)

nonmax supports Rust 1.47.0 and newer. Until this library reaches 1.0,
changes to the MSRV will require major version bumps. After 1.0, MSRV changes
will only require minor version bumps, but will need significant justification.
*/

/// An error type returned when a checked integral type conversion fails (mimics [std::num::TryFromIntError])
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryFromIntError(());

#[cfg(feature = "std")]
impl std::error::Error for TryFromIntError {}

impl core::fmt::Display for TryFromIntError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        "out of range integral type conversion attempted".fmt(fmt)
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

/// An error type returned when an integer cannot be parsed (mimics [std::num::ParseIntError])
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseIntError(());

#[cfg(feature = "std")]
impl std::error::Error for ParseIntError {}

impl core::fmt::Display for ParseIntError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        "unable to parse integer".fmt(fmt)
    }
}

impl From<core::num::ParseIntError> for ParseIntError {
    fn from(_: core::num::ParseIntError) -> Self {
        Self(())
    }
}

// error[E0658]: the `!` type is experimental
// https://github.com/rust-lang/rust/issues/35121
// impl From<!> for TryFromIntError { ... }

// https://doc.rust-lang.org/1.47.0/src/core/num/mod.rs.html#31-43
macro_rules! impl_nonmax_fmt {
    ( ( $( $Trait: ident ),+ ) for $nonmax: ident ) => {
        $(
            impl core::fmt::$Trait for $nonmax {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::$Trait::fmt(&self.get(), f)
                }
            }
        )+
    };
}

macro_rules! impl_option_nonmax_fmt {
    ( ( $( $Trait: ident ),+ ) for $option_nonmax: ident ) => {
        $(
            impl core::fmt::$Trait for $option_nonmax {
                #[inline]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::$Trait::fmt(&self.option(), f)
                }
            }
        )+
    };
}

macro_rules! nonmax {
    ( $nonmax: ident, $option_nonmax: ident, $primitive: ident ) => {
        /// An integer that is known not to equal its maximum value.
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $nonmax($primitive);

        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $option_nonmax($primitive);

        impl $option_nonmax {
            #[inline]
            #[track_caller]
            pub const fn unwrap(self) -> $nonmax {
                if self.0 == $primitive::MAX {
                    panic!("called `Option::unwrap()` on a `None` value");
                }
                $nonmax(self.0)
            }

            #[inline]
            pub const fn new(value: $primitive) -> Self {
                Self(value)
            }

            #[inline]
            pub const fn option(&self) -> Option<$nonmax> {
                if self.0 == $primitive::MAX {
                    None
                } else {
                    Some($nonmax(self.0))
                }
            }

            #[inline]
            pub const fn is_none(&self) -> bool {
                self.0 == $primitive::MAX
            }

            #[inline]
            pub const fn is_some(&self) -> bool {
                !self.is_none()
            }

            #[inline]
            pub fn set_none(&mut self) {
                self.0 = $primitive::MAX;
            }

            #[inline]
            #[track_caller]
            pub const unsafe fn unwrap_unchecked(self) -> $nonmax {
                debug_assert!(self.is_some());
                $nonmax(self.0)
            }

            pub const NONE: $option_nonmax = $option_nonmax($primitive::MAX);
        }

        impl From<Option<$nonmax>> for $option_nonmax {
            fn from(value: Option<$nonmax>) -> Self {
                match value {
                    Some(number) => number.option(),
                    None => Self::NONE,
                }
            }
        }

        impl $nonmax {
            /// Creates a new non-max if the given value is not the maximum
            /// value.
            #[inline]
            pub const fn new(value: $primitive) -> $option_nonmax {
                $option_nonmax(value)
            }

            #[inline]
            pub const fn option(self) -> $option_nonmax {
                $option_nonmax(self.0)
            }

            /// Creates a new non-max without checking the value.
            ///
            /// # Safety
            ///
            /// The value must not equal the maximum representable value for the
            /// primitive type.
            #[inline]
            pub const unsafe fn new_unchecked(value: $primitive) -> Self {
                Self(value)
            }

            /// Returns the value as a primitive type.
            #[inline]
            pub const fn get(&self) -> $primitive {
                self.0
            }

            /// Gets non-max with the value zero (0)
            pub const ZERO: $nonmax = unsafe { Self::new_unchecked(0) };

            /// Gets non-max with the value one (1)
            pub const ONE: $nonmax = unsafe { Self::new_unchecked(1) };

            /// Gets non-max with maximum possible value (which is maximum of the underlying primitive minus one)
            pub const MAX: $nonmax = unsafe { Self::new_unchecked($primitive::MAX - 1) };
        }

        impl Default for $nonmax {
            fn default() -> Self {
                unsafe { Self::new_unchecked(0) }
            }
        }

        impl From<$nonmax> for $primitive {
            fn from(value: $nonmax) -> Self {
                value.get()
            }
        }

        impl core::convert::TryFrom<$primitive> for $nonmax {
            type Error = TryFromIntError;
            fn try_from(value: $primitive) -> Result<Self, Self::Error> {
                Self::new(value).option().ok_or(TryFromIntError(()))
            }
        }

        impl core::str::FromStr for $nonmax {
            type Err = ParseIntError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new($primitive::from_str(value)?)
                    .option()
                    .ok_or(ParseIntError(()))
            }
        }

        impl core::cmp::Ord for $nonmax {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }
        impl core::cmp::PartialOrd for $nonmax {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        // https://doc.rust-lang.org/1.47.0/src/core/num/mod.rs.html#173-175
        impl_nonmax_fmt! {
            (Debug, Display, Binary, Octal, LowerHex, UpperHex) for $nonmax
        }

        impl_option_nonmax_fmt! {
            (Debug) for $option_nonmax
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $nonmax {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.get().serialize(serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $nonmax {
            fn deserialize<D>(deserializer: D) -> Result<$nonmax, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = $primitive::deserialize(deserializer)?;
                use core::convert::TryFrom;
                Self::try_from(value).map_err(serde::de::Error::custom)
            }
        }

        #[cfg(test)]
        mod $primitive {
            use super::*;

            use core::mem::size_of;

            #[test]
            fn construct() {
                let zero = $nonmax::new(0).unwrap();
                assert_eq!(zero.get(), 0);

                let some = $nonmax::new(19).unwrap();
                assert_eq!(some.get(), 19);

                let max = $nonmax::new($primitive::MAX);
                assert_eq!(max.option(), None);
            }

            #[test]
            fn sizes_correct() {
                assert_eq!(size_of::<$primitive>(), size_of::<$nonmax>());
                assert_eq!(size_of::<$nonmax>(), size_of::<$option_nonmax>());
            }

            #[test]
            fn convert() {
                use core::convert::TryFrom;
                let zero = $nonmax::try_from(0 as $primitive).unwrap();
                let zero = $primitive::from(zero);
                assert_eq!(zero, 0);

                $nonmax::try_from($primitive::MAX).unwrap_err();
            }

            #[test]
            fn cmp() {
                let zero = $nonmax::new(0).unwrap();
                let one = $nonmax::new(1).unwrap();
                let two = $nonmax::new(2).unwrap();
                assert!(zero < one);
                assert!(one < two);
                assert!(two > one);
                assert!(one > zero);
            }

            #[test]
            fn constants() {
                let zero = $nonmax::ZERO;
                let one = $nonmax::ONE;
                let max = $nonmax::MAX;
                assert_eq!(zero.get(), 0);
                assert_eq!(one.get(), 1);
                assert_eq!(max.get(), $primitive::MAX - 1);
            }

            #[test]
            #[cfg(feature = "std")] // to_string
            fn parse() {
                for value in [0, 19, $primitive::MAX - 1].iter().copied() {
                    let string = value.to_string();
                    let nonmax = string.parse::<$nonmax>().unwrap();
                    assert_eq!(nonmax.get(), value);
                }
                $primitive::MAX.to_string().parse::<$nonmax>().unwrap_err();
            }

            #[test]
            #[cfg(feature = "std")] // format!
            fn fmt() {
                let zero = $nonmax::new(0).unwrap();
                let some = $nonmax::new(19).unwrap();
                let max1 = $nonmax::new($primitive::MAX - 1).unwrap();
                for value in [zero, some, max1].iter().copied() {
                    assert_eq!(format!("{}", value.get()), format!("{}", value)); // Display
                    assert_eq!(format!("{:?}", value.get()), format!("{:?}", value)); // Debug
                    assert_eq!(format!("{:b}", value.get()), format!("{:b}", value)); // Binary
                    assert_eq!(format!("{:o}", value.get()), format!("{:o}", value)); // Octal
                    assert_eq!(format!("{:x}", value.get()), format!("{:x}", value)); // LowerHex
                    assert_eq!(format!("{:X}", value.get()), format!("{:X}", value)); // UpperHex
                }
            }
        }
    };
}

nonmax!(NonMaxU8, OptionNonMaxU8, u8);
nonmax!(NonMaxU16, OptionNonMaxU16, u16);
nonmax!(NonMaxU32, OptionNonMaxU32, u32);
nonmax!(NonMaxU64, OptionNonMaxU64, u64);
nonmax!(NonMaxU128, OptionNonMaxU128, u128);
nonmax!(NonMaxUsize, OptionNonMaxUsize, usize);

// https://doc.rust-lang.org/1.47.0/src/core/convert/num.rs.html#383-407
macro_rules! impl_nonmax_from {
    ( $small: ty, $large: ty ) => {
        impl From<$small> for $large {
            #[inline]
            fn from(small: $small) -> Self {
                // SAFETY: smaller input type guarantees the value is non-max
                unsafe { Self::new_unchecked(small.get().into()) }
            }
        }
    };
}

// Non-max Unsigned -> Non-max Unsigned
impl_nonmax_from!(NonMaxU8, NonMaxU16);
impl_nonmax_from!(NonMaxU8, NonMaxU32);
impl_nonmax_from!(NonMaxU8, NonMaxU64);
impl_nonmax_from!(NonMaxU8, NonMaxU128);
impl_nonmax_from!(NonMaxU8, NonMaxUsize);
impl_nonmax_from!(NonMaxU16, NonMaxU32);
impl_nonmax_from!(NonMaxU16, NonMaxU64);
impl_nonmax_from!(NonMaxU16, NonMaxU128);
impl_nonmax_from!(NonMaxU16, NonMaxUsize);
impl_nonmax_from!(NonMaxU32, NonMaxU64);
impl_nonmax_from!(NonMaxU32, NonMaxU128);
impl_nonmax_from!(NonMaxU64, NonMaxU128);

// https://doc.rust-lang.org/1.47.0/src/core/convert/num.rs.html#383-407
macro_rules! impl_smaller_from {
    ( $small: ty, $large: ty ) => {
        impl From<$small> for $large {
            #[inline]
            fn from(small: $small) -> Self {
                // SAFETY: smaller input type guarantees the value is non-max
                unsafe { Self::new_unchecked(small.into()) }
            }
        }
    };
}

// Unsigned -> Non-max Unsigned
impl_smaller_from!(u8, NonMaxU16);
impl_smaller_from!(u8, NonMaxU32);
impl_smaller_from!(u8, NonMaxU64);
impl_smaller_from!(u8, NonMaxU128);
impl_smaller_from!(u8, NonMaxUsize);
impl_smaller_from!(u16, NonMaxU32);
impl_smaller_from!(u16, NonMaxU64);
impl_smaller_from!(u16, NonMaxU128);
impl_smaller_from!(u16, NonMaxUsize);
impl_smaller_from!(u32, NonMaxU64);
impl_smaller_from!(u32, NonMaxU128);
impl_smaller_from!(u64, NonMaxU128);
