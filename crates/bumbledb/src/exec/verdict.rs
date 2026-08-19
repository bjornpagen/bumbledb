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

use crate::image::view::{
    Const, FilterPredicate, IntervalConst, OperandAddr, SlotOps, ViewWordSource, WordOrParam,
    duration_holds, holds,
};
use crate::ir::WordCmp;
use crate::ir::normalize::lower_literal;
use crate::ir::validate::{ClassifiedComparison, DurationOperand, SealedConst};

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

/// One written rule's compiled verdict: the Or over its lowered
/// disjuncts of the And over each disjunct's leaves — exactly the
/// Kleene fold of the written condition trees, by distributivity.
/// Compiled once at prepare against the ray probe's slot layout (the
/// disjuncts of one written rule share one variable scope, so one
/// layout serves them all). Leaves are the shared [`FilterPredicate`]
/// algebra, addressed at binding slots.
#[derive(Debug)]
pub struct CompiledVerdict {
    /// Per disjunct: `[start, end)` into `leaves`.
    disjuncts: Vec<(usize, usize)>,
    leaves: Vec<FilterPredicate>,
}

impl CompiledVerdict {
    /// Compiles one written rule's disjunct set against a slot layout.
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]
    pub(crate) fn compile(
        disjuncts: &[&[ClassifiedComparison]],
        slot_of: &impl Fn(crate::ir::VarId) -> usize,
        width_of: &impl Fn(crate::ir::VarId) -> usize,
    ) -> Self {
        let addr = |var: crate::ir::VarId| OperandAddr::from_span(slot_of(var), width_of(var));
        let pair = |var: crate::ir::VarId| OperandAddr::from_span(slot_of(var), 2);
        let word = |var: crate::ir::VarId| OperandAddr::from_slot(slot_of(var));
        let sealed = |constant: &SealedConst| match constant {
            SealedConst::Param(param) => Const::Param(*param),
            SealedConst::Literal(literal) => lower_literal(literal),
        };
        let interval = |constant: &SealedConst| match sealed(constant) {
            Const::Interval { start, end } => IntervalConst::Interval { start, end },
            Const::Param(param) => IntervalConst::Param(param),
            _ => unreachable!("validated: Allen/within constants are intervals"),
        };
        let point = |constant: &SealedConst| match sealed(constant) {
            Const::Word(w) => ViewWordSource::Word(w),
            Const::Byte(b) => ViewWordSource::Word(u64::from(b)),
            Const::Param(param) => ViewWordSource::Param(param),
            _ => unreachable!("validated: point constants are scalar words"),
        };
        let measure = |constant: &SealedConst| match sealed(constant) {
            Const::Word(w) => WordOrParam::Word(w),
            Const::Param(param) => WordOrParam::Param(param),
            _ => unreachable!("validated: measure constants are u64 words"),
        };
        let mut leaves = Vec::new();
        let mut ranges = Vec::with_capacity(disjuncts.len());
        for disjunct in disjuncts {
            let start = leaves.len();
            for comparison in *disjunct {
                leaves.push(match comparison {
                    ClassifiedComparison::VarVar { op, lhs, rhs } => {
                        FilterPredicate::FieldsCompare {
                            left: addr(*lhs),
                            right: addr(*rhs),
                            op: *op,
                        }
                    }
                    ClassifiedComparison::VarConst { op, var, value } => FilterPredicate::Compare {
                        field: addr(*var),
                        op: *op,
                        value: sealed(value),
                    },
                    ClassifiedComparison::VarInSet { var, set } => FilterPredicate::Compare {
                        field: addr(*var),
                        op: WordCmp::Eq,
                        value: Const::ParamSet(*set),
                    },
                    ClassifiedComparison::AllenVarVar { lhs, rhs, mask } => {
                        FilterPredicate::FieldsAllen {
                            left: pair(*lhs),
                            right: pair(*rhs),
                            mask: *mask,
                        }
                    }
                    ClassifiedComparison::AllenVarConst { var, other, mask } => {
                        FilterPredicate::FieldAllen {
                            field: pair(*var),
                            other: interval(other),
                            mask: *mask,
                        }
                    }
                    ClassifiedComparison::PointInVarVar { interval, point } => {
                        FilterPredicate::FieldsPointIn {
                            interval: pair(*interval),
                            point: word(*point),
                        }
                    }
                    ClassifiedComparison::PointInVarPoint {
                        interval: iv,
                        point: p,
                    } => FilterPredicate::PointIn {
                        field: pair(*iv),
                        point: point(p),
                    },
                    ClassifiedComparison::VarWithin { var, outer } => {
                        FilterPredicate::FieldWithin {
                            field: word(*var),
                            outer: interval(outer),
                        }
                    }
                    ClassifiedComparison::Duration {
                        interval: iv,
                        op,
                        other,
                    } => match other {
                        DurationOperand::Var(scalar) => FilterPredicate::DurationFieldsCompare {
                            interval: pair(*iv),
                            op: *op,
                            scalar: word(*scalar),
                        },
                        DurationOperand::Const(constant) => FilterPredicate::DurationCompare {
                            field: pair(*iv),
                            op: *op,
                            value: measure(constant),
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
    pub(crate) fn resolve_interns<C: crate::storage::catalog::CatalogRead>(
        &mut self,
        catalog: &C,
    ) -> crate::error::Result<()> {
        for leaf in &mut self.leaves {
            if let FilterPredicate::Compare {
                value: Const::PendingIntern { bytes },
                ..
            } = leaf
                && let Some(word) = catalog.dict_lookup(bytes)?
            {
                *leaf = match leaf {
                    FilterPredicate::Compare { field, op, .. } => FilterPredicate::Compare {
                        field: *field,
                        op: *op,
                        value: Const::Word(word.raw()),
                    },
                    _ => unreachable!(),
                };
            }
        }
        Ok(())
    }

    /// The Kleene fold at one binding: Or over disjuncts of And over
    /// leaves. Short-circuits only on absorbing elements (`Fails`
    /// absorbs And, `Holds` absorbs Or), so the cut never moves the
    /// verdict — order stays unobservable.
    pub(crate) fn eval(&self, word: &impl Fn(usize) -> u64, params: &[Const]) -> Verdict3 {
        let ops = SlotOps { word };
        let mut folded = Verdict3::Fails;
        for &(start, end) in &self.disjuncts {
            let mut conjunct = Verdict3::Holds;
            for leaf in &self.leaves[start..end] {
                match (conjunct, leaf_verdict(leaf, &ops, params)) {
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

fn leaf_verdict<F: Fn(usize) -> u64 + ?Sized>(
    leaf: &FilterPredicate,
    ops: &SlotOps<'_, F>,
    params: &[Const],
) -> Verdict3 {
    match leaf {
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            match duration_holds(leaf, ops, params).unwrap_or_else(|e| match e {}) {
                None => Verdict3::Ray,
                Some(holds) => Verdict3::of(holds),
            }
        }
        other => Verdict3::of(holds(other, ops, params).unwrap_or_else(|e| match e {})),
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
    ray: crate::exec::sink::RayPoison,
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
            ray: crate::exec::sink::RayPoison::Clear,
        }
    }

    /// The first Ray verdict's offending interval words, if any binding
    /// rendered one.
    pub(crate) fn measure_of_ray(&self) -> Option<[u64; 2]> {
        self.ray.span()
    }
}

impl crate::exec::run::Sink for RayArbiter<'_> {
    fn emit(&mut self, bindings: &crate::exec::run::Bindings) -> crate::exec::run::Flow {
        if matches!(self.ray, crate::exec::sink::RayPoison::Clear)
            && self.verdict.eval(&|slot| bindings.get(slot), self.params) == Verdict3::Ray
        {
            self.ray = crate::exec::sink::RayPoison::Hit([
                bindings.get(self.measured_slot),
                bindings.get(self.measured_slot + 1),
            ]);
        }
        crate::exec::run::Flow::Continue
    }

    fn emit_batch(&mut self, batch: &crate::exec::run::LeafBatch<'_>) -> crate::exec::run::Flow {
        for &entry in batch.survivors {
            if matches!(self.ray, crate::exec::sink::RayPoison::Hit(_)) {
                break;
            }
            let word = |slot: usize| match batch.source_of(slot) {
                crate::exec::run::LeafSource::Key(key_word) => batch.key(entry, key_word),
                crate::exec::run::LeafSource::Outer => batch.bindings.get(slot),
            };
            if self.verdict.eval(&word, self.params) == Verdict3::Ray {
                self.ray = crate::exec::sink::RayPoison::Hit([
                    word(self.measured_slot),
                    word(self.measured_slot + 1),
                ]);
            }
        }
        crate::exec::run::Flow::Continue
    }
}

#[cfg(test)]
mod tests;
