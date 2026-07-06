//! The two consumers of bindings (docs/architecture/30-execution.md): set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup
//! (`docs/architecture/30-execution.md` D2/D3; semantics normative in
//! `20-query-ir.md`).
//!
//! Aggregation never materializes the join: group maps live in sink state;
//! the fold domain of every aggregate is the group's **set of distinct
//! full bindings over all query variables** — two postings of amount 100
//! to one account are two distinct bindings (their serial ids differ), so
//! `Sum(amount) by account` is 200. The stated footgun: joining a
//! multiplicity-adding relation multiplies the binding set, exactly as in
//! SQL.

use crate::encoding::encode_i64;
use crate::error::{Error, Result};
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafSource, Sink};
use crate::exec::wordmap::WordMap;
use crate::ir::AggOp;

/// One find term in execution form: a projected slot or an aggregate spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindSpec {
    /// A projected (group-key) variable's binding slot.
    Var { slot: usize },
    /// An aggregate over a slot (`None` for the nullary Count).
    Agg {
        op: AggOp,
        over_slot: Option<usize>,
        /// Whether the input is I64 (its column word is the sign-flipped
        /// biased form; Sum must decode before accumulating).
        signed: bool,
    },
}

/// Decodes a binding word back to the i64 it encodes (the biased word form
/// is order-preserving; arithmetic needs the logical value).
fn word_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

fn i64_to_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

/// The projection sink: dedups projected find tuples, and reports
/// staleness (`SkipSuffix`) so the executor can unwind suffixes that bind
/// nothing projection-relevant (D2 — legal for this sink only).
#[derive(Debug)]
pub struct ProjectionSink {
    slots: Vec<usize>,
    seen: WordMap<()>,
    scratch: Vec<u64>,
    /// Per-slot leaf-batch sources, recomputed once per batch (PRD 01):
    /// `Some(word)` reads the batch keys, `None` the outer bindings.
    batch_sources: Vec<Option<usize>>,
}

impl ProjectionSink {
    /// `slots`: the projected variables' binding slots, in find order.
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        let arity = slots.len();
        Self {
            slots,
            seen: WordMap::new(arity),
            scratch: vec![0; arity],
            batch_sources: vec![None; arity],
        }
    }

    /// The distinct projected tuples, unordered (results are sets; the
    /// host sorts).
    pub fn rows(&self) -> impl Iterator<Item = &[u64]> {
        self.seen.iter().map(|(key, ())| key)
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.seen.clear();
    }
}

impl Sink for ProjectionSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for (i, slot) in self.slots.iter().enumerate() {
            self.scratch[i] = bindings.get(*slot);
        }
        self.seen.insert(&self.scratch);
        // The doc's first-emit signal (30-execution D2): once a projected
        // tuple lands — new or duplicate — the current suffix can only
        // multiply witnesses. The executor's sink_relevant gating
        // (run.rs's skip-absorption arm) decides how far the skip
        // unwinds — for projections the bits come from the group key
        // (hardening PRD 05); signaling on the *first* emit (not the
        // first duplicate) saves one full suffix descent per distinct
        // output tuple.
        Flow::SkipSuffix
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        // Sources once per batch; outer slots prefilled once — the row
        // loop touches only the varying key words and the seen-set.
        for (i, slot) in self.slots.iter().enumerate() {
            match batch.source_of(*slot) {
                LeafSource::Key(word) => self.batch_sources[i] = Some(word),
                LeafSource::Outer => {
                    self.batch_sources[i] = None;
                    self.scratch[i] = batch.bindings.get(*slot);
                }
            }
        }
        for &entry in batch.survivors {
            for (i, source) in self.batch_sources.iter().enumerate() {
                if let Some(word) = source {
                    self.scratch[i] = batch.key(entry, *word);
                }
            }
            self.seen.insert(&self.scratch);
            if stop_on_skip {
                // First-emit semantics (see `emit`): the remaining rows
                // bind nothing sink-relevant — the executor unwinds.
                return Flow::SkipSuffix;
            }
        }
        Flow::Continue
    }

    fn may_skip(&self) -> bool {
        true
    }
}

/// One accumulator cell.
#[derive(Debug, Clone, Copy)]
enum Acc {
    /// i128 accumulation: deterministic under any fold order — set folds
    /// have none; one range check at finalization (u128 for unsigned).
    SumSigned(i128),
    SumUnsigned(u128),
    /// Min/Max compare column words — correct because words are
    /// order-preserving (docs/architecture/30-execution.md).
    Min(u64),
    Max(u64),
    Count(u64),
}

/// The aggregate sink: group map keyed by the group-key words, folding each
/// distinct full binding exactly once. Never returns `SkipSuffix` — the
/// skip is illegal under aggregation (any new bound variable multiplies
/// the binding set the fold is defined over). The illegality is also
/// encoded structurally: aggregate plans mark every node sink-relevant
/// (hardening PRD 05; run.rs's skip-absorption arm), so even a skip
/// signaled by mistake would be absorbed at its producing node.
#[derive(Debug)]
pub struct AggregateSink {
    finds: Vec<FindSpec>,
    /// Group-key slots (the `Var` specs, in find order).
    group_slots: Vec<usize>,
    /// Group key words -> accumulator row index.
    groups: WordMap<usize>,
    /// Flat accumulator rows: `accs[group * n_aggs ..][..n_aggs]`.
    accs: Vec<Acc>,
    n_aggs: usize,
    /// Full-binding dedup, elided when the plan proves distinct bindings.
    seen: Option<WordMap<()>>,
    key_scratch: Vec<u64>,
    binding_scratch: Vec<u64>,
}

impl AggregateSink {
    /// Builds the sink. `slot_count` is the plan's binding-slot count;
    /// `distinct_bindings` is the plan's elision flag (30-execution): when
    /// set, the seen-set is skipped entirely.
    #[must_use]
    pub fn new(finds: Vec<FindSpec>, slot_count: usize, distinct_bindings: bool) -> Self {
        let group_slots: Vec<usize> = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Var { slot } => Some(*slot),
                FindSpec::Agg { .. } => None,
            })
            .collect();
        let n_aggs = finds.len() - group_slots.len();
        Self {
            groups: WordMap::new(group_slots.len()),
            key_scratch: vec![0; group_slots.len()],
            binding_scratch: vec![0; slot_count],
            seen: (!distinct_bindings).then(|| WordMap::new(slot_count)),
            group_slots,
            finds,
            accs: Vec::new(),
            n_aggs,
        }
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.groups.clear();
        self.accs.clear();
        if let Some(seen) = &mut self.seen {
            seen.clear();
        }
    }

    /// Finalizes each group's row (values in find order) into `emit`,
    /// assembling rows in a caller-reused scratch. Sums are range-checked
    /// here, once — deterministic by construction (i128 cannot overflow
    /// summing fewer than 2^64 i64 terms). Empty input yields zero rows: a
    /// global aggregate over nothing is the empty set, not a 0 or NULL row.
    ///
    /// # Errors
    ///
    /// `Overflow` when a Sum's final value exceeds its result type; errors
    /// from `emit` propagate.
    pub fn finalize_into(
        &self,
        row_scratch: &mut Vec<u64>,
        mut emit: impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        for (key, group_idx) in self.groups.iter() {
            let accs = &self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
            row_scratch.clear();
            let mut key_cursor = 0;
            let mut acc_cursor = 0;
            for (find_idx, find) in self.finds.iter().enumerate() {
                match find {
                    FindSpec::Var { .. } => {
                        row_scratch.push(key[key_cursor]);
                        key_cursor += 1;
                    }
                    FindSpec::Agg { .. } => {
                        row_scratch.push(finalize(accs[acc_cursor], find_idx)?);
                        acc_cursor += 1;
                    }
                }
            }
            emit(row_scratch)?;
        }
        Ok(())
    }

    /// Convenience finalization into fresh vectors (tests).
    ///
    /// # Errors
    ///
    /// As [`Self::finalize_into`].
    #[cfg(test)]
    pub fn into_rows(self) -> Result<Vec<Vec<u64>>> {
        let mut rows = Vec::with_capacity(self.groups.len());
        let mut scratch = Vec::new();
        self.finalize_into(&mut scratch, |row| {
            rows.push(row.to_vec());
            Ok(())
        })?;
        Ok(rows)
    }
}

/// Range-checks and word-encodes one accumulator.
fn finalize(acc: Acc, find_idx: usize) -> Result<u64> {
    match acc {
        Acc::SumSigned(total) => i64::try_from(total)
            .map(i64_to_word)
            .map_err(|_| Error::Overflow { find: find_idx }),
        Acc::SumUnsigned(total) => {
            u64::try_from(total).map_err(|_| Error::Overflow { find: find_idx })
        }
        Acc::Min(word) | Acc::Max(word) | Acc::Count(word) => Ok(word),
    }
}

impl AggregateSink {
    /// Folds the full binding currently in `binding_scratch`: dedup
    /// (unless elided), group resolution, accumulator update. Both emit
    /// paths land here — the scratch row is the one representation.
    fn fold_scratch_row(&mut self) {
        // Binding dedup: fold only the first occurrence of each distinct
        // full binding — unless the plan proved distinctness (elision).
        if let Some(seen) = &mut self.seen {
            if !seen.insert(&self.binding_scratch) {
                return;
            }
        }

        for (i, slot) in self.group_slots.iter().enumerate() {
            self.key_scratch[i] = self.binding_scratch[*slot];
        }
        let (group_idx, inserted) = {
            let next = self.groups.len();
            let (idx, inserted) = self.groups.get_or_insert_with(&self.key_scratch, || next);
            (*idx, inserted)
        };
        if inserted {
            // Fresh accumulator row, seeded per op.
            for find in &self.finds {
                if let FindSpec::Agg { op, signed, .. } = find {
                    self.accs.push(match (op, signed) {
                        (AggOp::Sum, true) => Acc::SumSigned(0),
                        (AggOp::Sum, false) => Acc::SumUnsigned(0),
                        (AggOp::Min, _) => Acc::Min(u64::MAX),
                        (AggOp::Max, _) => Acc::Max(u64::MIN),
                        (AggOp::Count, _) => Acc::Count(0),
                    });
                }
            }
        }

        let accs = &mut self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
        let mut acc_cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg {
                op,
                over_slot,
                signed,
            } = find
            else {
                continue;
            };
            let acc = &mut accs[acc_cursor];
            acc_cursor += 1;
            match (op, acc) {
                (AggOp::Count, Acc::Count(n)) => *n += 1,
                (AggOp::Sum, Acc::SumSigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    debug_assert!(*signed);
                    *total += i128::from(word_to_i64(word));
                }
                (AggOp::Sum, Acc::SumUnsigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    *total += u128::from(word);
                }
                (AggOp::Min, Acc::Min(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Min has a variable")];
                    *best = (*best).min(word);
                }
                (AggOp::Max, Acc::Max(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Max has a variable")];
                    *best = (*best).max(word);
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
    }
}

impl Sink for AggregateSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..bindings.slot_count() {
            self.binding_scratch[slot] = bindings.get(slot);
        }
        self.fold_scratch_row();
        Flow::Continue
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        // Aggregate plans mark every node sink-relevant (hardening
        // PRD 05), so the executor never asks a fold to stop on skip.
        debug_assert!(!stop_on_skip, "folds never stop on skip");
        // Outer slots are constant across the batch: prefill once; the
        // row loop overwrites only the leaf's key slots.
        for slot in 0..self.binding_scratch.len() {
            if matches!(batch.source_of(slot), LeafSource::Outer) {
                self.binding_scratch[slot] = batch.bindings.get(slot);
            }
        }
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                self.binding_scratch[*slot] = batch.key(entry, word);
            }
            self.fold_scratch_row();
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::exec::colt::Colt;
    use crate::exec::run::{Counters, Executor};
    use crate::image::view::apply;
    use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence};
    use crate::ir::VarId;
    use crate::plan::fj::{binary2fj, factor, validate, ValidatedPlan};
    use crate::plan::planner::JoinOrder;
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema,
        SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    /// Posting(id serial u64, account u64, amount i64) +
    /// PostingTag(posting u64, tag u64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Posting".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Serial,
                        },
                        FieldDescriptor {
                            name: "account".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "amount".into(),
                            value_type: ValueType::I64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "PostingTag".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "posting".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "tag".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                },
            ],
        }
        .validate()
        .expect("valid fixture")
    }

    const POSTING: RelationId = RelationId(0);
    const TAG: RelationId = RelationId(1);

    fn views_of(
        dir: &TempDir,
        schema: &Schema,
        postings: &[(u64, u64, i64)],
        tags: &[(u64, u64)],
    ) -> Vec<Arc<crate::image::RelationImage>> {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (id, account, amount) in postings {
            let mut bytes = Vec::new();
            encode_fact(
                &[
                    ValueRef::U64(*id),
                    ValueRef::U64(*account),
                    ValueRef::I64(*amount),
                ],
                schema.relation(POSTING).layout(),
                &mut bytes,
            );
            delta.insert(&view, POSTING, &bytes).expect("insert");
        }
        for (posting, tag) in tags {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(*posting), ValueRef::U64(*tag)],
                schema.relation(TAG).layout(),
                &mut bytes,
            );
            delta.insert(&view, TAG, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        [POSTING, TAG]
            .iter()
            .map(|rel| crate::image::build(&txn, schema, *rel).expect("build"))
            .collect()
    }

    fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
        plan.occurrences()
            .iter()
            .map(|occurrence| {
                let columns: Vec<Vec<usize>> = occurrence
                    .trie_schema
                    .iter()
                    .map(|level| {
                        level
                            .iter()
                            .map(|var| {
                                let (field, _) = occurrence
                                    .vars
                                    .iter()
                                    .find(|(_, v)| v == var)
                                    .expect("plan vars");
                                usize::from(field.0)
                            })
                            .collect()
                    })
                    .collect();
                Colt::new(
                    apply(
                        &images[usize::try_from(occurrence.relation.0).expect("small")],
                        &[],
                        &[],
                        Vec::new(),
                    ),
                    &[],
                    columns,
                )
            })
            .collect()
    }

    fn occurrence(occ: u16, relation: RelationId, vars: &[(u16, u16)]) -> Occurrence {
        Occurrence {
            occ_id: OccId(occ),
            relation,
            vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
            filters: vec![],
        }
    }

    fn planned(
        normalized: &NormalizedQuery,
        schema: &Schema,
        order: &[u16],
        sink_vars: &[u16],
    ) -> ValidatedPlan {
        let join_order = JoinOrder {
            order: order.iter().map(|o| OccId(*o)).collect(),
            estimates: vec![0; order.len()],
        };
        let mut plan = binary2fj(normalized, &join_order);
        factor(&mut plan);
        let sinks: BTreeSet<VarId> = sink_vars.iter().map(|v| VarId(*v)).collect();
        validate(&plan, normalized, schema, vec![0; order.len()], &sinks).expect("valid plan")
    }

    fn run_aggregate(
        plan: &ValidatedPlan,
        views: &[Arc<crate::image::RelationImage>],
        finds: Vec<FindSpec>,
    ) -> Result<Vec<Vec<u64>>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds, plan.slots().len(), plan.distinct_bindings());
        Executor::new(plan).execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
        let mut rows = sink.into_rows()?;
        rows.sort_unstable();
        Ok(rows)
    }

    /// Counters recording D2 skips.
    #[derive(Default)]
    struct SkipCounter {
        skips: usize,
    }

    impl Counters for SkipCounter {
        fn batch(&mut self, _: usize, _: usize) {}
        fn node_entry(&mut self, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {
            self.skips += 1;
        }
    }

    #[test]
    fn duplicate_witness_projection_dedups_and_skips_suffixes() {
        let dir = TempDir::new("sink-projection-skip");
        let schema = schema();
        // One posting, many tags: projecting only the account, the tag
        // suffix multiplies witnesses without changing the projection.
        // The tag node is the LEAF and is not sink-relevant: at batch
        // size 128 all 50 tags arrive in one leaf batch and the batch
        // emit must stop at the first row (PRD 01's stop_on_skip) — the
        // same skip the recursive path signaled per-row.
        let postings = vec![(1u64, 7u64, 100i64)];
        let tags: Vec<(u64, u64)> = (0..50).map(|t| (1, t)).collect();
        let views = views_of(&dir, &schema, &postings, &tags);
        // Q(account) :- Posting(id=p, account=a), PostingTag(posting=p).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, POSTING, &[(0, 0), (1, 1)]),
                occurrence(1, TAG, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        // Sink-relevant vars: just the account (var 1).
        let plan = planned(&normalized, &schema, &[0, 1], &[1]);
        for batch in [1usize, 2, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1))]);
            let mut counters = SkipCounter::default();
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut counters,
            );

            let rows: Vec<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
            assert_eq!(rows, vec![vec![7]], "batch {batch}");
            assert!(
                counters.skips > 0,
                "batch {batch}: the tag suffix must be skipped after the first witness"
            );
        }
    }

    /// PRD 01 (docs/perf/): the aggregate leaf batch folds bit-identically
    /// to the scalar degenerate case at every batch size, including the
    /// deterministic-overflow class at the i64 boundary.
    #[test]
    fn aggregate_leaf_batches_match_the_scalar_fold_at_the_boundary() {
        let dir = TempDir::new("sink-batch-boundary");
        let schema = schema();
        // Account 7 sums to exactly i64::MAX (in range); account 8
        // overflows deterministically.
        let postings = vec![
            (1u64, 7u64, i64::MAX),
            (2, 7, 1),
            (3, 7, -2),
            (4, 7, 1),
            (5, 8, i64::MAX),
            (6, 8, 1),
        ];
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[1]);
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(1)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Count,
                    over_slot: None,
                    signed: false,
                },
            ]
        };
        for batch in [1usize, 2, 7, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), true);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            // Account 8's Sum overflows: the error is deterministic and
            // carries the find index, at every batch size.
            let err = sink.into_rows().unwrap_err();
            assert!(
                matches!(err, Error::Overflow { find: 1 }),
                "batch {batch}: {err:?}"
            );
        }
        // Remove the overflowing account: values identical at every size.
        let dir2 = TempDir::new("sink-batch-boundary-ok");
        let views = views_of(&dir2, &schema, &postings[..4], &[]);
        let mut reference: Option<Vec<Vec<u64>>> = None;
        for batch in [1usize, 2, 7, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), true);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            let mut rows = sink.into_rows().expect("in range");
            rows.sort_unstable();
            assert_eq!(
                rows,
                vec![vec![7, i64_to_word(i64::MAX), 4]],
                "batch {batch}"
            );
            match &reference {
                None => reference = Some(rows),
                Some(r) => assert_eq!(*r, rows, "batch {batch}"),
            }
        }
    }

    #[test]
    fn sum_distinguishes_bound_serials_and_collapses_unbound_ones() {
        let dir = TempDir::new("sink-footgun");
        let schema = schema();
        // Two postings of amount 100 to account 7.
        let postings = vec![(1u64, 7u64, 100i64), (2, 7, 100)];
        let views = views_of(&dir, &schema, &postings, &[]);

        // Serials bound: two distinct bindings -> Sum = 200.
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[1]);
        let finds = vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
        ];
        let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
        assert_eq!(rows, vec![vec![7, i64_to_word(200)]]);

        // Serials unbound: the two facts collapse to one binding -> 100.
        // This documents the set-semantics footgun deliberately.
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[0]);
        let finds = vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(0)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(1))),
                signed: true,
            },
        ];
        let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
        assert_eq!(rows, vec![vec![7, i64_to_word(100)]]);
    }

    #[test]
    fn joining_a_three_tag_relation_triples_the_sum() {
        let dir = TempDir::new("sink-tag-triple");
        let schema = schema();
        let postings = vec![(1u64, 7u64, 100i64)];
        let tags = vec![(1u64, 10u64), (1, 11), (1, 12)];
        let views = views_of(&dir, &schema, &postings, &tags);
        // Sum(amount) by account joined with tags: the 3 tag bindings
        // multiply the binding set — exactly the documented footgun.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)]),
                occurrence(1, TAG, &[(0, 0), (1, 3)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1], &[1]);
        let finds = vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
        ];
        let rows = run_aggregate(&plan, &views, finds).expect("rows");
        assert_eq!(rows, vec![vec![7, i64_to_word(300)]]);
    }

    #[test]
    fn distinct_flag_elision_matches_the_seen_set_path() {
        let dir = TempDir::new("sink-elision");
        let schema = schema();
        let postings = vec![(1u64, 7u64, 10i64), (2, 7, 20), (3, 8, 30)];
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[1]);
        assert!(plan.distinct_bindings(), "serials are bound");
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(1)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
            ]
        };

        // Elided path (as the plan proves) vs forced seen-set path.
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut elided = AggregateSink::new(finds(&plan), plan.slots().len(), true);
        Executor::new(&plan).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut elided,
            &mut crate::exec::run::NoopCounters,
        );
        let mut colts = colts_for(&plan, &views);
        let mut checked = AggregateSink::new(finds(&plan), plan.slots().len(), false);
        Executor::new(&plan).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut checked,
            &mut crate::exec::run::NoopCounters,
        );
        let mut a = elided.into_rows().expect("rows");
        let mut b = checked.into_rows().expect("rows");
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn global_aggregate_over_empty_input_yields_zero_rows() {
        let dir = TempDir::new("sink-empty-global");
        let schema = schema();
        let views = views_of(&dir, &schema, &[], &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (2, 1)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[]);
        let finds = vec![
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(1))),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Count,
                over_slot: None,
                signed: false,
            },
        ];
        let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
        // The empty set — not a [NULL] or [0] row (documented divergence
        // from SQL's ungrouped-aggregate behavior).
        assert!(rows.is_empty());
    }

    #[test]
    fn sum_is_order_independent_near_the_boundary() {
        // {i64::MAX, 1, -2} sums to MAX-1 under any fold order thanks to
        // i128 accumulation; {MAX, 1} overflows deterministically.
        for order in [[0usize, 1, 2], [2, 1, 0], [1, 2, 0]] {
            let values = [i64::MAX, 1, -2];
            let mut sink = AggregateSink::new(
                vec![FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(0),
                    signed: true,
                }],
                1,
                true,
            );
            let mut bindings = Bindings::new(1);
            bindings.reset();
            for idx in order {
                bindings.set(0, i64_to_word(values[idx]));
                assert_eq!(sink.emit(&bindings), Flow::Continue);
            }
            let rows = sink.into_rows().expect("in range");
            assert_eq!(rows, vec![vec![i64_to_word(i64::MAX - 1)]]);
        }
        for order in [[0usize, 1], [1, 0]] {
            let values = [i64::MAX, 1];
            let mut sink = AggregateSink::new(
                vec![FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(0),
                    signed: true,
                }],
                1,
                true,
            );
            let mut bindings = Bindings::new(1);
            bindings.reset();
            for idx in order {
                bindings.set(0, i64_to_word(values[idx]));
                sink.emit(&bindings);
            }
            let err = sink.into_rows().unwrap_err();
            assert!(matches!(err, Error::Overflow { find: 0 }), "{err:?}");
        }
    }

    #[test]
    fn min_and_max_honor_logical_i64_order_across_the_sign_boundary() {
        let mut sink = AggregateSink::new(
            vec![
                FindSpec::Agg {
                    op: AggOp::Min,
                    over_slot: Some(0),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Max,
                    over_slot: Some(0),
                    signed: true,
                },
            ],
            1,
            true,
        );
        let mut bindings = Bindings::new(1);
        bindings.reset();
        for v in [-5i64, 3, -100, 42, 0] {
            bindings.set(0, i64_to_word(v));
            sink.emit(&bindings);
        }
        let rows = sink.into_rows().expect("rows");
        assert_eq!(rows, vec![vec![i64_to_word(-100), i64_to_word(42)]]);
    }
}
