//! Filtered views: per-atom filter evaluation producing
//! survivor-position vectors over images. Views are query-local and never
//! cached; COLT roots iterate the view,
//! and view positions index the image.
use std::sync::Arc;

use crate::image::RelationImage;

mod apply;
mod eval;

pub use apply::apply;
pub(crate) use eval::{
    DENSE_NEG_INF_KEY, ImageRow, Loaded, OperandAddr, Operands, dense_probe_word,
    element_probe_word, holds, is_prepare_resolvable, render_filter, resolve_filter_into,
};

#[cfg(test)]
mod positions;

/// The constant side of a lowered filter. `Word`/`Byte` are column form —
/// the byte-order-normalized word for 8-byte columns, the raw byte for
/// 1-byte columns. `Param` resolves at bind time through the evaluator's
/// param slice; `PendingIntern` is immutable UTF-8 literal bytes resolved to
/// an owned `Text` in a separate resolved slot. `Text` and `WordSet` keep
/// canonical text alive when a resolved filter or selection is copied. The
/// template retains its bytes so rotation can resolve it again. Interning
/// an unstored text still produces an exact word that matches no row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Const {
    Word(u64),
    Text(super::intern::InternedText),
    Byte(u8),

    Words(Box<[u64]>),

    Interval { start: u64, end: u64 },

    Param(crate::ir::ParamId),

    ParamSet(crate::ir::ParamId),

    // Keep text ownership out of every scalar constant's inline footprint.
    WordSet(Box<ResolvedWords>),

    PendingIntern { bytes: Box<[u8]> },
}

/// Column-form selection keys or set elements, with owners for any resident
/// text tokens. The executor borrows only `words`; cloning retained query
/// state also clones these shared owners, never the string payloads.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ResolvedWords {
    pub(crate) words: Vec<u64>,
    pub(crate) texts: Vec<super::intern::InternedText>,
}

impl Clone for ResolvedWords {
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
            texts: self.texts.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.words.clone_from(&source.words);
        self.texts.clone_from(&source.texts);
    }
}

impl From<Vec<u64>> for ResolvedWords {
    fn from(words: Vec<u64>) -> Self {
        Self {
            words,
            texts: Vec::new(),
        }
    }
}

impl From<Vec<u64>> for Box<ResolvedWords> {
    fn from(words: Vec<u64>) -> Self {
        Box::new(ResolvedWords::from(words))
    }
}

impl AsRef<[u64]> for ResolvedWords {
    fn as_ref(&self) -> &[u64] {
        &self.words
    }
}

impl ResolvedWords {
    pub(crate) fn clear(&mut self) {
        self.words.clear();
        self.texts.clear();
    }

    pub(crate) fn push_text(&mut self, text: super::intern::InternedText) {
        self.words.push(text.word);
        self.texts.push(text);
    }

    pub(crate) fn dedup_texts(&mut self) {
        self.texts.sort_unstable_by_key(|text| text.word);
        self.texts.dedup_by_key(|text| text.word);
    }
}

/// View-evaluator point word: a resolved literal or a bind-time param.
/// Plan/exec membership probes keep [`ResolvedWordSource::Var`] on the
/// shared enum; a view-level [`FilterPredicate::PointIn`] cannot spell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewWordSource {
    Word(u64),
    Param(crate::ir::ParamId),
}

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
/// bind), or a bound variable's slot word. A `Var` source never reaches the
/// view evaluator: plan validation routes occurrence `point_vars` into
/// the executor's membership probes (`PlanNode::point_probes` for
/// positive occurrences, the anti-probe's point checks for negated ones),
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
/// The membership kinds are **fixed word-comparison compositions** over
/// the interval field's two encoded column words; the `Allen` kinds carry
/// the mask with the four endpoint operands — the configuration kernel's
/// operand shape (`exec/kernel/allen.rs`; classify-then-test scalar on
/// the refine path). No expression tree exists: shapes as kinds is
/// the representation-over-control-flow answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterPredicate {
    Compare {
        field: OperandAddr,
        op: crate::ir::WordCmp,
        value: Const,
    },

    FieldsCompare {
        left: OperandAddr,
        right: OperandAddr,
        op: crate::ir::WordCmp,
    },

    PointIn {
        field: OperandAddr,
        point: ViewWordSource,
        /// The field's element domain is the dense F64 line: nonfinite
        /// probe words are ordinary NONMATCHES, guarded
        /// at evaluation (`eval::dense_probe_word`) — word order alone
        /// would wrongly admit `-Infinity` into a left ray.
        dense: bool,
    },

    AnyPointIn {
        field: OperandAddr,
        set: SetConst,
        /// As [`FilterPredicate::PointIn::dense`].
        dense: bool,
    },

    FieldsAllen {
        left: OperandAddr,
        right: OperandAddr,
        mask: MaskConst,
    },

    FieldAllen {
        field: OperandAddr,
        other: IntervalConst,
        mask: MaskConst,
    },

    FieldsPointIn {
        interval: OperandAddr,
        point: OperandAddr,
        /// As [`FilterPredicate::PointIn::dense`].
        dense: bool,
    },

    FieldWithin {
        field: OperandAddr,
        outer: IntervalConst,
        /// As [`FilterPredicate::PointIn::dense`]: the scalar field is
        /// F64, so a stored nonfinite value never lies within any range.
        dense: bool,
    },
}

impl FilterPredicate {
    pub(crate) fn compare_sides(&self) -> (OperandAddr, OperandAddr, crate::ir::WordCmp) {
        match *self {
            Self::FieldsCompare { left, right, op } => (left, right, op),
            _ => unreachable!("kind-grouped compare residual"),
        }
    }

    pub(crate) fn allen_sides(&self) -> (OperandAddr, OperandAddr, MaskConst) {
        match *self {
            Self::FieldsAllen { left, right, mask } => (left, right, mask),
            _ => unreachable!("kind-grouped Allen residual"),
        }
    }
}

/// A view bound to a generation: every position, or the filter's survivors.
/// Prepare stores [`View::Unbound`]; the executor holds this after bind.
#[derive(Debug)]
pub enum BoundView {
    All(Arc<RelationImage>),

    Survivors {
        image: Arc<RelationImage>,
        positions: Vec<u32>,
    },
}

impl BoundView {
    #[must_use]
    pub fn image(&self) -> &Arc<RelationImage> {
        match self {
            Self::All(image) | Self::Survivors { image, .. } => image,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::All(image) => image.row_count(),
            Self::Survivors { positions, .. } => positions.len(),
        }
    }

    /// # Panics
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
    Unbound,

    Bound(BoundView),
}

impl View {
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

    #[must_use]
    pub fn bound(&self) -> Option<&BoundView> {
        match self {
            Self::Unbound => None,
            Self::Bound(bound) => Some(bound),
        }
    }

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
