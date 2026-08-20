//! Shared descriptor-wire tags. The encoder ([`super::fingerprint`]) names
//! these discriminants; the bytes are the historical stream (tag 1 remains
//! the deleted-enum tombstone). There is no decoder.

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
    Bool = 0,
    U64 = 2,
    I64 = 3,
    String = 4,
    FixedBytes = 5,
    Interval = 6,
    FixedInterval = 7,
});

wire_tag!(IntervalElementTag {
    U64 = 0,
    I64 = 1,
});

wire_tag!(GenerationTag {
    None = 0,
    Fresh = 1,
});

wire_tag!(ClosednessTag {
    Ordinary = 0,
    Closed = 1,
});

wire_tag!(StatementFormTag {
    Functionality = 0,
    Containment = 1,
    Capacity = 4,
});

wire_tag!(WeightTag {
    Unit = 0,
    Field = 1,
    DurationOf = 2,
});

wire_tag!(HiPresence {
    Absent = 0,
    Present = 1,
});

wire_tag!(BoundKind {
    Lit = 0,
    TargetField = 1,
    TargetDuration = 2,
});

/// The four-arm ceiling the sealed side already names. Encodes as the
/// nested presence+kind tags the historical stream uses — bytes unchanged.
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
