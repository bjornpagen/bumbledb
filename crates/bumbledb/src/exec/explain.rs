//! EXPLAIN (PRD 24): the debugging surface — an instrumented execution of
//! the same plan through the `Counters` seam, never a runtime mode
//! (`docs/architecture/30-execution.md`, observability).
//!
//! The normal path instantiates `NoopCounters` (zero-sized, compiled to
//! nothing); the EXPLAIN entry point instantiates [`CountingCounters`] and
//! executes the real query — ANALYZE semantics. Counter methods are plain
//! increments into plan-sized arrays: no formatting, no allocation in the
//! join loops. Output shape is OPEN per the architecture README; this
//! rendering is plain and stable-ish.

use std::fmt;

use crate::exec::dispatch::GuardPlan;
use crate::exec::run::Counters;
use crate::plan::fj::ValidatedPlan;

/// Plan-sized counters: every method is an increment, sized once at
/// construction (node count x max subatoms per node).
#[derive(Debug)]
pub struct CountingCounters {
    stride: usize,
    node_entries: Vec<u64>,
    /// Per (node, subatom): times chosen as cover with an `[Exact,
    /// Estimate]` count label — aggregated per node, not per entry.
    cover_choices: Vec<[u64; 2]>,
    /// Per (node, subatom): probe `[hit, miss]`.
    probes: Vec<[u64; 2]>,
    /// Per (node, subatom): phase-1 hash computations.
    hashes: Vec<u64>,
    /// Per node: residual `[pass, fail]`.
    residuals: Vec<[u64; 2]>,
    /// Per node: D2 subtree skips propagated through it.
    skips: Vec<u64>,
    emits: u64,
}

impl CountingCounters {
    #[must_use]
    pub fn new(plan: &ValidatedPlan) -> Self {
        let nodes = plan.nodes().len();
        let stride = plan
            .nodes()
            .iter()
            .map(|n| n.subatoms.len())
            .max()
            .unwrap_or(0);
        Self {
            stride,
            node_entries: vec![0; nodes],
            cover_choices: vec![[0; 2]; nodes * stride],
            probes: vec![[0; 2]; nodes * stride],
            hashes: vec![0; nodes * stride],
            residuals: vec![[0; 2]; nodes],
            skips: vec![0; nodes],
            emits: 0,
        }
    }

    /// Bindings emitted to the sink (the measured cardinality after the
    /// last node).
    #[cfg(test)]
    #[must_use]
    pub fn emits(&self) -> u64 {
        self.emits
    }

    /// The measured cardinality *after* node `k`: how many complete
    /// extensions survived it — entries of the next node, or sink emits
    /// for the last.
    #[must_use]
    pub fn actual_after(&self, node: usize) -> u64 {
        self.node_entries
            .get(node + 1)
            .copied()
            .unwrap_or(self.emits)
    }

    /// The `[Exact, Estimate]` cover-choice histogram cell.
    #[must_use]
    pub fn cover_histogram(&self, node: usize, subatom: usize) -> [u64; 2] {
        self.cover_choices[node * self.stride + subatom]
    }
}

impl Counters for CountingCounters {
    fn node_entry(&mut self, node: usize) {
        self.node_entries[node] += 1;
    }
    fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool) {
        self.cover_choices[node * self.stride + subatom][usize::from(!exact)] += 1;
    }
    fn probe_hash(&mut self, node: usize, subatom: usize) {
        self.hashes[node * self.stride + subatom] += 1;
    }
    fn probe(&mut self, node: usize, subatom: usize, hit: bool) {
        self.probes[node * self.stride + subatom][usize::from(!hit)] += 1;
    }
    fn residual(&mut self, node: usize, pass: bool) {
        self.residuals[node][usize::from(!pass)] += 1;
    }
    fn emit(&mut self) {
        self.emits += 1;
    }
    fn skip(&mut self, node: usize) {
        self.skips[node] += 1;
    }
}

/// The EXPLAIN report: the plan rendering plus (for the join engine) the
/// counted execution. `Display` formats lazily — nothing here ran inside
/// the hot loops.
#[derive(Debug)]
pub enum Report<'p> {
    /// The query classified as a point lookup (PRD 23).
    GuardProbe { plan: &'p GuardPlan },
    /// The Free Join engine, with its counted execution.
    FreeJoin {
        plan: &'p ValidatedPlan,
        counters: CountingCounters,
    },
}

impl fmt::Display for Report<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GuardProbe { plan } => {
                writeln!(f, "access path: guard probe")?;
                writeln!(f, "  relation: {}", plan.relation.0)?;
                match plan.constraint {
                    Some(c) => writeln!(f, "  unique constraint: {}", c.0)?,
                    None => writeln!(f, "  full-fact membership probe")?,
                }
                writeln!(
                    f,
                    "  key fields: {:?}",
                    plan.key
                        .iter()
                        .map(|(field, _)| field.0)
                        .collect::<Vec<_>>()
                )?;
                writeln!(f, "  remaining filters: {}", plan.remaining_filters.len())?;
                Ok(())
            }
            Self::FreeJoin { plan, counters } => {
                writeln!(f, "access path: free join ({} nodes)", plan.nodes().len())?;
                for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
                    writeln!(
                        f,
                        "  occurrence {occ_idx}: relation {} trie schema {:?} ({} filters)",
                        occurrence.relation.0,
                        occurrence
                            .trie_schema
                            .iter()
                            .map(|level| level.iter().map(|v| v.0).collect::<Vec<_>>())
                            .collect::<Vec<_>>(),
                        occurrence.filters.len(),
                    )?;
                }
                for (node_idx, node) in plan.nodes().iter().enumerate() {
                    writeln!(f, "  node {node_idx}:")?;
                    for (sub_idx, subatom) in node.subatoms.iter().enumerate() {
                        let [hit, miss] = counters.probes[node_idx * counters.stride + sub_idx];
                        let [exact, estimate] = counters.cover_histogram(node_idx, sub_idx);
                        writeln!(
                            f,
                            "    subatom {sub_idx}: occ {} vars {:?} cover({}) chosen \
                             exact={exact} estimate={estimate} probes hit={hit} miss={miss}",
                            subatom.occ.0,
                            subatom.vars.iter().map(|v| v.0).collect::<Vec<_>>(),
                            node.covers.contains(
                                &u8::try_from(sub_idx).expect("subatoms per node fit u8")
                            ),
                        )?;
                    }
                    let [pass, fail] = counters.residuals[node_idx];
                    writeln!(
                        f,
                        "    residuals: {} placed, pass={pass} fail={fail}",
                        node.residuals.len()
                    )?;
                    writeln!(
                        f,
                        "    estimated={} actual={} entries={} skips={}",
                        plan.estimates().get(node_idx).copied().unwrap_or(0),
                        counters.actual_after(node_idx),
                        counters.node_entries[node_idx],
                        counters.skips[node_idx],
                    )?;
                }
                writeln!(f, "  emitted bindings: {}", counters.emits)?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::exec::colt::Colt;
    use crate::exec::dispatch::classify;
    use crate::exec::run::{Bindings, Executor, NoopCounters};
    use crate::exec::sink::ProjectionSink;
    use crate::image::view::{apply, Const, FilterPredicate};
    use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence};
    use crate::ir::{CmpOp, VarId};
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

    fn schema(relations: usize) -> Schema {
        SchemaDescriptor {
            relations: (0..relations)
                .map(|r| RelationDescriptor {
                    name: format!("R{r}").into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "a".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Serial,
                        },
                        FieldDescriptor {
                            name: "b".into(),
                            value_type: ValueType::U64,
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

    fn views_of(
        dir: &TempDir,
        schema: &Schema,
        data: &[Vec<(u64, u64)>],
    ) -> Vec<Arc<crate::image::RelationImage>> {
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
                crate::image::build(&txn, schema, rel_id).expect("build")
            })
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

    #[test]
    fn estimates_and_actuals_populate_for_a_join_fixture() {
        let dir = TempDir::new("explain-join");
        let schema = schema(2);
        // R0: 5 rows; R1: joins on R0's serial (FK-walk shape).
        let r0: Vec<(u64, u64)> = (0..5).map(|i| (i, i * 10)).collect();
        let r1: Vec<(u64, u64)> = (0..20).map(|i| (i, i % 5)).collect();
        let views = views_of(&dir, &schema, &[r0, r1]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(1, 0), (0, 2)]),
            ],
            residuals: vec![],
        };
        let order = JoinOrder {
            order: vec![OccId(0), OccId(1)],
            estimates: vec![5, 20],
        };
        let mut fj = binary2fj(&normalized, &order);
        factor(&mut fj);
        let plan = validate(
            &fj,
            &normalized,
            &schema,
            order.estimates.clone(),
            &BTreeSet::new(),
        )
        .expect("valid plan");

        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(2))]);
        let mut counters = CountingCounters::new(&plan);
        Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

        // 20 R1 rows each match exactly one R0 row: actual after the last
        // node is 20 emits; estimates rendered beside them.
        assert_eq!(counters.emits(), 20);
        assert!(counters.actual_after(0) > 0);
        let report = Report::FreeJoin {
            plan: &plan,
            counters,
        };
        let text = format!("{report}");
        assert!(text.contains("estimated=5"));
        assert!(text.contains("emitted bindings: 20"));
    }

    #[test]
    fn the_skew_fixture_shows_the_expected_cover_choice() {
        // The correct-but-slow regression detector (50-validation): on a
        // constructed skew fixture, the histogram must show the forced
        // small side chosen with an Exact label.
        let dir = TempDir::new("explain-skew");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..500).map(|i| (i, i % 250)).collect();
        let s: Vec<(u64, u64)> = vec![(0, 0), (1, 1)];
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(1, 0), (0, 1)]),
                occurrence(1, 1, &[(1, 0), (0, 2)]),
            ],
            residuals: vec![],
        };
        // GJ-style node with both as covers.
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
        let mut colts = colts_for(&plan, &views);
        // Pre-force the tiny side so its Exact(2) beats Estimate(500).
        let s_root = Colt::root();
        colts[1].get(s_root, 0, &[0]);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(0))]);
        let mut counters = CountingCounters::new(&plan);
        Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

        // Node 0 chose subatom 1 (the forced small side), labeled Exact.
        assert_eq!(counters.cover_histogram(0, 1)[0], 1);
        assert_eq!(counters.cover_histogram(0, 0), [0, 0]);
        let report = Report::FreeJoin {
            plan: &plan,
            counters,
        };
        assert!(format!("{report}").contains("exact=1"));
    }

    #[test]
    fn guard_probe_queries_report_their_classification() {
        let schema = schema(1);
        let normalized = NormalizedQuery {
            occurrences: vec![Occurrence {
                occ_id: OccId(0),
                relation: RelationId(0),
                vars: vec![(FieldId(1), VarId(0))],
                filters: vec![FilterPredicate::Compare {
                    field: FieldId(0),
                    op: CmpOp::Eq,
                    value: Const::Word(5),
                }],
            }],
            residuals: vec![],
        };
        let guard = classify(&normalized, &schema).expect("guard probe");
        let report = Report::GuardProbe { plan: &guard };
        let text = format!("{report}");
        assert!(text.contains("guard probe"));
        assert!(text.contains("unique constraint: 0"));
    }

    #[test]
    fn noop_counters_are_zero_sized_and_the_normal_path_carries_no_state() {
        // The type-system proof (30-execution): the release path's counter
        // type occupies no memory and the executor stores no counter field
        // (counters are a call-site parameter, monomorphized away).
        assert_eq!(std::mem::size_of::<NoopCounters>(), 0);
    }
}
