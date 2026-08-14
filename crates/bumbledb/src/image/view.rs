//! Filtered views (docs/architecture/40-execution.md): per-atom filter evaluation producing
//! survivor-position vectors over images. Views are query-local and never
//! cached (`docs/architecture/50-storage.md`); COLT roots iterate the view,
//! and view positions index the image.

use std::sync::Arc;

use crate::image::RelationImage;
use crate::ir::CmpOp;
use bumbledb_theory::schema::FieldId;

mod apply;

pub use apply::apply;
pub(crate) use apply::mask_of;

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
/// an intern-id word per execution (the 40-execution doc). Miss semantics
/// are per-operator: an `Eq` miss empties the whole query on this
/// snapshot (the evaluator never sees it); any other operator resolves
/// to the never-minted sentinel id, which `Ne` matches everywhere —
/// ordinary word comparison carries the semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Const {
    Word(u64),
    Byte(u8),
    /// A multi-word `bytes<N>` constant (N > 8): its `⌈N/8⌉` encoded
    /// column words, in column order — the padded canonical bytes read as
    /// big-endian words, exactly what the image's word columns hold. A
    /// bytes<N ≤ 8> constant is one padded word and rides [`Const::Word`]
    /// like every other scalar. Compared word-wise under `Eq`/`Ne` only.
    Words(Box<[u64]>),
    /// An interval constant as its two encoded column words (each half
    /// byte-order-normalized exactly like a `Word`, so u64 word order is
    /// value order). Compared pairwise under `Eq`, and the constant side
    /// of `FieldAllen` and `FieldWithin`.
    Interval {
        start: u64,
        end: u64,
    },
    /// Bind-time symbolic constant; the evaluator indexes the param slice.
    Param(crate::ir::ParamId),
    /// A param bound as a *set* at execution (`Term::ParamSet`): resolves
    /// to a sorted, deduplicated word list ([`Const::WordSet`] in the param
    /// slice); an `Eq` compare against it matches any element. The plan's
    /// selection machinery carries the set through the probe path
    /// (`docs/architecture/20-query-ir.md`, § param sets; executor side is
    /// PRD 17).
    ParamSet(crate::ir::ParamId),
    /// A set's bind-time resolution: the sorted, deduplicated column words
    /// of the bound elements, in pooled storage — the `Vec` is reused
    /// across binds (warm re-binds of a differently-sized set reuse its
    /// capacity, docs/architecture/40-execution.md § allocation contract).
    /// Flat element-major rows: each element contributes its column-word
    /// span (1 word for every scalar, `⌈N/8⌉` for a `bytes<N>` element —
    /// the anchored field's span names the width), sorted and
    /// deduplicated span-wise. Lives in the evaluator's param slice and
    /// in resolved filters — a `ParamSet` marker resolves to one of
    /// these.
    WordSet(Vec<u64>),
    /// A raw String literal awaiting per-execution intern resolution —
    /// the dictionary is str-only, so no type tag exists
    /// (docs/architecture/50-storage.md).
    PendingIntern {
        bytes: Box<[u8]>,
    },
}

/// View-evaluator point word: a resolved literal or a bind-time param.
/// Plan/exec membership probes keep [`ResolvedWordSource::Var`] on the
/// shared enum; a view-level [`FilterPredicate::PointIn`] cannot spell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewWordSource {
    Word(u64),
    Param(crate::ir::ParamId),
}

/// [`FilterPredicate::DurationCompare`]'s constant side — a u64 word or a
/// bind-time param. Same shape as [`ViewWordSource`]; a distinct name so
/// the filter site documents the measure, not a membership point.
pub type WordOrParam = ViewWordSource;

/// [`FilterPredicate::AnyPointIn`]'s set: a bind-time param-set marker or
/// its resolved word list. The param slice still holds [`Const::WordSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetConst {
    ParamSet(crate::ir::ParamId),
    WordSet(Vec<u64>),
}

/// [`FilterPredicate::FieldAllen`] / [`FilterPredicate::FieldWithin`]
/// constant side: an interval literal or a bind-time param. The param
/// slice still holds [`Const::Interval`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntervalConst {
    Interval { start: u64, end: u64 },
    Param(crate::ir::ParamId),
}

/// Where a lowered point word comes from, per execution: an encoded
/// literal word (resolved at lowering), a bound param's word (resolved at
/// bind), or a bound variable's slot word (a membership binding whose
/// point variable is bound by another occurrence — evaluated once the
/// variable is bound; the point-membership scan of
/// `docs/architecture/40-execution.md`). A `Var` source never reaches the
/// view evaluator: plan validation routes [`FilterPredicate::PointVar`]
/// into the executor's membership probes (`PlanNode::point_probes` for
/// positive occurrences, the anti-probe's point checks for negated ones),
/// because a view is built per execution while a variable binds per join
/// row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "plan/exec membership probes keep Var; view filters use ViewWordSource"
)]
pub enum ResolvedWordSource {
    Word(u64),
    Param(crate::ir::ParamId),
    Var(crate::ir::VarId),
}

/// The mask side of a lowered `Allen` shape: a resolved 13-bit mask,
/// with the mirrored form pre-encoded (`Allen(a, b, m) ≡ Allen(b, a,
/// converse(m))`, `crate::allen`). A comparison written constant-first
/// lowers with the field kept on the left and the mask already conversed.
pub type MaskConst = bumbledb_theory::allen::AllenMask;

/// One lowered per-atom filter (produced by the 20-query-ir doc's normalization).
///
/// The membership kinds are **fixed word-comparison compositions** over
/// the interval field's two encoded column words; the `Allen` kinds carry
/// the mask with the four endpoint operands — the configuration kernel's
/// operand shape (`exec/kernel/allen.rs`; classify-then-test scalar on
/// the refine path). No expression tree exists: shapes as kinds is
/// the representation-over-control-flow answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterPredicate {
    /// `field <op> constant`. An interval field appears here only under
    /// `Eq` (a value-equality binding); every interval-pair *predicate*
    /// is an `Allen` kind below.
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
    /// their two-word span — repeated-variable `Eq` only: interval
    /// comparisons canonicalize to masks).
    FieldsCompare {
        left: FieldId,
        right: FieldId,
        op: CmpOp,
    },
    /// Point membership in the interval field: `start ≤ p AND p < end`
    /// over the field's two column words (the lowering of a membership
    /// binding, and of `PointIn(field, point-constant)`).
    PointIn {
        field: FieldId,
        point: ViewWordSource,
    },
    /// Var-sourced point membership: plan validation lifts this into
    /// membership probes and strips it from the view filter list. The
    /// view evaluator never sees it.
    PointVar {
        field: FieldId,
        var: crate::ir::VarId,
    },
    /// Point-set membership in the interval field: any element of the
    /// bound set lies in the interval (`Term::ParamSet` on an interval
    /// field — `docs/architecture/20-query-ir.md`, § param sets). `set`
    /// is [`SetConst::ParamSet`] in the lowered template and resolves to a
    /// [`SetConst::WordSet`] per execution, exactly like a `Compare`
    /// constant.
    AnyPointIn { field: FieldId, set: SetConst },
    /// Same-atom `Allen` over two interval fields:
    /// `classify(left, right) ∈ mask` — four endpoint words and the mask,
    /// the whole algebra as one shape.
    FieldsAllen {
        left: FieldId,
        right: FieldId,
        mask: MaskConst,
    },
    /// `Allen` between an interval field (always the **left** operand —
    /// the mirrored form is pre-encoded in the mask, [`MaskConst`]) and
    /// an interval constant (`Interval`/`Param` by construction):
    /// `classify(field, other) ∈ mask`.
    FieldAllen {
        field: FieldId,
        other: IntervalConst,
        mask: MaskConst,
    },
    /// Same-atom `PointIn` with a point field (the predicate form of the
    /// membership rule, and the lowering of a same-atom membership-var
    /// binding): `interval.start ≤ point AND point < interval.end`.
    FieldsPointIn { interval: FieldId, point: FieldId },
    /// A scalar field's point within a constant interval — the reversed
    /// point membership `PointIn(constant, field)`:
    /// `outer.start ≤ f AND f < outer.end`. `outer` is `Interval`/`Param`
    /// by construction; the field is scalar by construction (an interval
    /// field under a constant is [`FilterPredicate::FieldAllen`]).
    FieldWithin {
        field: FieldId,
        outer: IntervalConst,
    },
    /// The measure against a constant: `(end − start) <op> value` over
    /// the interval field's two encoded column words — one subtraction,
    /// exact for both element types (the encodings are unit-spaced
    /// order-preserving maps onto u64 words: u64 is the identity, I64
    /// the +2⁶³ bias, so the bias cancels and `end > start` by the
    /// constructor invariant keeps the difference exact). `op` is an
    /// order operator and `value` a u64 word (`Word` or `Param`) by
    /// validation.
    ///
    /// **The filter-order law (normative for both measure kinds):** the
    /// measure evaluates only on facts surviving the atom's *other*
    /// filters — an `Allen` ray filter or a bounded-end filter on the
    /// same atom always runs first, so a filtered fact never reaches the
    /// subtraction. On the survivors a ray (`end == MAX`) never passes
    /// the comparison AND never errors here: its verdict is Ray, not
    /// Fails (the Kleene verdict algebra, ruled 2026-07-23, R6), and the
    /// prepared query's ray-probe pass renders it after the rule loop —
    /// [`crate::Error::MeasureOfRay`] iff some complete binding's folded
    /// verdict is Ray (`exec/verdict.rs`).
    DurationCompare {
        field: FieldId,
        op: CmpOp,
        value: WordOrParam,
    },
    /// The same-atom measure comparison: `(end − start) <op> scalar`
    /// where the u64 side is another field of the same fact (the
    /// lowering of `Duration(v) <op> w` with both variables bound on one
    /// occurrence). Ray semantics and the filter-order law as
    /// [`FilterPredicate::DurationCompare`].
    DurationFieldsCompare {
        interval: FieldId,
        op: CmpOp,
        scalar: FieldId,
    },
}

/// A view bound to a generation: every position, or the filter's survivors.
/// Prepare stores [`View::Unbound`]; the executor holds this after bind.
#[derive(Debug)]
pub enum BoundView {
    /// Every position `0..row_count`.
    All(Arc<RelationImage>),
    /// The survivor positions, in ascending order.
    Survivors {
        image: Arc<RelationImage>,
        positions: Vec<u32>,
    },
}

impl BoundView {
    /// The underlying image.
    #[must_use]
    pub fn image(&self) -> &Arc<RelationImage> {
        match self {
            Self::All(image) | Self::Survivors { image, .. } => image,
        }
    }

    /// Number of positions the bound view exposes.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::All(image) => image.row_count(),
            Self::Survivors { positions, .. } => positions.len(),
        }
    }

    /// The image position at view index `idx` (reader: COLT root
    /// iteration, the 40-execution doc).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `idx` out of the view's range.
    #[must_use]
    pub fn position_at(&self, idx: usize) -> u32 {
        match self {
            Self::All(_) => u32::try_from(idx).expect("positions fit u32"),
            Self::Survivors { positions, .. } => positions[idx],
        }
    }
}

/// A query-local view over an image: not yet bound to any generation
/// (the state every COLT holds between prepare and its first execution
/// — carrying *nothing*, so prepare pins no image), or a [`BoundView`].
/// A three-variant representation, not a sentinel vector.
#[derive(Debug)]
pub enum View {
    /// No image at all: the view has not been bound to a generation.
    /// Unrepresentable as data that pins anything — a prepared query
    /// holds only `Unbound` views until it executes.
    Unbound,
    /// Bound to a generation — every position, or the filter's survivors.
    Bound(BoundView),
}

impl View {
    /// Number of positions the view exposes (an unbound view exposes
    /// none).
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Unbound => 0,
            Self::Bound(bound) => bound.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bound view, if this view has been bound to a generation.
    #[must_use]
    pub fn bound(&self) -> Option<&BoundView> {
        match self {
            Self::Unbound => None,
            Self::Bound(bound) => Some(bound),
        }
    }

    /// Clones the view into caller-owned storage: the image `Arc` bumps,
    /// survivor positions copy into `buffer` (capacity retained — the
    /// occurrence-dedup path's storage discipline: the prepared query's
    /// spare buffer circulates through here exactly as through `apply`).
    #[must_use]
    pub fn clone_in(&self, mut buffer: Vec<u32>) -> Self {
        buffer.clear();
        match self {
            Self::Unbound => Self::Unbound,
            Self::Bound(BoundView::All(image)) => Self::Bound(BoundView::All(Arc::clone(image))),
            Self::Bound(BoundView::Survivors { image, positions }) => {
                buffer.extend_from_slice(positions);
                Self::Bound(BoundView::Survivors {
                    image: Arc::clone(image),
                    positions: buffer,
                })
            }
        }
    }

    /// Reclaims the survivor buffer for reuse (the caller-owned storage
    /// discipline: buffers belong to the prepared query, the 40-execution doc).
    #[must_use]
    pub fn recycle(self) -> Vec<u32> {
        match self {
            Self::Unbound | Self::Bound(BoundView::All(_)) => Vec::new(),
            Self::Bound(BoundView::Survivors { positions, .. }) => positions,
        }
    }
}

#[cfg(test)]
mod tests;
