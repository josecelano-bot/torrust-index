use std::num::NonZeroU32;

/// A handle for identifying graph nodes.
///
/// The ID is guaranteed to be non-zero, with valid indices starting from 0
/// (which maps to an internal value of 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GNodeId(NonZeroU32);

/// A handle for identifying value nodes.
///
/// The ID is guaranteed to be non-zero, with valid indices starting from 0
/// (which maps to an internal value of 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VNodeId(NonZeroU32);

macro_rules! impl_handle {
    ($ty:ident) => {
        impl $ty {
            /// Creates a new handle from a zero-based index.
            ///
            /// # Panics
            /// Panics if the index would cause an overflow when converting to u32
            /// or if the resulting internal value exceeds u32::MAX.
            #[must_use]
            pub fn from_index(index: usize) -> Self {
                let raw = u32::try_from(index)
                    .ok()
                    .and_then(|i| i.checked_add(1))
                    .and_then(NonZeroU32::new)
                    .expect(concat!(stringify!($ty), ": index out of range"));
                Self(raw)
            }

            /// Returns the zero-based index corresponding to this handle.
            #[must_use]
            #[inline]
            pub const fn index(self) -> usize {
                (self.0.get() - 1) as usize
            }
        }

        impl TryFrom<u32> for $ty {
            type Error = &'static str;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                NonZeroU32::new(value)
                    .map(Self)
                    .ok_or("Handle ID must be non-zero")
            }
        }
    };
}

impl_handle!(GNodeId);
impl_handle!(VNodeId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_index_roundtrip() {
        for i in 0..100 {
            let id = GNodeId::from_index(i);
            assert_eq!(id.index(), i);
        }
        for i in 0..100 {
            let id = VNodeId::from_index(i);
            assert_eq!(id.index(), i);
        }
    }

    #[test]
    fn test_try_from_u32() {
        // Valid cases
        for i in 1..=10 {
            let id = GNodeId::try_from(i).unwrap();
            assert_eq!(id.0.get(), i);
            let id = VNodeId::try_from(i).unwrap();
            assert_eq!(id.0.get(), i);
        }

        // Invalid case: zero
        assert!(GNodeId::try_from(0).is_err());
        assert!(VNodeId::try_from(0).is_err());
    }
}
