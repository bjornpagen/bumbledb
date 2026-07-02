//! The recursive Free Join executor (PRDs 19 + 21) — vectorized execution
//! is the default and only path; batch size 1 is merely its degenerate
//! setting, never a mode (`docs/architecture/30-execution.md` D4,
//! post-mortem §31; paper §3.3 Fig. 5, §4.3).
//!
//! Everything is a monomorphized generic — no `dyn` anywhere in the hot
//! path. Per node entry: choose the cover by labeled key count, iterate it
//! in batches, two-phase-probe each sibling (phase 1 computes every hash —
//! pure ALU; phase 2 issues all bucket loads — independent chains the
//! out-of-order window overlaps), compact survivors branchlessly, evaluate residuals as
//! batch compaction, then recurse per surviving element with the scalar
//! journal discipline.
//!
//! Honest caveat, stated (D4): deep in the plan the batch source is the
//! current subtrie, whose fanout on FK walks is often 1-10 — large batches
//! are reliably available only at the root; cross-node-entry accumulation
//! is future work, not assumed.

use crate::exec::colt::{BatchToken, Colt, Cursor, KeyCount};
use crate::ir::normalize::PlacedComparison;
use crate::plan::fj::ValidatedPlan;

/// The sink's reply to one emitted binding: `SkipSuffix` requests the D2
/// subtree skip (legal only for the projection sink; the executor enforces
/// the plan's per-node sink-relevance bits, the sink just reports
/// staleness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    SkipSuffix,
}

/// Consumes complete bindings (D3: the executor emits to a sink, never an
/// `output()`).
pub trait Sink {
    fn emit(&mut self, bindings: &Bindings) -> Flow;
}

/// Execution observability seam (30-execution): the normal path
/// instantiates [`NoopCounters`] — zero-sized, compiled to nothing; the
/// EXPLAIN entry point (PRD 24) instantiates the counting variant.
pub trait Counters {
    fn node_entry(&mut self, node: usize);
    /// A cover was chosen: which subatom, and whether its count was Exact.
    fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool);
    /// Phase 1 computed one probe hash (ordering assertions: every hash of
    /// a batch precedes its first probe).
    fn probe_hash(&mut self, node: usize, subatom: usize);
    fn probe(&mut self, node: usize, subatom: usize, hit: bool);
    fn residual(&mut self, node: usize, pass: bool);
    fn emit(&mut self);
    /// A D2 subtree skip propagated through this node.
    fn skip(&mut self, node: usize);
}

/// The release-path counters: every method compiles to nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCounters;

impl Counters for NoopCounters {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn probe_hash(&mut self, _: usize, _: usize) {}
    #[inline]
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn residual(&mut self, _: usize, _: bool) {}
    #[inline]
    fn emit(&mut self) {}
    #[inline]
    fn skip(&mut self, _: usize) {}
}

/// Dense slot-indexed binding array with an epoch discipline instead of
/// `Option` (branch-light: stale slots are never read — reads are
/// plan-scoped — the epoch exists for debug assertions).
#[derive(Debug)]
pub struct Bindings {
    slots: Vec<u64>,
    epochs: Vec<u64>,
    current: u64,
}

impl Bindings {
    #[must_use]
    pub fn new(slot_count: usize) -> Self {
        Self {
            slots: vec![0; slot_count],
            epochs: vec![0; slot_count],
            current: 0,
        }
    }

    /// Starts a fresh execution: every slot becomes stale at once.
    pub fn reset(&mut self) {
        self.current += 1;
    }

    pub fn set(&mut self, slot: usize, value: u64) {
        self.slots[slot] = value;
        self.epochs[slot] = self.current;
    }

    /// Reads a bound slot.
    #[must_use]
    pub fn get(&self, slot: usize) -> u64 {
        debug_assert_eq!(
            self.epochs[slot], self.current,
            "reads are plan-scoped: slot {slot} must be bound"
        );
        self.slots[slot]
    }

    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }
}

/// The starting batch size: sized so ~28 MLP lanes see >=28 independent
/// probes in flight with bookkeeping amortized over several waves (D4's
/// model). The exact number is measurement-owned (OPEN, architecture
/// README) — this is the one place it lives.
pub const BATCH: usize = 128;

/// Where a value read during batched probing comes from: a word of the
/// current batch's cover keys (varying per element) or an already-bound
/// outer slot (constant across the batch).
#[derive(Debug, Clone, Copy)]
enum Source {
    Batch(usize),
    Slot(usize),
}

/// Per-node reusable scratch: each node's frame is active at most once in
/// the recursion (frames advance strictly by node index), so scratch is
/// indexed by node and allocated once per executor construction.
struct NodeScratch {
    /// Cover-entry key words, entry-major (`entry * arity + word`).
    entry_keys: Vec<u64>,
    /// Cover-entry child cursors.
    children: Vec<Cursor>,
    /// Surviving batch-entry indices (branchlessly compacted).
    survivors: Vec<u32>,
    /// Phase-1 gathered probe keys, entry-major per sibling pass.
    probe_keys: Vec<u64>,
    /// Phase-1 hashes, aligned with `survivors`.
    hashes: Vec<u64>,
    /// Per subatom, per entry: the probed child cursor.
    sibling_children: Vec<Vec<Cursor>>,
    /// Per sibling-var value sources, recomputed per node entry (the
    /// runtime cover choice decides what comes from the batch).
    sources: Vec<Vec<Source>>,
    /// Residual operand sources, aligned with the node's residual list.
    residual_sources: Vec<(Source, Source)>,
    /// Per-entry survivor mask for the compaction kernel.
    mask: Vec<u8>,
    /// Undo journal: (occurrence index, previous cursor, previous level).
    journal: Vec<(usize, Cursor, usize)>,
}

/// The executor scratch for one plan shape: per-execution cursor state and
/// per-node buffers, sized once at construction. It does not borrow the
/// plan — the same `&ValidatedPlan` is passed to [`Executor::execute`]
/// (the prepared query owns both, PRD 25).
pub struct Executor {
    batch: usize,
    /// Per occurrence: (current cursor, current trie level).
    cursors: Vec<(Cursor, usize)>,
    /// Per subatom slot maps, precomputed: `slot_map[node][subatom][i]` is
    /// the binding slot of that subatom's i-th variable.
    slot_map: Vec<Vec<Vec<usize>>>,
    /// Per residual: (lhs slot, rhs slot), aligned with each node's list.
    residual_slots: Vec<Vec<(PlacedComparison, usize, usize)>>,
    scratch: Vec<NodeScratch>,
}

impl Executor {
    /// An executor with the default batch size ([`BATCH`]).
    #[must_use]
    pub fn new(plan: &ValidatedPlan) -> Self {
        Self::with_batch_size(plan, BATCH)
    }

    /// An executor with an explicit batch size — the scalar/vectorized
    /// equality tests parameterize this; there is no mode, only the number.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: a zero batch size.
    #[must_use]
    pub fn with_batch_size(plan: &ValidatedPlan, batch: usize) -> Self {
        assert!(batch > 0, "a batch has at least one element");
        let slot_map: Vec<Vec<Vec<usize>>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.subatoms
                    .iter()
                    .map(|s| s.vars.iter().map(|v| plan.slot_of(*v)).collect())
                    .collect()
            })
            .collect();
        let residual_slots: Vec<Vec<(PlacedComparison, usize, usize)>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.residuals
                    .iter()
                    .map(|r| (*r, plan.slot_of(r.lhs), plan.slot_of(r.rhs)))
                    .collect()
            })
            .collect();
        let scratch = plan
            .nodes()
            .iter()
            .map(|node| {
                let max_arity = node
                    .subatoms
                    .iter()
                    .map(|s| s.vars.len())
                    .max()
                    .unwrap_or(0)
                    .max(1);
                NodeScratch {
                    entry_keys: vec![0; batch * max_arity],
                    children: vec![Cursor::Row(0); batch],
                    survivors: Vec::with_capacity(batch),
                    probe_keys: vec![0; batch * max_arity],
                    hashes: Vec::with_capacity(batch),
                    sibling_children: node
                        .subatoms
                        .iter()
                        .map(|_| vec![Cursor::Row(0); batch])
                        .collect(),
                    sources: node.subatoms.iter().map(|_| Vec::new()).collect(),
                    residual_sources: Vec::new(),
                    mask: Vec::with_capacity(batch),
                    journal: Vec::new(),
                }
            })
            .collect();
        Self {
            batch,
            cursors: Vec::new(),
            slot_map,
            residual_slots,
            scratch,
        }
    }

    /// Runs the plan over the COLT sources (one per occurrence, indexed by
    /// occurrence id), emitting complete bindings to the sink.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (sources not matching the
    /// plan's occurrences).
    pub fn execute<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        colts: &mut [Colt<'_>],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        assert_eq!(colts.len(), plan.occurrences().len());
        debug_assert_eq!(plan.nodes().len(), self.scratch.len(), "same plan shape");
        bindings.reset();
        self.cursors.clear();
        self.cursors
            .extend(colts.iter().map(|colt| (colt.root(), 0usize)));
        self.run_node(plan, 0, colts, bindings, sink, counters);
    }

    #[allow(clippy::too_many_lines)] // the one hot loop; splitting it would
                                     // scatter the batch invariants the comments walk through in order
    fn run_node<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt<'_>],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        if node_idx == plan.nodes().len() {
            counters.emit();
            return sink.emit(bindings);
        }
        counters.node_entry(node_idx);

        // Dynamic cover choice (§4.4): prefer the smallest Exact, else the
        // smallest Estimate — the labels are load-bearing and never
        // compared as the same quantity (post-mortem §40).
        let cover_sub = self.choose_cover(plan, node_idx, colts);
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        let (cover_cursor, cover_level) = self.cursors[cover_occ];
        counters.cover_choice(
            node_idx,
            cover_sub,
            matches!(colts[cover_occ].key_count(cover_cursor), KeyCount::Exact(_)),
        );

        let arity = node.subatoms[cover_sub].vars.len();
        let mut scratch = std::mem::replace(
            &mut self.scratch[node_idx],
            NodeScratch {
                entry_keys: Vec::new(),
                children: Vec::new(),
                survivors: Vec::new(),
                probe_keys: Vec::new(),
                hashes: Vec::new(),
                sibling_children: Vec::new(),
                sources: Vec::new(),
                residual_sources: Vec::new(),
                mask: Vec::new(),
                journal: Vec::new(),
            },
        );

        // Resolve value sources against the runtime cover choice: a var
        // bound by the chosen cover reads the batch key column; everything
        // else reads its (already bound) outer slot.
        let cover_vars = &plan.nodes()[node_idx].subatoms[cover_sub].vars;
        for (sub_idx, subatom) in plan.nodes()[node_idx].subatoms.iter().enumerate() {
            scratch.sources[sub_idx].clear();
            for (i, var) in subatom.vars.iter().enumerate() {
                let source = cover_vars.iter().position(|cv| cv == var).map_or(
                    Source::Slot(self.slot_map[node_idx][sub_idx][i]),
                    Source::Batch,
                );
                scratch.sources[sub_idx].push(source);
            }
        }
        scratch.residual_sources.clear();
        for (residual, lhs_slot, rhs_slot) in &self.residual_slots[node_idx] {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                cover_vars
                    .iter()
                    .position(|cv| *cv == var)
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.residual_sources.push((
                resolve(residual.lhs, *lhs_slot),
                resolve(residual.rhs, *rhs_slot),
            ));
        }

        let mut token = BatchToken::default();
        let mut flow = Flow::Continue;

        'outer: loop {
            let (yielded, next_token) = colts[cover_occ].iter_batch(
                cover_cursor,
                cover_level,
                token,
                &mut scratch.entry_keys,
                &mut scratch.children,
                self.batch,
            );
            if yielded == 0 {
                break;
            }
            token = next_token;
            scratch.survivors.clear();
            scratch
                .survivors
                .extend(0..u32::try_from(yielded).expect("batch fits u32"));

            // Per sibling: the two-phase probe, then branchless compaction.
            let value_of = |sources: &[Source],
                            entry_keys: &[u64],
                            bindings: &Bindings,
                            entry: usize,
                            i: usize| match sources[i] {
                Source::Batch(word) => entry_keys[entry * arity + word],
                Source::Slot(slot) => bindings.get(slot),
            };
            for sub_idx in 0..plan.nodes()[node_idx].subatoms.len() {
                if sub_idx == cover_sub || scratch.survivors.is_empty() {
                    continue;
                }
                let subatom = &plan.nodes()[node_idx].subatoms[sub_idx];
                let sub_arity = subatom.vars.len();
                let occ = usize::from(subatom.occ.0);
                let (s_cursor, s_level) = self.cursors[occ];
                colts[occ].ensure_forced(s_cursor, s_level);

                // Phase 1: gather every probe key and compute every hash —
                // pure ALU, no bucket loads.
                scratch.hashes.clear();
                for (k, &e) in scratch.survivors.iter().enumerate() {
                    let entry = usize::try_from(e).expect("batch fits usize");
                    for i in 0..sub_arity {
                        scratch.probe_keys[k * sub_arity + i] = value_of(
                            &scratch.sources[sub_idx],
                            &scratch.entry_keys,
                            bindings,
                            entry,
                            i,
                        );
                    }
                    counters.probe_hash(node_idx, sub_idx);
                    scratch.hashes.push(crate::exec::colt::hash_key(
                        &scratch.probe_keys[k * sub_arity..(k + 1) * sub_arity],
                    ));
                }

                // Phase 2: all bucket loads — independent chains the
                // out-of-order window overlaps — then kernel compaction.
                scratch.mask.clear();
                for k in 0..scratch.survivors.len() {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let hit = colts[occ].get_prehashed(
                        s_cursor,
                        s_level,
                        &scratch.probe_keys[k * sub_arity..(k + 1) * sub_arity],
                        scratch.hashes[k],
                    );
                    counters.probe(node_idx, sub_idx, hit.is_some());
                    scratch.sibling_children[sub_idx][entry] = hit.unwrap_or(Cursor::Row(0));
                    scratch.mask.push(u8::from(hit.is_some()));
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }

            // Residuals run as batch survivor compaction after the probes.
            for (r_idx, (lhs_src, rhs_src)) in scratch.residual_sources.iter().enumerate() {
                let op = self.residual_slots[node_idx][r_idx].0.op;
                scratch.mask.clear();
                for k in 0..scratch.survivors.len() {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word],
                        Source::Slot(slot) => bindings.get(slot),
                    };
                    let pass = op.compare(&value(lhs_src), &value(rhs_src));
                    counters.residual(node_idx, pass);
                    scratch.mask.push(u8::from(pass));
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }

            // Recurse per surviving element (paper §4.3: batch within a
            // node, recurse per tuple) with the scalar journal discipline.
            for k in 0..scratch.survivors.len() {
                let entry = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    bindings.set(*slot, scratch.entry_keys[entry * arity + i]);
                }
                scratch.journal.clear();
                scratch.journal.push((cover_occ, cover_cursor, cover_level));
                self.cursors[cover_occ] = (scratch.children[entry], cover_level + 1);
                for (sub_idx, subatom) in plan.nodes()[node_idx].subatoms.iter().enumerate() {
                    if sub_idx == cover_sub {
                        continue;
                    }
                    let occ = usize::from(subatom.occ.0);
                    let (cursor, level) = self.cursors[occ];
                    scratch.journal.push((occ, cursor, level));
                    self.cursors[occ] = (scratch.sibling_children[sub_idx][entry], level + 1);
                }

                flow = self.run_node(plan, node_idx + 1, colts, bindings, sink, counters);

                for (occ, cursor, level) in scratch.journal.drain(..).rev() {
                    self.cursors[occ] = (cursor, level);
                }

                if flow == Flow::SkipSuffix {
                    if plan.nodes()[node_idx].sink_relevant {
                        // This node binds a projected variable: absorb the
                        // skip — later entries change the output.
                        flow = Flow::Continue;
                    } else {
                        // The suffix from here binds nothing sink-relevant:
                        // propagate the unwind (D2).
                        counters.skip(node_idx);
                        break 'outer;
                    }
                }
            }
        }

        self.scratch[node_idx] = scratch;
        flow
    }

    /// Chooses the cover with the fewest keys: smallest `Exact` wins;
    /// otherwise the smallest `Estimate` (v0 rule, 30-execution).
    fn choose_cover(&self, plan: &ValidatedPlan, node_idx: usize, colts: &[Colt<'_>]) -> usize {
        let node = &plan.nodes()[node_idx];
        let mut best: Option<(usize, KeyCount)> = None;
        for &cover in &node.covers {
            let sub_idx = usize::from(cover);
            let occ = usize::from(node.subatoms[sub_idx].occ.0);
            let count = colts[occ].key_count(self.cursors[occ].0);
            let better = match (&best, count) {
                // An Exact always displaces an Estimate; never vice versa.
                (None, _) | (Some((_, KeyCount::Estimate(_))), KeyCount::Exact(_)) => true,
                (Some((_, KeyCount::Exact(_))), KeyCount::Estimate(_)) => false,
                (Some((_, KeyCount::Exact(b))), KeyCount::Exact(n))
                | (Some((_, KeyCount::Estimate(b))), KeyCount::Estimate(n)) => n < *b,
            };
            if better {
                best = Some((sub_idx, count));
            }
        }
        best.expect("validated plans have non-empty cover sets").0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::image::view::{apply, View};
    use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence, PlacedComparison};
    use crate::ir::{CmpOp, VarId};
    use crate::plan::fj::{binary2fj, factor, validate, ValidatedPlan};
    use crate::plan::planner::JoinOrder;
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema,
        SchemaDescriptor,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    /// A sink collecting distinct full binding tuples (set semantics).
    #[derive(Default)]
    struct CollectSink {
        rows: BTreeSet<Vec<u64>>,
    }

    impl Sink for CollectSink {
        fn emit(&mut self, bindings: &Bindings) -> Flow {
            let row: Vec<u64> = (0..bindings.slot_count())
                .map(|s| bindings.get(s))
                .collect();
            self.rows.insert(row);
            Flow::Continue
        }
    }

    /// Counters recording cover choices for the skew assertion.
    #[derive(Default)]
    struct RecordingCounters {
        cover_choices: Vec<(usize, usize, bool)>,
    }

    impl Counters for RecordingCounters {
        fn node_entry(&mut self, _: usize) {}
        fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool) {
            self.cover_choices.push((node, subatom, exact));
        }
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    /// Builds a schema of binary U64 relations R0..Rn(a, b).
    fn schema(relations: usize) -> Schema {
        SchemaDescriptor {
            relations: (0..relations)
                .map(|r| RelationDescriptor {
                    name: format!("R{r}").into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "a".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "b".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                })
                .collect(),
        }
        .validate()
        .expect("valid fixture")
    }

    /// Commits word rows into each relation and returns unfiltered views.
    fn views_of(dir: &TempDir, schema: &Schema, data: &[Vec<(u64, u64)>]) -> Vec<Arc<View>> {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (rel, rows) in data.iter().enumerate() {
            let rel_id = RelationId(u32::try_from(rel).expect("small"));
            for (a, b) in rows {
                let mut bytes = Vec::new();
                encode_fact(
                    &[ValueRef::U64(*a), ValueRef::U64(*b)],
                    schema.relation(rel_id).layout(),
                    &mut bytes,
                );
                delta.insert(&view, rel_id, &bytes).expect("insert");
            }
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        (0..data.len())
            .map(|rel| {
                let rel_id = RelationId(u32::try_from(rel).expect("small"));
                let image = crate::image::build(&txn, schema, rel_id).expect("build");
                Arc::new(apply(&image, &[], &[], Vec::new()))
            })
            .collect()
    }

    /// COLT sources for a plan: schema columns from each occurrence's trie
    /// schema and var-to-field map.
    fn colts_for<'v>(plan: &ValidatedPlan, views: &'v [Arc<View>]) -> Vec<Colt<'v>> {
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
                                    .expect("plan vars come from the occurrence");
                                usize::from(field.0)
                            })
                            .collect()
                    })
                    .collect();
                Colt::new(
                    &views[usize::try_from(occurrence.relation.0).expect("small")],
                    columns,
                )
            })
            .collect()
    }

    fn occurrence(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
        Occurrence {
            occ_id: OccId(occ),
            relation: RelationId(relation),
            vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
            filters: vec![],
        }
    }

    fn planned(normalized: &NormalizedQuery, schema: &Schema, order: &[u16]) -> ValidatedPlan {
        let join_order = JoinOrder {
            order: order.iter().map(|o| OccId(*o)).collect(),
            estimates: vec![0; order.len()],
        };
        let mut plan = binary2fj(normalized, &join_order);
        factor(&mut plan);
        validate(
            &plan,
            normalized,
            schema,
            vec![0; order.len()],
            &BTreeSet::new(),
        )
        .expect("valid plan")
    }

    fn run(plan: &ValidatedPlan, views: &[Arc<View>]) -> BTreeSet<Vec<u64>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut executor = Executor::new(plan);
        executor.execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        sink.rows
    }

    /// The clover query over the paper's Fig. 4 instance: only
    /// (x0, a0, b0, c0) joins.
    #[test]
    fn clover_on_the_papers_instance() {
        let dir = TempDir::new("run-clover");
        let schema = schema(3);
        let n = 20u64;
        // R = {(x0,a0)} u {(x1,ai_l), (x2,ai_r)}; S, T rotated (Fig. 4).
        // Encode x0..x3 as 0..3 and the a/b/c values as 100+i / 200+i.
        let mut r = vec![(0, 100)];
        let mut s = vec![(0, 200)];
        let mut t = vec![(0, 300)];
        for i in 1..=n {
            r.push((1, 100 + i));
            r.push((2, 100 + n + i));
            s.push((2, 200 + i));
            s.push((3, 200 + n + i));
            t.push((3, 300 + i));
            t.push((1, 300 + n + i));
        }
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

        // Q(x,a,b,c) :- R(x,a), S(x,b), T(x,c).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
                occurrence(2, 2, &[(0, 0), (1, 3)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let results = run(&plan, &views);

        // Naive oracle: triple loop.
        let mut expected = BTreeSet::new();
        for (rx, ra) in &r {
            for (sx, sb) in &s {
                for (tx, tc) in &t {
                    if rx == sx && sx == tx {
                        expected.insert(vec![*rx, *ra, *sb, *tc]);
                    }
                }
            }
        }
        assert_eq!(results, expected);
        assert_eq!(results.len(), 1, "only the center of the clover joins");
    }

    #[test]
    fn chain_query_matches_the_nested_loop_oracle() {
        let dir = TempDir::new("run-chain");
        let schema = schema(3);
        let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i + 1)).collect();
        let s: Vec<(u64, u64)> = (0..10).map(|i| (i + 1, i + 2)).collect();
        let t: Vec<(u64, u64)> = (0..10).map(|i| (i + 2, i + 3)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

        // Q(x,y,z,w) :- R(x,y), S(y,z), T(z,w).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let results = run(&plan, &views);

        let mut expected = BTreeSet::new();
        for (rx, ry) in &r {
            for (sy, sz) in &s {
                for (tz, tw) in &t {
                    if ry == sy && sz == tz {
                        expected.insert(vec![*rx, *ry, *sz, *tw]);
                    }
                }
            }
        }
        assert_eq!(results, expected);
        assert!(!results.is_empty());
    }

    #[test]
    fn self_join_grandparent() {
        let dir = TempDir::new("run-grandparent");
        let schema = schema(1);
        // OrgParent(child, parent): 0->1->2->3 plus a fork 4->1.
        let edges = vec![(0u64, 1u64), (1, 2), (2, 3), (4, 1)];
        let views = views_of(&dir, &schema, std::slice::from_ref(&edges));

        // Grandparent(c, g) :- OrgParent(c, p), OrgParent(p, g) — two
        // occurrences of relation 0.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 0, &[(0, 1), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        // Both occurrences read relation 0: views vector must be indexed by
        // occurrence, not relation — build colts by occurrence's relation.
        let results = run(&plan, &views);

        let mut expected = BTreeSet::new();
        for (c, p) in &edges {
            for (p2, g) in &edges {
                if p == p2 {
                    expected.insert(vec![*c, *p, *g]);
                }
            }
        }
        assert_eq!(results, expected);
        assert_eq!(results.len(), 3); // 0->1->2, 1->2->3, 4->1->2
    }

    #[test]
    fn triangle_is_wcoj_honest() {
        let dir = TempDir::new("run-triangle");
        let schema = schema(3);
        // R(x,y), S(y,z), T(z,x) over a small dense instance.
        let r: Vec<(u64, u64)> = (0..6).flat_map(|x| (0..6).map(move |y| (x, y))).collect();
        let s: Vec<(u64, u64)> = (0..6).map(|y| (y, (y + 1) % 6)).collect();
        let t: Vec<(u64, u64)> = (0..6).map(|z| (z, (z + 2) % 6)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 0)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let results = run(&plan, &views);

        let mut expected = BTreeSet::new();
        for (rx, ry) in &r {
            for (sy, sz) in &s {
                for (tz, tx) in &t {
                    if ry == sy && sz == tz && tx == rx {
                        expected.insert(vec![*rx, *ry, *sz]);
                    }
                }
            }
        }
        assert_eq!(results, expected);
        assert!(!results.is_empty());
    }

    #[test]
    fn zero_binding_atom_gates_the_query() {
        let dir = TempDir::new("run-gate");
        let schema = schema(2);
        let r = vec![(1u64, 2u64), (3, 4)];
        // Gate nonempty: results flow; gate empty: nothing.
        for (gate_rows, expect_rows) in [(vec![(9u64, 9u64)], 2usize), (vec![], 0)] {
            let dir2 = TempDir::new(&format!("run-gate-{expect_rows}"));
            let views = views_of(&dir2, &schema, &[r.clone(), gate_rows]);
            let normalized = NormalizedQuery {
                occurrences: vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    Occurrence {
                        occ_id: OccId(1),
                        relation: RelationId(1),
                        vars: vec![],
                        filters: vec![],
                    },
                ],
                residuals: vec![],
            };
            let plan = planned(&normalized, &schema, &[0, 1]);
            let results = run(&plan, &views);
            assert_eq!(results.len(), expect_rows, "gate case {expect_rows}");
        }
        drop(dir);
    }

    #[test]
    fn empty_relations_yield_empty_results() {
        let dir = TempDir::new("run-empty");
        let schema = schema(2);
        let views = views_of(&dir, &schema, &[vec![(1, 2)], vec![]]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        assert!(run(&plan, &views).is_empty());
    }

    #[test]
    fn duplicate_heavy_skew_collapses_to_the_distinct_binding_set() {
        let dir = TempDir::new("run-skew");
        let schema = schema(2);
        // Heavy duplication in the join column (post-collapse the binding
        // set is small).
        let r: Vec<(u64, u64)> = (0..50).map(|i| (i % 2, i % 3)).collect();
        let s: Vec<(u64, u64)> = (0..50).map(|i| (i % 3, i % 5)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let results = run(&plan, &views);
        let mut expected = BTreeSet::new();
        for (ra, rb) in &r {
            for (sa, sb) in &s {
                if rb == sa {
                    expected.insert(vec![*ra, *rb, *sb]);
                }
            }
        }
        assert_eq!(results, expected);
    }

    #[test]
    fn residuals_filter_across_atoms() {
        let dir = TempDir::new("run-residuals");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
        let s: Vec<(u64, u64)> = (0..10).map(|i| (i, 9 - i)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);
        // R(x, a), S(x, b), a < b.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![PlacedComparison {
                op: CmpOp::Lt,
                lhs: VarId(1),
                rhs: VarId(2),
            }],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let results = run(&plan, &views);
        let mut expected = BTreeSet::new();
        for (rx, ra) in &r {
            for (sx, sb) in &s {
                if rx == sx && ra < sb {
                    expected.insert(vec![*rx, *ra, *sb]);
                }
            }
        }
        assert_eq!(results, expected);
        assert_eq!(results.len(), 5); // i in 0..=4: i < 9-i
    }

    #[test]
    fn dynamic_cover_prefers_the_forced_small_side() {
        let dir = TempDir::new("run-cover-choice");
        let schema = schema(2);
        // R: huge with duplicate x; S: tiny. Node 0 = [R(x), S(x)] via a
        // GJ-style hand plan where both are covers.
        let r: Vec<(u64, u64)> = (0..500).map(|i| (i % 250, i)).collect();
        let s: Vec<(u64, u64)> = vec![(0, 0), (1, 1)];
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        // Hand-build the GJ plan: [[R(x), S(x)], [R(a)], [S(b)]].
        let plan = crate::plan::fj::FjPlan {
            nodes: vec![
                crate::plan::fj::Node {
                    subatoms: vec![
                        crate::plan::fj::Subatom {
                            occ: OccId(0),
                            vars: vec![VarId(0)],
                        },
                        crate::plan::fj::Subatom {
                            occ: OccId(1),
                            vars: vec![VarId(0)],
                        },
                    ],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(1)],
                    }],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(2)],
                    }],
                },
            ],
        };
        let plan = validate(&plan, &normalized, &schema, vec![0; 3], &BTreeSet::new())
            .expect("valid plan");

        // Pre-force S's root so its Exact(2) beats R's Estimate(500).
        let mut colts = colts_for(&plan, &views);
        let s_root = colts[1].root();
        colts[1].get(s_root, 0, &[0]);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut counters = RecordingCounters::default();
        Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

        // Node 0's first choice: subatom 1 (S), whose count is Exact.
        let (node, subatom, exact) = counters.cover_choices[0];
        assert_eq!((node, subatom, exact), (0, 1, true));
        assert!(!sink.rows.is_empty());
    }

    #[test]
    fn backtracking_restores_sources_across_sequential_executions() {
        let dir = TempDir::new("run-backtrack");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..20).map(|i| (i % 4, i)).collect();
        let s: Vec<(u64, u64)> = (0..4).map(|i| (i, i * 10)).collect();
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut executor = Executor::new(&plan);

        let mut first = CollectSink::default();
        executor.execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut first,
            &mut NoopCounters,
        );
        let mut second = CollectSink::default();
        executor.execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut second,
            &mut NoopCounters,
        );
        assert_eq!(first.rows, second.rows);
        assert!(!first.rows.is_empty());
    }
    // ---------- PRD 21: vectorized execution ----------

    /// Runs a plan at a given batch size.
    fn run_batched(plan: &ValidatedPlan, views: &[Arc<View>], batch: usize) -> BTreeSet<Vec<u64>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut executor = Executor::with_batch_size(plan, batch);
        executor.execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        sink.rows
    }

    #[test]
    fn results_are_identical_across_batch_sizes() {
        // Skew, empty relations, partial final batches, and batch > row
        // count are all covered by these fixtures x sizes.
        let dir = TempDir::new("run-batch-equality");
        let schema = schema(3);
        let r: Vec<(u64, u64)> = (0..150).map(|i| (i % 7, i % 11)).collect();
        let s: Vec<(u64, u64)> = (0..90).map(|i| (i % 11, i % 5)).collect();
        let t: Vec<(u64, u64)> = (0..40).map(|i| (i % 5, i)).collect();
        let views = views_of(&dir, &schema, &[r, s, t]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            residuals: vec![PlacedComparison {
                op: CmpOp::Ne,
                lhs: VarId(0),
                rhs: VarId(3),
            }],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let reference = run_batched(&plan, &views, 1);
        assert!(!reference.is_empty());
        for batch in [2usize, 64, 128, 1024] {
            assert_eq!(
                run_batched(&plan, &views, batch),
                reference,
                "batch size {batch} must match the scalar degenerate case"
            );
        }

        // An empty relation, every batch size.
        let dir2 = TempDir::new("run-batch-empty");
        let views = views_of(&dir2, &schema, &[vec![(1, 2)], vec![], vec![(0, 0)]]);
        for batch in [1usize, 2, 64, 128, 1024] {
            assert!(run_batched(&plan, &views, batch).is_empty());
        }
    }

    /// Counters recording the phase-1/phase-2 event order.
    #[derive(Default)]
    struct PhaseOrderCounters {
        events: Vec<(&'static str, usize, usize)>,
    }

    impl Counters for PhaseOrderCounters {
        fn node_entry(&mut self, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, node: usize, subatom: usize) {
            self.events.push(("hash", node, subatom));
        }
        fn probe(&mut self, node: usize, subatom: usize, _: bool) {
            self.events.push(("probe", node, subatom));
        }
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    #[test]
    fn phase_one_hashes_the_whole_batch_before_any_phase_two_probe() {
        let dir = TempDir::new("run-two-phase");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
        let s: Vec<(u64, u64)> = (0..10).map(|i| (i, i * 2)).collect();
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut counters = PhaseOrderCounters::default();
        Executor::with_batch_size(&plan, 128).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut counters,
        );

        // All 10 root entries fit one batch: every hash of node 0's sibling
        // pass must precede its first probe.
        let first_probe = counters
            .events
            .iter()
            .position(|(kind, node, _)| *kind == "probe" && *node == 0)
            .expect("probes happened");
        let hashes_before = counters.events[..first_probe]
            .iter()
            .filter(|(kind, node, _)| *kind == "hash" && *node == 0)
            .count();
        assert_eq!(
            hashes_before, 10,
            "the entire batch is hashed before the first bucket load"
        );
        assert!(!sink.rows.is_empty());
    }
}
