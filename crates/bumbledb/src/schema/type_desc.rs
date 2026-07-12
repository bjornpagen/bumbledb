//! Lowering a structural [`ValueType`] to its encoding-level [`TypeDesc`].

use super::ValueType;
use crate::encoding::TypeDesc;

impl ValueType {
    /// The encoding-level description this type lowers to.
    #[must_use]
    pub fn type_desc(&self) -> TypeDesc {
        match self {
            Self::Bool => TypeDesc::Bool,
            Self::U64 => TypeDesc::U64,
            Self::I64 => TypeDesc::I64,
            Self::String => TypeDesc::String,
            Self::FixedBytes { len } => TypeDesc::FixedBytes { len: *len },
            Self::Interval { element } => TypeDesc::Interval { element: *element },
        }
    }
}
