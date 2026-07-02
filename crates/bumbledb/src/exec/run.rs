//! The recursive Free Join executor (PRD 19), scalar form — the semantics
//! reference PRD 21's vectorized path must match exactly
//! (`docs/architecture/30-execution.md`, paper §3.3 Fig. 5).
//!
//! Everything is a monomorphized generic — no `dyn` anywhere in the hot
//! path. The node loop: choose the cover by labeled key count, iterate it,
//! write binding slots, probe siblings, evaluate the node's residuals,
//! recurse; an undo journal restores each occurrence's current trie on
//! backtrack.

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

/// Per-node reusable scratch: each node's frame is active at most once in
/// the recursion (frames advance strictly by node index), so scratch is
/// indexed by node and allocated once per executor construction.
struct NodeScratch {
    /// Cover-entry key words (batch of 1 in the scalar executor).
    entry_keys: Vec<u64>,
    /// Probe-key gather buffer.
    probe_key: Vec<u64>,
    /// Undo journal: (occurrence index, previous cursor, previous level).
    journal: Vec<(usize, Cursor, usize)>,
}

/// The executor over one plan. Owns per-execution cursor state and
/// per-node scratch; borrows the COLT sources.
pub struct Executor<'p> {
    plan: &'p ValidatedPlan,
    /// Per occurrence: (current cursor, current trie level).
    cursors: Vec<(Cursor, usize)>,
    /// Per subatom slot maps, precomputed: `slot_map[node][subatom][i]` is
    /// the binding slot of that subatom's i-th variable.
    slot_map: Vec<Vec<Vec<usize>>>,
    /// Per residual: (lhs slot, rhs slot), aligned with each node's list.
    residual_slots: Vec<Vec<(PlacedComparison, usize, usize)>>,
    scratch: Vec<NodeScratch>,
}

impl<'p> Executor<'p> {
    #[must_use]
    pub fn new(plan: &'p ValidatedPlan) -> Self {
        let slot_map = plan
            .nodes()
            .iter()
            .map(|node| {
                node.subatoms
                    .iter()
                    .map(|s| s.vars.iter().map(|v| plan.slot_of(*v)).collect())
                    .collect()
            })
            .collect();
        let residual_slots = plan
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
                    .unwrap_or(0);
                NodeScratch {
                    entry_keys: vec![0; max_arity.max(1)],
                    probe_key: Vec::with_capacity(max_arity),
                    journal: Vec::new(),
                }
            })
            .collect();
        Self {
            plan,
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
        colts: &mut [Colt<'_>],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        assert_eq!(colts.len(), self.plan.occurrences().len());
        bindings.reset();
        self.cursors.clear();
        self.cursors
            .extend(colts.iter().map(|colt| (colt.root(), 0usize)));
        self.run_node(0, colts, bindings, sink, counters);
    }

    fn run_node<S: Sink, C: Counters>(
        &mut self,
        node_idx: usize,
        colts: &mut [Colt<'_>],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        if node_idx == self.plan.nodes().len() {
            counters.emit();
            return sink.emit(bindings);
        }
        counters.node_entry(node_idx);
        let node = &self.plan.nodes()[node_idx];

        // Dynamic cover choice (§4.4): prefer the smallest Exact, else the
        // smallest Estimate — the labels are load-bearing and never
        // compared as the same quantity (post-mortem §40).
        let cover_sub = self.choose_cover(node_idx, colts);
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
                probe_key: Vec::new(),
                journal: Vec::new(),
            },
        );
        let mut token = BatchToken::default();
        let mut child_buf = [Cursor::Row(0); 1];
        let mut flow = Flow::Continue;

        loop {
            let (yielded, next_token) = colts[cover_occ].iter_batch(
                cover_cursor,
                cover_level,
                token,
                &mut scratch.entry_keys,
                &mut child_buf,
                1,
            );
            if yielded == 0 {
                break;
            }
            token = next_token;

            scratch.journal.clear();
            // Bind the cover's variables and descend its trie.
            for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                bindings.set(*slot, scratch.entry_keys[i]);
            }
            debug_assert_eq!(self.slot_map[node_idx][cover_sub].len(), arity);
            scratch.journal.push((cover_occ, cover_cursor, cover_level));
            self.cursors[cover_occ] = (child_buf[0], cover_level + 1);

            // Probe the sibling subatoms in order.
            let mut alive = true;
            for (sub_idx, subatom) in self.plan.nodes()[node_idx].subatoms.iter().enumerate() {
                if sub_idx == cover_sub {
                    continue;
                }
                let occ = usize::from(subatom.occ.0);
                scratch.probe_key.clear();
                for slot in &self.slot_map[node_idx][sub_idx] {
                    scratch.probe_key.push(bindings.get(*slot));
                }
                let (cursor, level) = self.cursors[occ];
                let hit = colts[occ].get(cursor, level, &scratch.probe_key);
                counters.probe(node_idx, sub_idx, hit.is_some());
                if let Some(child) = hit {
                    scratch.journal.push((occ, cursor, level));
                    self.cursors[occ] = (child, level + 1);
                } else {
                    alive = false;
                    break;
                }
            }

            // Evaluate the node's residual comparisons on the slots.
            if alive {
                for (residual, lhs_slot, rhs_slot) in &self.residual_slots[node_idx] {
                    let pass = residual
                        .op
                        .compare(&bindings.get(*lhs_slot), &bindings.get(*rhs_slot));
                    counters.residual(node_idx, pass);
                    if !pass {
                        alive = false;
                        break;
                    }
                }
            }

            if alive {
                flow = self.run_node(node_idx + 1, colts, bindings, sink, counters);
            }

            // Backtrack: restore every occurrence this entry advanced.
            for (occ, cursor, level) in scratch.journal.drain(..).rev() {
                self.cursors[occ] = (cursor, level);
            }

            if flow == Flow::SkipSuffix {
                if self.plan.nodes()[node_idx].sink_relevant {
                    // This node binds a projected variable: absorb the skip
                    // and keep iterating — later entries change the output.
                    flow = Flow::Continue;
                } else {
                    // The suffix from here binds nothing sink-relevant:
                    // propagate the unwind (D2).
                    counters.skip(node_idx);
                    break;
                }
            }
        }

        self.scratch[node_idx] = scratch;
        flow
    }

    /// Chooses the cover with the fewest keys: smallest `Exact` wins;
    /// otherwise the smallest `Estimate` (v0 rule, 30-execution).
    fn choose_cover(&self, node_idx: usize, colts: &[Colt<'_>]) -> usize {
        let node = &self.plan.nodes()[node_idx];
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
        executor.execute(&mut colts, &mut bindings, &mut sink, &mut NoopCounters);
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
        Executor::new(&plan).execute(&mut colts, &mut bindings, &mut sink, &mut counters);

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
        executor.execute(&mut colts, &mut bindings, &mut first, &mut NoopCounters);
        let mut second = CollectSink::default();
        executor.execute(&mut colts, &mut bindings, &mut second, &mut NoopCounters);
        assert_eq!(first.rows, second.rows);
        assert!(!first.rows.is_empty());
    }
}
