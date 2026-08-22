//! Allen's interval algebra as a coordinate system — the mask vocabulary
//! .
//! The 13 basic relations are jointly exhaustive and pairwise disjoint
//! over nonempty half-open intervals (the type's precondition —
//! [`crate::Interval`]), so the set of all interval-pair predicates *is*
//! the powerset 2¹³: one operator parameterized by a 13-bit mask replaces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Basic {
    Before = 0,

    Meets = 1,

    Overlaps = 2,

    Starts = 3,

    During = 4,

    Finishes = 5,

    Equals = 6,

    FinishedBy = 7,

    Contains = 8,

    StartedBy = 9,

    OverlappedBy = 10,

    MetBy = 11,

    After = 12,
}

impl Basic {
    #[must_use]
    pub const fn bit(self) -> u16 {
        1 << (self as u16)
    }

    #[must_use]
    pub const fn converse(self) -> Self {
        match self {
            Self::Before => Self::After,
            Self::Meets => Self::MetBy,
            Self::Overlaps => Self::OverlappedBy,
            Self::Starts => Self::StartedBy,
            Self::During => Self::Contains,
            Self::Finishes => Self::FinishedBy,
            Self::Equals => Self::Equals,
            Self::FinishedBy => Self::Finishes,
            Self::Contains => Self::During,
            Self::StartedBy => Self::Starts,
            Self::OverlappedBy => Self::Overlaps,
            Self::MetBy => Self::Meets,
            Self::After => Self::Before,
        }
    }

    pub const ALL: [Self; 13] = [
        Self::Before,
        Self::Meets,
        Self::Overlaps,
        Self::Starts,
        Self::During,
        Self::Finishes,
        Self::Equals,
        Self::FinishedBy,
        Self::Contains,
        Self::StartedBy,
        Self::OverlappedBy,
        Self::MetBy,
        Self::After,
    ];
}

/// A set of Allen basic relations: a 13-bit mask, bit *i* = [`Basic`] *i*
/// in the palindromic order (module doc). A mask **is** an interval-pair
/// predicate — `Allen(a, b, m)` holds iff `classify(a, b) ∈ m` — and every
/// interval-pair predicate is a mask.
/// (`converse` and `complement` are total); *as predicates* they are
/// vacuous, and the query boundary rejects both with distinct typed
/// errors. Bits above the low 13 are
/// unrepresentable: [`AllenMask::new`] parses, the constants and the
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AllenMask(u16);

/// The all-13-bits word.
const ALL_BITS: u16 = (1 << 13) - 1;

impl AllenMask {
    pub const BEFORE: Self = Self(Basic::Before.bit());
    pub const MEETS: Self = Self(Basic::Meets.bit());
    pub const OVERLAPS: Self = Self(Basic::Overlaps.bit());
    pub const STARTS: Self = Self(Basic::Starts.bit());
    pub const DURING: Self = Self(Basic::During.bit());
    pub const FINISHES: Self = Self(Basic::Finishes.bit());
    pub const EQUALS: Self = Self(Basic::Equals.bit());
    pub const FINISHED_BY: Self = Self(Basic::FinishedBy.bit());
    pub const CONTAINS: Self = Self(Basic::Contains.bit());
    pub const STARTED_BY: Self = Self(Basic::StartedBy.bit());
    pub const OVERLAPPED_BY: Self = Self(Basic::OverlappedBy.bit());
    pub const MET_BY: Self = Self(Basic::MetBy.bit());
    pub const AFTER: Self = Self(Basic::After.bit());

    pub const INTERSECTS: Self = Self(
        Basic::Overlaps.bit()
            | Basic::Starts.bit()
            | Basic::During.bit()
            | Basic::Finishes.bit()
            | Basic::Equals.bit()
            | Basic::FinishedBy.bit()
            | Basic::Contains.bit()
            | Basic::StartedBy.bit()
            | Basic::OverlappedBy.bit(),
    );

    pub const COVERS: Self = Self(
        Basic::Equals.bit()
            | Basic::Contains.bit()
            | Basic::StartedBy.bit()
            | Basic::FinishedBy.bit(),
    );

    pub const COVERED_BY: Self = Self(
        Basic::Equals.bit() | Basic::During.bit() | Basic::Starts.bit() | Basic::Finishes.bit(),
    );

    pub const DISJOINT: Self =
        Self(Basic::Before.bit() | Basic::Meets.bit() | Basic::MetBy.bit() | Basic::After.bit());

    pub const FULL: Self = Self(ALL_BITS);

    pub const EMPTY: Self = Self(0);

    #[must_use]
    pub const fn new(bits: u16) -> Option<Self> {
        if bits & !ALL_BITS == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn contains(self, basic: Basic) -> bool {
        self.0 & basic.bit() != 0
    }

    #[must_use]
    pub const fn converse(self) -> Self {
        Self(self.0.reverse_bits() >> 3)
    }

    #[must_use]
    pub const fn complement(self) -> Self {
        Self(!self.0 & ALL_BITS)
    }

    #[must_use]
    pub const fn popcount(self) -> u32 {
        self.0.count_ones()
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn is_full(self) -> bool {
        self.0 == ALL_BITS
    }
}

impl std::ops::BitOr for AllenMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{AllenMask, Basic};

    #[test]
    fn constants_and_parse_shape() {
        assert_eq!(AllenMask::INTERSECTS.popcount(), 9);
        assert_eq!(AllenMask::INTERSECTS.complement(), AllenMask::DISJOINT);
        assert_eq!(AllenMask::COVERS.converse(), AllenMask::COVERED_BY);
        assert_eq!(AllenMask::DISJOINT.converse(), AllenMask::DISJOINT);
        assert_eq!(
            AllenMask::COVERS,
            AllenMask::EQUALS
                | AllenMask::CONTAINS
                | AllenMask::STARTED_BY
                | AllenMask::FINISHED_BY
        );
        assert!(AllenMask::EMPTY.is_empty());
        assert!(AllenMask::FULL.is_full());
        assert_eq!(AllenMask::EQUALS.complement().popcount(), 12);

        assert!(AllenMask::new(0x2000).is_none());
        assert!(AllenMask::new(0x1FFF).is_some());
    }

    #[test]
    fn exhaustive_converse_involution_over_all_8192_masks() {
        let mut visited = 0u32;
        for bits in 0..=0x1FFF_u16 {
            let mask = AllenMask::new(bits).expect("13-bit range");
            assert_eq!(
                mask.converse().converse(),
                mask,
                "involution at {bits:#06x}"
            );

            assert_eq!(mask.converse().popcount(), mask.popcount());
            visited += 1;
        }
        assert_eq!(visited, 8_192, "the full 2^13 mask space was enumerated");
    }

    #[test]
    fn mask_converse_agrees_with_basic_converse() {
        for bits in 0..=0x1FFF_u16 {
            let mask = AllenMask::new(bits).expect("13-bit range");
            for basic in Basic::ALL {
                assert_eq!(
                    mask.contains(basic),
                    mask.converse().contains(basic.converse())
                );
            }
        }
    }
}
