use super::*;
use crate::encoding::{encode_fact, ValueRef};
use crate::image::view::apply;
use crate::ir::normalize::{
    AntiProbe, NormalizedQuery, OccId, Occurrence, PlacedComparison, Polarity, SlotWidth,
};
use crate::ir::{CmpOp, VarId};
use crate::plan::fj::{binary2fj, factor, validate, ValidatedPlan};
use crate::plan::planner::JoinOrder;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use std::collections::{BTreeMap, BTreeSet};
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

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        debug_assert!(!stop_on_skip, "CollectSink never skips");
        for &entry in batch.survivors {
            let row: Vec<u64> = (0..batch.bindings.slot_count())
                .map(|slot| match batch.source_of(slot) {
                    LeafSource::Key(word) => batch.key(entry, word),
                    LeafSource::Outer => batch.bindings.get(slot),
                })
                .collect();
            self.rows.insert(row);
        }
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
    fn batch(&mut self, _: usize, _: usize) {}
    fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool) {
        self.cover_choices.push((node, subatom, exact));
    }
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
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
            })
            .collect(),
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Commits word rows into each relation and returns unfiltered views.
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

/// COLT sources for a plan: schema columns from each occurrence's trie
/// schema and var-to-field map, over the occurrence's filtered view
/// (production shape: a negated occurrence's constants are its filter
/// list, evaluated at the source — docs/architecture/40-execution.md,
/// § anti-probe filters).
fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
    colts_with_params(plan, images, &[])
}

/// [`colts_for`] with a bind-time param slice for filter evaluation
/// (set-carrying negated filters resolve through it in these fixtures).
fn colts_with_params(
    plan: &ValidatedPlan,
    images: &[Arc<crate::image::RelationImage>],
    params: &[crate::image::view::Const],
) -> Vec<Colt> {
    plan.occurrences()
        .iter()
        .map(|occurrence| {
            // Field→column through the span map (production shape —
            // interval fields cover two columns and shift their
            // successors).
            let columns: Vec<Vec<usize>> = occurrence
                .trie_schema
                .iter()
                .map(|level| {
                    level
                        .iter()
                        .flat_map(|var| {
                            let (field, _) = occurrence
                                .vars
                                .iter()
                                .find(|(_, v)| v == var)
                                .expect("plan vars come from the occurrence");
                            let span = occurrence.spans[usize::from(field.0)];
                            let first = usize::from(span.first_column);
                            match span.width {
                                crate::image::ColumnWidth::WordPair => vec![first, first + 1],
                                _ => vec![first],
                            }
                        })
                        .collect()
                })
                .collect();
            Colt::new(
                apply(
                    &images[usize::try_from(occurrence.relation.0).expect("small")],
                    &occurrence.filters,
                    params,
                    Vec::new(),
                ),
                &[],
                columns,
            )
        })
        .collect()
}

fn occurrence(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
    Occurrence {
        occ_id: OccId(occ),
        relation: RelationId(relation),
        polarity: Polarity::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters: vec![],
    }
}

/// A negated occurrence: joins no node, probed through its anti-probe.
fn negated(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
    Occurrence {
        polarity: Polarity::Negated,
        ..occurrence(occ, relation, vars)
    }
}

/// Assembles a `NormalizedQuery` the way `normalize` would: anti-probe
/// descriptors derived from the negated occurrences, every variable one
/// slot wide (these fixtures are scalar-only).
fn normalized(occurrences: Vec<Occurrence>, residuals: Vec<PlacedComparison>) -> NormalizedQuery {
    let anti_probes = occurrences
        .iter()
        .filter(|o| o.polarity == Polarity::Negated)
        .map(|o| AntiProbe {
            occurrence: o.occ_id,
            probe_bindings: o.vars.clone(),
        })
        .collect();
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| o.vars.iter().map(|(_, v)| (*v, SlotWidth::One)))
        .collect();
    NormalizedQuery {
        occurrences,
        residuals,
        word_residuals: vec![],
        anti_probes,
        slot_widths,
    }
}

fn planned(normalized: &NormalizedQuery, schema: &Schema, order: &[u16]) -> ValidatedPlan {
    planned_with_sinks(normalized, schema, order, &BTreeSet::new())
}

/// A plan with explicit sink vars — all-vars sets make every node
/// sink-relevant, i.e. skip-free: the pipelined executor's shapes.
fn planned_with_sinks(
    normalized: &NormalizedQuery,
    schema: &Schema,
    order: &[u16],
    sinks: &BTreeSet<VarId>,
) -> ValidatedPlan {
    let join_order = JoinOrder {
        order: order.iter().map(|o| OccId(*o)).collect(),
        estimates: vec![0; order.len()],
    };
    let mut plan = binary2fj(normalized, &join_order);
    factor(&mut plan);
    validate(&plan, normalized, schema, vec![0; order.len()], sinks).expect("valid plan")
}

/// All the query's vars — the skip-free sink set.
fn all_vars(normalized: &NormalizedQuery) -> BTreeSet<VarId> {
    normalized
        .occurrences
        .iter()
        .flat_map(|o| o.vars.iter().map(|(_, v)| *v))
        .collect()
}

fn run(plan: &ValidatedPlan, views: &[Arc<crate::image::RelationImage>]) -> BTreeSet<Vec<u64>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = Bindings::new(plan.slot_count());
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

/// Counters recording D2 skips (pipeline flavor).
#[derive(Default)]
struct SkipCounterRun {
    skips: usize,
}

impl Counters for SkipCounterRun {
    fn batch(&mut self, _: usize, _: usize) {}
    fn node_entry(&mut self, _: usize) {}
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {
        self.skips += 1;
    }
}

/// The real projection sink, re-exported for pipeline D2 tests.
use crate::exec::sink::ProjectionSink as ProjectionSinkForTest;

trait FirstCol {
    fn rows_first_col(&self) -> Vec<u64>;
}
impl FirstCol for ProjectionSinkForTest {
    fn rows_first_col(&self) -> Vec<u64> {
        self.rows().map(|r| r[0]).collect()
    }
}

// ---------- the 30-execution doc: vectorized execution ----------

/// Runs a plan at a given batch size.
fn run_batched(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    batch: usize,
) -> BTreeSet<Vec<u64>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = Bindings::new(plan.slot_count());
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

/// Counters recording the phase-1/phase-2 event order.
#[derive(Default)]
struct PhaseOrderCounters {
    events: Vec<(&'static str, usize, usize)>,
}

impl Counters for PhaseOrderCounters {
    fn batch(&mut self, _: usize, _: usize) {}
    fn node_entry(&mut self, _: usize) {}
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    fn probe_hash(&mut self, node: usize, subatom: usize) {
        self.events.push(("hash", node, subatom));
    }
    fn probe(&mut self, node: usize, subatom: usize, _: bool) {
        self.events.push(("probe", node, subatom));
    }
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {}
}

/// Runs a plan at a given batch size, collecting the full binding set.
fn run_at(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    batch: usize,
) -> BTreeSet<Vec<u64>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = Bindings::new(plan.slot_count());
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

mod cancellation;
mod correctness;
mod intervals;
mod mechanics;
mod negation;
mod pipeline;
