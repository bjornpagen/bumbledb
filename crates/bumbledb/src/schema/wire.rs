//! The tags of the schema fingerprint's canonical encoding
//! ([`super::fingerprint`]), numbered densely from 1.

use super::{Bound, FieldId};

macro_rules! wire_tag {
    ($name:ident { $($var:ident = $val:literal),* $(,)? }) => {
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) enum $name {
            $($var = $val,)*
        }

        impl $name {
            pub(crate) const fn tag(self) -> u8 {
                self as u8
            }
        }
    };
}

wire_tag!(ValueTypeTag {
    Bool = 1,
    U64 = 2,
    I64 = 3,
    String = 4,
    FixedBytes = 5,
    Interval = 6,
    FixedInterval = 7,
    F64 = 8,
    Uuid = 9,
});

wire_tag!(IntervalElementTag {
    U64 = 1,
    I64 = 2,
    F64 = 3,
});

wire_tag!(ClosednessTag {
    Ordinary = 1,
    Closed = 2,
});

wire_tag!(StatementFormTag {
    Functionality = 1,
    Containment = 2,
    Capacity = 3,
});

wire_tag!(WeightTag {
    Unit = 1,
    Field = 2,
    DurationOf = 3,
});

wire_tag!(HiPresence {
    Absent = 1,
    Present = 2,
});

wire_tag!(BoundKind {
    Lit = 1,
    TargetField = 2,
    TargetDuration = 3,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EncodedHi {
    Unbounded,
    Lit(u64),
    TargetField(FieldId),
    TargetDuration(FieldId),
}

impl EncodedHi {
    pub(crate) fn from_bound(hi: Option<Bound>) -> Self {
        match hi {
            None => Self::Unbounded,
            Some(Bound::Lit(value)) => Self::Lit(value),
            Some(Bound::TargetField(field)) => Self::TargetField(field),
            Some(Bound::TargetDuration(field)) => Self::TargetDuration(field),
        }
    }

    pub(crate) fn write(self, out: &mut Vec<u8>) {
        match self {
            Self::Unbounded => out.push(HiPresence::Absent.tag()),
            Self::Lit(value) => {
                out.push(HiPresence::Present.tag());
                out.push(BoundKind::Lit.tag());
                out.extend_from_slice(&value.to_le_bytes());
            }
            Self::TargetField(field) => {
                out.push(HiPresence::Present.tag());
                out.push(BoundKind::TargetField.tag());
                out.extend_from_slice(&field.0.to_le_bytes());
            }
            Self::TargetDuration(field) => {
                out.push(HiPresence::Present.tag());
                out.push(BoundKind::TargetDuration.tag());
                out.extend_from_slice(&field.0.to_le_bytes());
            }
        }
    }
}
