//! Lowering a structural [`ValueType`] to its encoding-level [`TypeDesc`].

use super::ValueType;
use crate::encoding::TypeDesc;

impl ValueType {
    /// The encoding-level description this type lowers to.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: an enum with more than 256
    /// variants, which schema validation makes unconstructible.
    #[must_use]
    pub fn type_desc(&self) -> TypeDesc {
        match self {
            Self::Bool => TypeDesc::Bool,
            Self::Enum { variants } => TypeDesc::Enum {
                variant_count: u16::try_from(variants.len())
                    .expect("validated schema: <=256 enum variants"),
            },
            Self::U64 => TypeDesc::U64,
            Self::I64 => TypeDesc::I64,
            Self::String => TypeDesc::String,
            Self::Bytes => TypeDesc::Bytes,
            Self::Interval { element } => TypeDesc::Interval { element: *element },
        }
    }
}
