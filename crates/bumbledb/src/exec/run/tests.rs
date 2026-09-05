use super::*;
use crate::image::testsupport::TestSource;
use crate::image::view::{FilterPredicate, OperandAddr, apply};
use crate::ir::VarId;
use crate::ir::normalize::{
    AntiProbe, NormalizedQuery, OccBind, OccId, Occurrence, Role, SlotWidth,
};
use crate::plan::fj::{ValidatedPlan, binary2fj, factor, validate};
use crate::plan::planner::JoinOrder;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
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

#[derive(Default)]
struct RecordingCounters {
    cover_choices: Vec<(usize, usize, bool)>,
}

impl Counters for RecordingCounters {
    fn node_entry(&mut self, _: usize) {}
    fn batch(&mut self, _: usize, _: usize) {}
    fn cover_choice(&mut self, node: usize, subatom: usize, count: crate::exec::colt::KeyCount) {
        self.cover_choices.push((
            node,
            subatom,
            matches!(count, crate::exec::colt::KeyCount::Exact(_)),
        ));
    }
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {}
}

fn schema(relations: usize) -> Schema {
    SchemaDescriptor {
        relations: (0..relations)
            .map(|r| RelationDescriptor {
                extension: None,
                name: format!("R{r}").into(),
                fields: vec![
                    FieldDescriptor {
                        name: "a".into(),
                        value_type: bumbledb_theory::schema::ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "b".into(),
                        value_type: bumbledb_theory::schema::ValueType::U64,
                    },
                ],
            })
            .collect(),
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn views_of(schema: &Schema, data: &[Vec<(u64, u64)>]) -> Vec<Arc<crate::image::RelationImage>> {
    let rows: Vec<(RelationId, Vec<Vec<crate::ir::Value>>)> = data
        .iter()
        .enumerate()
        .map(|(rel, rows)| {
            let rel_id = RelationId(u32::try_from(rel).expect("small"));
            let facts = rows
                .iter()
                .map(|(a, b)| vec![crate::ir::Value::U64(*a), crate::ir::Value::U64(*b)])
                .collect();
            (rel_id, facts)
        })
        .collect();
    let source = TestSource::new(schema, &rows);
    let cache = crate::image::cache::ImageCache::new(schema);
    (0..data.len())
        .map(|rel| {
            let rel_id = RelationId(u32::try_from(rel).expect("small"));
            source.image(&cache, rel_id)
        })
        .collect()
}

fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
    colts_with_params(plan, images, &[])
}

fn colts_with_params(
    plan: &ValidatedPlan,
    images: &[Arc<crate::image::RelationImage>],
    params: &[crate::image::view::Const],
) -> Vec<Colt> {
    plan.occurrences()
        .iter()
        .map(|occurrence| {
            // Field→column through the span map (production shape —

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
                    &images[usize::try_from(occurrence.bind.edb().expect("fixture").0)
                        .expect("small")],
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
        bind: OccBind::Edb(RelationId(relation)),
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters: vec![],
        point_vars: vec![],
    }
}

fn negated(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
    Occurrence {
        role: Role::Negated,
        ..occurrence(occ, relation, vars)
    }
}

fn normalized(occurrences: Vec<Occurrence>, residuals: Vec<FilterPredicate>) -> NormalizedQuery {
    let anti_probes = occurrences
        .iter()
        .filter(|o| o.role == Role::Negated)
        .map(|o| AntiProbe {
            occurrence: o.occ_id,
            probe_bindings: o.vars.clone(),
        })
        .collect();
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| o.vars.iter().map(|(_, v)| (*v, SlotWidth::ONE)))
        .collect();
    NormalizedQuery {
        dead: None,
        occurrences,
        residuals,
        word_residuals: vec![],
        allen_residuals: vec![],
        anti_probes,
        slot_widths,
    }
}

fn planned(normalized: &NormalizedQuery, schema: &Schema, order: &[u16]) -> ValidatedPlan {
    planned_with_sinks(normalized, schema, order, &BTreeSet::new())
}

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
    validate(&plan, normalized, schema, sinks).expect("valid plan")
}

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
    executor
        .execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("execute");
    sink.rows
}

#[derive(Default)]
struct SkipCounterRun {
    skips: usize,
}

impl Counters for SkipCounterRun {
    fn batch(&mut self, _: usize, _: usize) {}
    fn node_entry(&mut self, _: usize) {}
    fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {
        self.skips += 1;
    }
}

use crate::exec::sink::ProjectionSink as ProjectionSinkForTest;

trait FirstCol {
    fn rows_first_col(&self) -> Vec<u64>;
}
impl FirstCol for ProjectionSinkForTest {
    fn rows_first_col(&self) -> Vec<u64> {
        self.answers().map(|answer| answer[0]).collect()
    }
}

fn run_batched(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    batch: usize,
) -> BTreeSet<Vec<u64>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut executor = Executor::with_batch_size(plan, batch);
    executor
        .execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("execute");
    sink.rows
}

#[derive(Default)]
struct PhaseOrderCounters {
    events: Vec<(&'static str, usize, usize)>,
}

impl Counters for PhaseOrderCounters {
    fn batch(&mut self, _: usize, _: usize) {}
    fn node_entry(&mut self, _: usize) {}
    fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
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

fn run_at(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    batch: usize,
) -> BTreeSet<Vec<u64>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut executor = Executor::with_batch_size(plan, batch);
    executor
        .execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("execute");
    sink.rows
}

mod batch_accounting;
mod cancellation;
mod correctness;
mod intervals;
mod mechanics;
mod negation;
mod pinned_run;
mod pipeline;
mod scan;
