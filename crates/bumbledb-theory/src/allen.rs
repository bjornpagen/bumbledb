//! Allen's interval algebra as a coordinate system — the mask vocabulary
//! (docs/architecture/20-query-ir.md § the Allen operator).
//!
//! The 13 basic relations are jointly exhaustive and pairwise disjoint
//! over nonempty half-open intervals (the type's precondition —
//! [`crate::Interval`]), so the set of all interval-pair predicates *is*
//! the powerset 2¹³: one operator parameterized by a 13-bit mask replaces
//! an operator vocabulary permanently. Nothing exists outside the
//! coordinate system, so the vocabulary can never grow again.
//!
//! **The bit order is a specified representation, not an implementation
//! detail**: bit *i* is basic relation *i* in the palindromic order
//! (before, meets, overlaps, starts, during, finishes, **equals**,
//! finished-by, contains, started-by, overlapped-by, met-by, after).
//! Each basic's converse sits at the mirrored position, so
//! [`AllenMask::converse`] is the 13-bit reversal — one `rbit` plus a
//! shift, scalar or vector: the bits are laid out as the algebra's
//! symmetry, and the involution costs one instruction.
//!
//! This module is the vocabulary only. Classification — the scalar
//! reference `classify` and the batch configuration kernel — is engine
//! machinery and stays in `bumbledb` (`bumbledb::allen`,
//! `exec/kernel/allen.rs`).

/// One of Allen's 13 basic relations, under Allen's own names. The
/// discriminant is the basic's bit index in the palindromic order (the
/// module doc) — `converse` is `12 − i` by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Basic {
    /// `a` ends before `b` starts: `a.end < b.start`.
    Before = 0,
    /// `a` ends exactly where `b` starts: `a.end == b.start` — under
    /// half-open intervals the two share no point.
    Meets = 1,
    /// `a` starts first, they share points, `b` ends last:
    /// `a.start < b.start < a.end < b.end`.
    Overlaps = 2,
    /// Same start, `a` ends first: `a.start == b.start`, `a.end < b.end`.
    Starts = 3,
    /// `a` strictly inside `b`: `b.start < a.start`, `a.end < b.end`.
    During = 4,
    /// Same end, `a` starts later: `b.start < a.start`, `a.end == b.end`.
    Finishes = 5,
    /// Identical bounds.
    Equals = 6,
    /// Same end, `a` starts first (converse of finishes).
    FinishedBy = 7,
    /// `b` strictly inside `a` (converse of during).
    Contains = 8,
    /// Same start, `a` ends later (converse of starts).
    StartedBy = 9,
    /// `b` starts first, they share points, `a` ends last (converse of
    /// overlaps).
    OverlappedBy = 10,
    /// `b` ends exactly where `a` starts (converse of meets).
    MetBy = 11,
    /// `b` ends before `a` starts (converse of before).
    After = 12,
}

impl Basic {
    /// This basic's bit in the mask coordinate system.
    #[must_use]
    pub const fn bit(self) -> u16 {
        1 << (self as u16)
    }

    /// The converse basic — the mirrored position (`12 − i`): the
    /// palindromic order makes this arithmetic, not a table.
    #[must_use]
    pub const fn converse(self) -> Self {
        // The discriminant IS the bit index; `12 − i` is total over 0..=12.
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

    /// All 13 basics in bit order (test walks and oracle sweeps).
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
///
/// The empty and full masks are representable values of the algebra
/// (`converse` and `complement` are total); *as predicates* they are
/// vacuous, and the query boundary rejects both with distinct typed
/// errors (`docs/architecture/20-query-ir.md`). Bits above the low 13 are
/// unrepresentable: [`AllenMask::new`] parses, the constants and the
/// closed operations (`|`, `converse`, `complement`) preserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AllenMask(u16);

/// The all-13-bits word.
const ALL_BITS: u16 = (1 << 13) - 1;

impl AllenMask {
    /// The 13 singletons, under Allen's names.
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

    /// The point-sets share a point — the 9 middle bits (everything but
    /// before, meets, met-by, after: under half-open intervals *meets*
    /// shares no point).
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
    /// Point-set ⊇: equals ∪ contains ∪ started-by ∪ finished-by.
    pub const COVERS: Self = Self(
        Basic::Equals.bit()
            | Basic::Contains.bit()
            | Basic::StartedBy.bit()
            | Basic::FinishedBy.bit(),
    );
    /// Point-set ⊆ — [`AllenMask::COVERS`]'s converse: equals ∪ during ∪
    /// starts ∪ finishes.
    pub const COVERED_BY: Self = Self(
        Basic::Equals.bit() | Basic::During.bit() | Basic::Starts.bit() | Basic::Finishes.bit(),
    );
    /// The point-sets share no point: before ∪ meets ∪ met-by ∪ after
    /// (the complement of [`AllenMask::INTERSECTS`]) — and the pointwise
    /// key judgment's per-pair statement
    /// (`docs/architecture/30-dependencies.md`).
    pub const DISJOINT: Self =
        Self(Basic::Before.bit() | Basic::Meets.bit() | Basic::MetBy.bit() | Basic::After.bit());
    /// All 13 basics — the vacuous "always" (rejected as a condition; a
    /// value of the algebra, e.g. the complement's identity).
    pub const FULL: Self = Self(ALL_BITS);
    /// No basic — the vacuous "never" (rejected as a condition).
    pub const EMPTY: Self = Self(0);

    /// Parses raw bits; `None` when any bit above the low 13 is set.
    #[must_use]
    pub const fn new(bits: u16) -> Option<Self> {
        if bits & !ALL_BITS == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// The raw 13-bit word.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Whether the mask holds `basic`.
    #[must_use]
    pub const fn contains(self, basic: Basic) -> bool {
        self.0 & basic.bit() != 0
    }

    /// The converse mask: `Allen(a, b, m) ≡ Allen(b, a, converse(m))`.
    /// The palindromic bit order makes it the 13-bit reversal — one bit
    /// reversal plus a shift.
    #[must_use]
    pub const fn converse(self) -> Self {
        Self(self.0.reverse_bits() >> 3)
    }

    /// The complement within the 13-bit space: `¬m` holds exactly when
    /// `m` does not (JEPD — every pair is in exactly one basic).
    #[must_use]
    pub const fn complement(self) -> Self {
        Self(!self.0 & ALL_BITS)
    }

    /// Number of basics in the mask (the selectivity model's numerator,
    /// the engine's `plan/selectivity.rs`).
    #[must_use]
    pub const fn popcount(self) -> u32 {
        self.0.count_ones()
    }

    /// The vacuous "never" — rejected as a condition at the query
    /// boundary.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The vacuous "always" — rejected as a condition at the query
    /// boundary.
    #[must_use]
    pub const fn is_full(self) -> bool {
        self.0 == ALL_BITS
    }
}

impl std::ops::BitOr for AllenMask {
    type Output = Self;

    /// Set union — how composite masks are written (`BEFORE | MEETS`).
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{AllenMask, Basic};

    /// The mask constants under their definitions, and the parse boundary.
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
        // Bits above the low 13 are unrepresentable, not truncated.
        assert!(AllenMask::new(0x2000).is_none());
        assert!(AllenMask::new(0x1FFF).is_some());
    }

    /// The converse involution over the FULL mask space, with the domain
    /// size asserted as a counted loop bound (the crucible packet (git ecec1dc3)
    /// 15-exhaustive-miri.md: the loop bound is the claim).
    ///
    /// Domain arithmetic: a mask is a 13-bit word, so the whole space is
    /// 2¹³ = 8,192 masks — every one visited, none sampled.
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
            // The 13-bit reversal stays inside the mask space and
            // preserves cardinality (a permutation of the bits).
            assert_eq!(mask.converse().popcount(), mask.popcount());
            visited += 1;
        }
        assert_eq!(visited, 8_192, "the full 2^13 mask space was enumerated");
    }

    /// Mask converse agrees with per-basic converse across the whole
    /// mask space (the classification-duality half of this law lives
    /// engine-side, with `classify`).
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
