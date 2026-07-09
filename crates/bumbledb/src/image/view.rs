//! Filtered views (docs/architecture/30-execution.md): per-atom filter evaluation producing
//! survivor-position vectors over images. Views are query-local and never
//! cached (`docs/architecture/40-storage.md`); COLT roots iterate the view,
//! and view positions index the image.

use std::sync::Arc;

use crate::image::RelationImage;
use crate::ir::CmpOp;
use crate::schema::FieldId;

mod apply;

pub use apply::apply;

#[cfg(test)]
mod build_with_filters;
#[cfg(test)]
mod positions;

#[cfg(test)]
pub use build_with_filters::build_with_filters;

/// The constant side of a lowered filter. `Word`/`Byte` are column form —
/// the byte-order-normalized word for 8-byte columns, the raw byte for
/// 1-byte columns. `Param` resolves at bind time through the evaluator's
/// param slice; `PendingIntern` is a raw String/Bytes literal resolved to
/// an intern-id word per execution (the 30-execution doc). Miss semantics
/// are per-operator: an `Eq` miss empties the whole query on this
/// snapshot (the evaluator never sees it); any other operator resolves
/// to the never-minted sentinel id, which `Ne` matches everywhere —
/// ordinary word comparison carries the semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Const {
    Word(u64),
    Byte(u8),
    /// An interval constant as its two encoded column words (each half
    /// byte-order-normalized exactly like a `Word`, so u64 word order is
    /// value order). Compared pairwise under `Eq`/`Ne`, and as the constant
    /// side of `Overlaps`/`Contains` compares and `FieldWithin`.
    Interval {
        start: u64,
        end: u64,
    },
    /// Bind-time symbolic constant; the evaluator indexes the param slice.
    Param(crate::ir::ParamId),
    /// A param bound as a *set* at execution (`Term::ParamSet`): resolves
    /// to a sorted, deduplicated word list; an `Eq` compare against it
    /// matches any element. The plan's selection machinery carries the set
    /// through the probe path (`docs/architecture/20-query-ir.md`,
    /// § param sets; executor side is PRD 17).
    ParamSet(crate::ir::ParamId),
    /// A raw String/Bytes literal awaiting per-execution intern resolution
    /// (`tag` is the dictionary type tag).
    PendingIntern {
        tag: u8,
        bytes: Box<[u8]>,
    },
}

/// Where a lowered point word comes from, per execution: an encoded
/// literal word (resolved at lowering), a bound param's word (resolved at
/// bind), or a bound variable's slot word (a membership binding whose
/// point variable is bound by another occurrence — evaluated once the
/// variable is bound; the point-membership scan of
/// `docs/architecture/40-execution.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedWordSource {
    Word(u64),
    Param(crate::ir::ParamId),
    Var(crate::ir::VarId),
}

/// One lowered per-atom filter (produced by the 20-query-ir doc's normalization).
///
/// The interval kinds are **fixed word-comparison compositions** over the
/// interval field's two encoded column words (`docs/architecture/40-execution.md`
/// — interval predicates lower to word comparisons over the start/end
/// column pair; no expression tree exists, three shapes as three kinds is
/// the representation-over-control-flow answer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterPredicate {
    /// `field <op> constant`. Under `Overlaps`/`Contains` the field is the
    /// **left** operand and the constant is `Interval`/`Param` by
    /// construction (the reversed containment is [`FilterPredicate::FieldWithin`]).
    Compare {
        field: FieldId,
        op: CmpOp,
        value: Const,
    },
    /// Same-fact comparison between two fields of one atom: `Eq` is the
    /// lowering of a repeated in-atom variable; any operator is the
    /// lowering of a same-atom var-vs-var comparison (residuals are
    /// cross-atom only — `docs/architecture/20-query-ir.md`). Both fields
    /// have the same structural type by validation, hence the same column
    /// kind, and word comparison is value-faithful (biased I64, ordinal
    /// bytes, injective intern ids; interval fields compare pairwise over
    /// their two-word span).
    FieldsCompare {
        left: FieldId,
        right: FieldId,
        op: CmpOp,
    },
    /// Point membership in the interval field: `start ≤ p AND p < end`
    /// over the field's two column words (the lowering of a membership
    /// binding, and of `Contains(field, point-constant)`).
    PointIn {
        field: FieldId,
        point: ResolvedWordSource,
    },
    /// Point-set membership in the interval field: any element of the
    /// bound set lies in the interval (`Term::ParamSet` on an interval
    /// field — `docs/architecture/20-query-ir.md`, § param sets).
    AnyPointIn {
        field: FieldId,
        set: crate::ir::ParamId,
    },
    /// Same-atom `Overlaps` over two interval fields:
    /// `left.start < right.end AND right.start < left.end`.
    FieldsOverlap { left: FieldId, right: FieldId },
    /// Same-atom `Contains` over two interval fields (point-set ⊇):
    /// `outer.start ≤ inner.start AND inner.end ≤ outer.end`.
    FieldsContain { outer: FieldId, inner: FieldId },
    /// Same-atom `Contains` with a point field (the predicate form of the
    /// membership rule, and the lowering of a same-atom membership-var
    /// binding): `interval.start ≤ point AND point < interval.end`.
    FieldsContainPoint { interval: FieldId, point: FieldId },
    /// The field's point-set within a constant interval — the reversed
    /// `Contains(constant, field)`. Over a scalar field:
    /// `outer.start ≤ f AND f < outer.end`; over an interval field:
    /// `outer.start ≤ f.start AND f.end ≤ outer.end`. `outer` is
    /// `Interval`/`Param` by construction.
    FieldWithin { field: FieldId, outer: Const },
}

/// A query-local view over an image: not yet bound to any generation
/// (the state every COLT holds between prepare and its first execution
/// — carrying *nothing*, so prepare pins no image), every position
/// (unfiltered), or the filter's survivors. A three-variant
/// representation, not a sentinel vector.
#[derive(Debug)]
pub enum View {
    /// No image at all: the view has not been bound to a generation.
    /// Unrepresentable as data that pins anything — a prepared query
    /// holds only `Unbound` views until it executes.
    Unbound,
    /// Every position `0..row_count`.
    All(Arc<RelationImage>),
    /// The survivor positions, in ascending order.
    Survivors {
        image: Arc<RelationImage>,
        positions: Vec<u32>,
    },
}

impl View {
    /// The underlying image.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unbound view has no image
    /// — every execution path binds (or rebuilds) the view before any
    /// probe or force can ask for one.
    #[must_use]
    pub fn image(&self) -> &Arc<RelationImage> {
        match self {
            Self::All(image) | Self::Survivors { image, .. } => image,
            Self::Unbound => unreachable!("an unbound view has no image"),
        }
    }

    /// Number of positions the view exposes (an unbound view exposes
    /// none).
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Unbound => 0,
            Self::All(image) => image.row_count(),
            Self::Survivors { positions, .. } => positions.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The image position at view index `idx` (reader: COLT root
    /// iteration, the 30-execution doc).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `idx` out of the view's range.
    #[must_use]
    pub fn position_at(&self, idx: usize) -> u32 {
        match self {
            Self::Unbound => unreachable!("an unbound view has no positions"),
            Self::All(_) => u32::try_from(idx).expect("positions fit u32"),
            Self::Survivors { positions, .. } => positions[idx],
        }
    }

    /// Reclaims the survivor buffer for reuse (the caller-owned storage
    /// discipline: buffers belong to the prepared query, the 30-execution doc).
    #[must_use]
    pub fn recycle(self) -> Vec<u32> {
        match self {
            Self::Unbound | Self::All(_) => Vec::new(),
            Self::Survivors { positions, .. } => positions,
        }
    }
}

#[cfg(test)]
mod tests;
