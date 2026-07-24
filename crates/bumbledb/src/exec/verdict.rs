//! The engine's statement of the Kleene verdict algebra (ruled
//! 2026-07-23, R6; `docs/architecture/20-query-ir.md` § the measure;
//! `lean/Bumbledb/Query/Aggregates.lean: Verdict3`): a binding's
//! condition verdict is [`Verdict3`] — Holds, Fails, or Ray — folded
//! over the written rule's disjuncts in the strong Kleene lattice, and
//! a binding raises `MeasureOfRay` iff its folded verdict is Ray. Both
//! connectives are commutative and associative and conjunction
//! distributes over disjunction, so the fold over the LOWERED disjunct
//! set equals the fold over the written condition trees — evaluation
//! order is unobservable, which is what makes the error semantics
//! well-defined over an IR whose condition lists compare as sets.
//!
//! The mainline execution never renders Ray: measure filters and
//! residuals drop rays (Fails-side of the comparison is never taken —
//! the row simply does not Hold), so the only place a Ray verdict can
//! be rendered is here, over the bindings the **ray probe** enumerates
//! (`ir/normalize::normalize_ray_probe`: the rule's atoms, negations,
//! and memberships with the conditions replaced by an is-ray filter on
//! one measured variable). The probe runs after the rule loop through
//! the ordinary Free Join machinery into [`RayArbiter`], which folds
//! this compiled verdict per binding and poisons on the first Ray.

use crate::image::view::{Const, MaskConst, mask_of};
use crate::ir::normalize::lower_literal;
use crate::ir::validate::{ClassifiedComparison, DurationOperand, SealedConst};
use crate::ir::{CmpOp, MaskTerm};

/// The three-valued verdict of one condition evaluation — the strong
/// Kleene lattice: `Fails` absorbs `and`, `Holds` absorbs `or`, `Ray`
/// propagates otherwise (`Verdict3.and`/`Verdict3.or` in the Lean
/// statement; the naive oracle folds the same lattice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict3 {
    Holds,
    Fails,
    Ray,
}

impl Verdict3 {
    /// A two-valued leaf's verdict.
    fn of(holds: bool) -> Self {
        if holds { Self::Holds } else { Self::Fails }
    }
}

/// One projected word span: a variable's first binding slot + width.
#[derive(Debug, Clone, Copy)]
struct Span {
    slot: usize,
    width: usize,
}

/// One compiled condition leaf, slot-level: validation's sealed
/// comparison shapes ([`ClassifiedComparison`]) with variables resolved
/// to the ray probe's binding-slot layout and literals lowered to
/// column form ([`lower_literal`] — the one owner of every case).
/// Every kind is two-valued except [`Leaf::Duration`], the IR's one
/// partial predicate.
#[derive(Debug)]
enum Leaf {
    /// Scalar (or `bytes<N>` span) var-vs-var under `Eq`/`Ne`/order.
    VarVar { op: CmpOp, lhs: Span, rhs: Span },
    /// Var-vs-constant under `Eq`/`Ne`/order, operator variable-on-left.
    /// A `PendingIntern` constant evaluates as the never-minted
    /// dictionary sentinel until [`CompiledVerdict::resolve_interns`]
    /// latches it (the dictionary is append-only, so a hit is final).
    VarConst { op: CmpOp, var: Span, value: Const },
    /// `Eq` against a bound set: span-wise membership in the sorted
    /// flat element-major word rows.
    VarInSet { var: Span, set: crate::ir::ParamId },
    /// The interval-pair comparison over two variables.
    AllenVarVar {
        lhs: usize,
        rhs: usize,
        mask: MaskConst,
    },
    /// The interval-pair comparison against a constant interval.
    AllenVarConst {
        var: usize,
        other: Const,
        mask: MaskConst,
    },
    /// `interval-var ∋ point-var`.
    PointInVarVar { interval: usize, point: usize },
    /// `interval-var ∋ constant point`.
    PointInVarPoint { interval: usize, point: Const },
    /// `constant interval ∋ scalar-var`.
    VarWithin { var: usize, outer: Const },
    /// The measure comparison — three-valued: a ray (`end == MAX`) is
    /// the Ray verdict, never Fails.
    Duration {
        interval: usize,
        op: CmpOp,
        rhs: DurationSide,
    },
}

/// The measure's comparison side, slot-level.
#[derive(Debug)]
enum DurationSide {
    Slot(usize),
    Value(Const),
}

/// One written rule's compiled verdict: the Or over its lowered
/// disjuncts of the And over each disjunct's leaves — exactly the
/// Kleene fold of the written condition trees, by distributivity.
/// Compiled once at prepare against the ray probe's slot layout (the
/// disjuncts of one written rule share one variable scope, so one
/// layout serves them all).
#[derive(Debug)]
pub struct CompiledVerdict {
    /// Per disjunct: `[start, end)` into `leaves`.
    disjuncts: Vec<(usize, usize)>,
    leaves: Vec<Leaf>,
}

impl CompiledVerdict {
    /// Compiles one written rule's disjunct set against a slot layout.
    pub(crate) fn compile(
        disjuncts: &[&[ClassifiedComparison]],
        slot_of: &impl Fn(crate::ir::VarId) -> usize,
        width_of: &impl Fn(crate::ir::VarId) -> usize,
    ) -> Self {
        let span = |var: crate::ir::VarId| Span {
            slot: slot_of(var),
            width: width_of(var),
        };
        let sealed = |constant: &SealedConst| match constant {
            SealedConst::Param(param) => Const::Param(*param),
            SealedConst::Literal(literal) => lower_literal(literal),
        };
        let mut leaves = Vec::new();
        let mut ranges = Vec::with_capacity(disjuncts.len());
        for disjunct in disjuncts {
            let start = leaves.len();
            for comparison in *disjunct {
                leaves.push(match comparison {
                    ClassifiedComparison::VarVar { op, lhs, rhs } => Leaf::VarVar {
                        op: *op,
                        lhs: span(*lhs),
                        rhs: span(*rhs),
                    },
                    ClassifiedComparison::VarConst { op, var, value } => Leaf::VarConst {
                        op: *op,
                        var: span(*var),
                        value: sealed(value),
                    },
                    ClassifiedComparison::VarInSet { var, set } => Leaf::VarInSet {
                        var: span(*var),
                        set: *set,
                    },
                    ClassifiedComparison::AllenVarVar { lhs, rhs, mask } => Leaf::AllenVarVar {
                        lhs: slot_of(*lhs),
                        rhs: slot_of(*rhs),
                        mask: match mask {
                            MaskTerm::Literal(mask) => MaskConst::Mask(*mask),
                            MaskTerm::Param(param) => MaskConst::Param(*param),
                        },
                    },
                    ClassifiedComparison::AllenVarConst { var, other, mask } => {
                        Leaf::AllenVarConst {
                            var: slot_of(*var),
                            other: sealed(other),
                            mask: *mask,
                        }
                    }
                    ClassifiedComparison::PointInVarVar { interval, point } => {
                        Leaf::PointInVarVar {
                            interval: slot_of(*interval),
                            point: slot_of(*point),
                        }
                    }
                    ClassifiedComparison::PointInVarPoint { interval, point } => {
                        Leaf::PointInVarPoint {
                            interval: slot_of(*interval),
                            point: sealed(point),
                        }
                    }
                    ClassifiedComparison::VarWithin { var, outer } => Leaf::VarWithin {
                        var: slot_of(*var),
                        outer: sealed(outer),
                    },
                    ClassifiedComparison::Duration {
                        interval,
                        op,
                        other,
                    } => Leaf::Duration {
                        interval: slot_of(*interval),
                        op: *op,
                        rhs: match other {
                            DurationOperand::Var(scalar) => DurationSide::Slot(slot_of(*scalar)),
                            DurationOperand::Const(constant) => {
                                DurationSide::Value(sealed(constant))
                            }
                        },
                    },
                });
            }
            ranges.push((start, leaves.len()));
        }
        Self {
            disjuncts: ranges,
            leaves,
        }
    }

    /// Latches `str` literals to their dictionary words — append-only,
    /// so a hit is final; a miss stays pending and evaluates as the
    /// never-minted sentinel this execution (exactly the bind path's
    /// missed-param reading).
    ///
    /// # Errors
    ///
    /// `Lmdb`/`Corruption` from the dictionary read.
    pub(crate) fn resolve_interns(
        &mut self,
        txn: &crate::storage::env::ReadTxn<'_>,
    ) -> crate::error::Result<()> {
        for leaf in &mut self.leaves {
            if let Leaf::VarConst { value, .. } = leaf
                && let Const::PendingIntern { bytes } = value
                && let Some(word) = crate::storage::dict::lookup(txn, bytes)?
            {
                *value = Const::Word(word);
            }
        }
        Ok(())
    }

    /// The Kleene fold at one binding: Or over disjuncts of And over
    /// leaves. Short-circuits only on absorbing elements (`Fails`
    /// absorbs And, `Holds` absorbs Or), so the cut never moves the
    /// verdict — order stays unobservable.
    pub(crate) fn eval(&self, word: &impl Fn(usize) -> u64, params: &[Const]) -> Verdict3 {
        let mut folded = Verdict3::Fails;
        for &(start, end) in &self.disjuncts {
            let mut conjunct = Verdict3::Holds;
            for leaf in &self.leaves[start..end] {
                match (conjunct, leaf_verdict(leaf, word, params)) {
                    (_, Verdict3::Fails) => {
                        conjunct = Verdict3::Fails;
                        break;
                    }
                    (Verdict3::Holds, verdict) => conjunct = verdict,
                    _ => {}
                }
            }
            match (folded, conjunct) {
                (_, Verdict3::Holds) => return Verdict3::Holds,
                (Verdict3::Fails, verdict) => folded = verdict,
                _ => {}
            }
        }
        folded
    }
}

/// Resolves a leaf constant through the bind-time param slice (the
/// `apply` evaluator's rule, restated over the verdict's leaves).
fn resolve<'a>(value: &'a Const, params: &'a [Const]) -> &'a Const {
    match value {
        Const::Param(param) | Const::ParamSet(param) => &params[usize::from(param.0)],
        other => other,
    }
}

/// One constant's word at a scalar position: `Word` verbatim, `Byte`
/// widened (bool's strict 0/1 encoding), the dictionary sentinel for a
/// still-pending intern (a miss equals nothing — the never-minted id).
fn const_word(value: &Const, params: &[Const]) -> u64 {
    match resolve(value, params) {
        Const::Word(word) => *word,
        Const::Byte(byte) => u64::from(*byte),
        Const::PendingIntern { .. } => crate::storage::dict::SENTINEL_ID,
        other => unreachable!("validated: a scalar comparison side resolves scalar, got {other:?}"),
    }
}

/// One constant's encoded interval words.
fn const_interval(value: &Const, params: &[Const]) -> (u64, u64) {
    match resolve(value, params) {
        Const::Interval { start, end } => (*start, *end),
        other => unreachable!("validated: an interval side resolves to an interval, got {other:?}"),
    }
}

/// Point membership under the half-open interval.
const fn point_in(start: u64, end: u64, point: u64) -> bool {
    start <= point && point < end
}

/// One leaf's verdict at one binding — two-valued everywhere except the
/// measure, whose ray is the lattice's third value.
fn leaf_verdict(leaf: &Leaf, word: &impl Fn(usize) -> u64, params: &[Const]) -> Verdict3 {
    match leaf {
        Leaf::VarVar { op, lhs, rhs } => {
            debug_assert_eq!(lhs.width, rhs.width, "validated: one shared type");
            Verdict3::of(if lhs.width == 1 {
                op.compare(&word(lhs.slot), &word(rhs.slot))
            } else {
                // bytes<N> spans: word-wise identity, Eq/Ne only by
                // validation.
                let identical = (0..lhs.width).all(|i| word(lhs.slot + i) == word(rhs.slot + i));
                match op {
                    CmpOp::Eq => identical,
                    CmpOp::Ne => !identical,
                    _ => unreachable!("validated: spans compare under Eq/Ne only"),
                }
            })
        }
        Leaf::VarConst { op, var, value } => Verdict3::of(match resolve(value, params) {
            Const::Words(words) => {
                debug_assert_eq!(var.width, words.len(), "validated width");
                let identical = words
                    .iter()
                    .enumerate()
                    .all(|(i, expected)| word(var.slot + i) == *expected);
                match op {
                    CmpOp::Eq => identical,
                    CmpOp::Ne => !identical,
                    _ => unreachable!("validated: bytes<N> compares under Eq/Ne only"),
                }
            }
            resolved => op.compare(&word(var.slot), &const_word(resolved, params)),
        }),
        Leaf::VarInSet { var, set } => {
            let Const::WordSet(words) = &params[usize::from(set.0)] else {
                unreachable!("validated: a set param resolves to a word set")
            };
            Verdict3::of(if var.width == 1 {
                words.binary_search(&word(var.slot)).is_ok()
            } else {
                // Flat element-major rows: span-wise binary search.
                debug_assert_eq!(words.len() % var.width, 0, "flat element-major rows");
                let mut lo = 0usize;
                let mut hi = words.len() / var.width;
                let mut hit = false;
                while lo < hi {
                    let mid = usize::midpoint(lo, hi);
                    let row = &words[mid * var.width..(mid + 1) * var.width];
                    let ordering = (0..var.width)
                        .map(|i| row[i].cmp(&word(var.slot + i)))
                        .find(|o| o.is_ne())
                        .unwrap_or(std::cmp::Ordering::Equal);
                    match ordering {
                        std::cmp::Ordering::Less => lo = mid + 1,
                        std::cmp::Ordering::Greater => hi = mid,
                        std::cmp::Ordering::Equal => {
                            hit = true;
                            break;
                        }
                    }
                }
                hit
            })
        }
        Leaf::AllenVarVar { lhs, rhs, mask } => Verdict3::of(mask_of(*mask, params).contains(
            crate::allen::classify_bounds(&word(*lhs), &word(lhs + 1), &word(*rhs), &word(rhs + 1)),
        )),
        Leaf::AllenVarConst { var, other, mask } => {
            let (start, end) = const_interval(other, params);
            Verdict3::of(
                mask_of(*mask, params).contains(crate::allen::classify_bounds(
                    &word(*var),
                    &word(var + 1),
                    &start,
                    &end,
                )),
            )
        }
        Leaf::PointInVarVar { interval, point } => {
            Verdict3::of(point_in(word(*interval), word(interval + 1), word(*point)))
        }
        Leaf::PointInVarPoint { interval, point } => Verdict3::of(point_in(
            word(*interval),
            word(interval + 1),
            const_word(point, params),
        )),
        Leaf::VarWithin { var, outer } => {
            let (start, end) = const_interval(outer, params);
            Verdict3::of(point_in(start, end, word(*var)))
        }
        Leaf::Duration { interval, op, rhs } => {
            let (start, end) = (word(*interval), word(interval + 1));
            if end == u64::MAX {
                return Verdict3::Ray;
            }
            let scalar = match rhs {
                DurationSide::Slot(slot) => word(*slot),
                DurationSide::Value(value) => const_word(value, params),
            };
            Verdict3::of(op.compare(&(end - start), &scalar))
        }
    }
}

/// The ray probe's sink: folds the compiled verdict at every enumerated
/// binding (all of them have some measured interval a ray — the probe's
/// one filter) and records the first Ray with the measured interval's
/// two encoded words. Never skips, never scans — an arbiter, not an
/// answer consumer.
pub struct RayArbiter<'a> {
    verdict: &'a CompiledVerdict,
    params: &'a [Const],
    /// The probed variable's first binding slot — the offending
    /// interval's words for the typed error payload.
    measured_slot: usize,
    ray: Option<[u64; 2]>,
}

impl<'a> RayArbiter<'a> {
    pub(crate) fn new(
        verdict: &'a CompiledVerdict,
        params: &'a [Const],
        measured_slot: usize,
    ) -> Self {
        Self {
            verdict,
            params,
            measured_slot,
            ray: None,
        }
    }

    /// The first Ray verdict's offending interval words, if any binding
    /// rendered one.
    pub(crate) fn measure_of_ray(&self) -> Option<[u64; 2]> {
        self.ray
    }
}

impl crate::exec::run::Sink for RayArbiter<'_> {
    fn emit(&mut self, bindings: &crate::exec::run::Bindings) -> crate::exec::run::Flow {
        if self.ray.is_none()
            && self.verdict.eval(&|slot| bindings.get(slot), self.params) == Verdict3::Ray
        {
            self.ray = Some([
                bindings.get(self.measured_slot),
                bindings.get(self.measured_slot + 1),
            ]);
        }
        crate::exec::run::Flow::Continue
    }

    fn emit_batch(
        &mut self,
        batch: &crate::exec::run::LeafBatch<'_>,
        _stop_on_skip: bool,
    ) -> crate::exec::run::Flow {
        for &entry in batch.survivors {
            if self.ray.is_some() {
                break;
            }
            let word = |slot: usize| match batch.source_of(slot) {
                crate::exec::run::LeafSource::Key(key_word) => batch.key(entry, key_word),
                crate::exec::run::LeafSource::Outer => batch.bindings.get(slot),
            };
            if self.verdict.eval(&word, self.params) == Verdict3::Ray {
                self.ray = Some([word(self.measured_slot), word(self.measured_slot + 1)]);
            }
        }
        crate::exec::run::Flow::Continue
    }
}

#[cfg(test)]
mod tests;
