use super::*;
use crate::encoding::{ValueRef, encode_fact};
use crate::error::Result;
use crate::exec::colt::Colt;
use crate::exec::run::{Counters, Executor};
use crate::image::view::apply;
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence, Role, SlotWidth};
use crate::plan::fj::{ValidatedPlan, binary2fj, factor, validate};
use crate::plan::planner::JoinOrder;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod aggregate;
mod projection;
mod semantics;

/// The construction boundary is total: every symbolic find variant has
/// one exact measure-free execution form. Measure rows also pin the
/// derived-word table that preserves execution-time ray checking.
#[test]
fn every_find_spec_parses_to_its_sink_spec() {
    let slot_count = 10;
    let rows = [
        (
            FindSpec::Var { slot: 1, width: 2 },
            SinkSpec::Var { slot: 1, width: 2 },
            vec![],
        ),
        (
            FindSpec::Duration { slot: 3 },
            SinkSpec::Var {
                slot: slot_count,
                width: 1,
            },
            vec![(slot_count, 3)],
        ),
        (
            FindSpec::AggDuration {
                op: FoldOp::Sum,
                slot: 4,
            },
            SinkSpec::Agg {
                op: FoldOp::Sum,
                over_slot: Some(slot_count),
                over_width: 1,
                signed: false,
            },
            vec![(slot_count, 4)],
        ),
        (
            FindSpec::Agg {
                op: FoldOp::CountDistinct,
                over_slot: Some(5),
                over_width: 2,
                signed: false,
            },
            SinkSpec::Agg {
                op: FoldOp::CountDistinct,
                over_slot: Some(5),
                over_width: 2,
                signed: false,
            },
            vec![],
        ),
        (
            FindSpec::Arg {
                slot: 6,
                width: 2,
                key_slot: 8,
                max: true,
            },
            SinkSpec::Arg {
                slot: 6,
                width: 2,
                key_slot: 8,
                max: true,
            },
            vec![],
        ),
        (
            FindSpec::Pack { slot: 7 },
            SinkSpec::Pack { slot: 7 },
            vec![],
        ),
    ];
    for (input, expected, expected_measures) in rows {
        let (parsed, measures) = super::aggregate::parse_finds(&[input], slot_count);
        assert_eq!(parsed, vec![expected]);
        assert_eq!(measures, expected_measures);
    }
}

/// Posting(id fresh u64, account u64, amount i64) +
/// PostingTag(posting u64, tag u64) +
/// Payroll(id fresh u64, emp u64, during Interval<I64>).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
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
            },
            RelationDescriptor {
                extension: None,
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
            },
            RelationDescriptor {
                extension: None,
                name: "Payroll".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const POSTING: RelationId = RelationId(0);
const TAG: RelationId = RelationId(1);
const PAYROLL: RelationId = RelationId(2);

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

/// Commits Payroll rows (interval facts) and returns its image.
fn payroll_views_of(
    dir: &TempDir,
    schema: &Schema,
    rows: &[(u64, u64, (i64, i64))],
) -> Vec<Arc<crate::image::RelationImage>> {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, emp, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*emp),
                ValueRef::IntervalI64(
                    crate::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
            ],
            schema.relation(PAYROLL).layout(),
            &mut bytes,
        );
        delta.insert(&view, PAYROLL, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    // Relation-id-indexed like `views_of` (Posting/Tag images empty).
    [POSTING, TAG, PAYROLL]
        .iter()
        .map(|rel| crate::image::build(&txn, schema, *rel).expect("build"))
        .collect()
}

fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
    plan.occurrences()
        .iter()
        .map(|occurrence| {
            // Field→column through the span map (docs/architecture/
            // 50-storage.md image layout): an interval field covers its
            // start/end column pair and shifts every later field.
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
                                .expect("plan vars");
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
                    &[],
                    &[],
                    Vec::new(),
                )
                .expect("no measure filters"),
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
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters: vec![],
    }
}

/// Assembles a `NormalizedQuery` the way `normalize` would: slot widths
/// derived from the schema field each variable reads (the `SlotWidth`
/// layout — interval variables are two words); no negation in these
/// fixtures.
fn normalized(
    schema: &Schema,
    occurrences: Vec<Occurrence>,
    residuals: Vec<crate::ir::normalize::PlacedComparison>,
) -> NormalizedQuery {
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| {
            let relation = schema.relation(o.relation);
            o.vars
                .iter()
                .map(move |(f, v)| (*v, SlotWidth::of(&relation.field(*f).value_type)))
        })
        .collect();
    NormalizedQuery {
        dead: None,
        occurrences,
        residuals,
        word_residuals: vec![],
        allen_residuals: Vec::new(),
        duration_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths,
    }
}

fn planned(
    schema: &Schema,
    normalized: &NormalizedQuery,
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

/// Hand-built two-node plans (group var above the leaf — the stats
/// shape) used by the batch-regime tests.
fn two_node_plan(
    schema: &Schema,
    normalized: &NormalizedQuery,
    first: &[u16],
    second: &[u16],
    sink_vars: &[u16],
) -> ValidatedPlan {
    let node = |vars: &[u16]| crate::plan::fj::Node {
        subatoms: vec![crate::plan::fj::Subatom {
            occ: OccId(0),
            vars: vars.iter().map(|v| VarId(*v)).collect(),
        }],
    };
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![node(first), node(second)],
    };
    let sinks: BTreeSet<VarId> = sink_vars.iter().map(|v| VarId(*v)).collect();
    validate(&plan, normalized, schema, vec![0; 2], &sinks).expect("valid plan")
}

fn run_aggregate(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    finds: Vec<FindSpec>,
) -> Result<Vec<Vec<u64>>> {
    run_aggregate_distinct(plan, views, finds, plan.distinct_bindings())
}

fn run_aggregate_distinct(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    finds: Vec<FindSpec>,
    distinct: bool,
) -> Result<Vec<Vec<u64>>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
    let mut sink = AggregateSink::new(finds, plan.slot_count(), distinct);
    Executor::new(plan)
        .execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        )
        .expect("execute");
    let mut rows = sink.into_rows()?;
    rows.sort_unstable();
    Ok(rows)
}

/// A scalar find's spec (width 1 through the layout map).
fn var_spec(plan: &ValidatedPlan, var: u16) -> FindSpec {
    FindSpec::Var {
        slot: plan.slot_of(VarId(var)),
        width: plan.width_of(VarId(var)),
    }
}

/// A scalar fold's spec.
fn agg_spec(plan: &ValidatedPlan, op: FoldOp, over: Option<u16>, signed: bool) -> FindSpec {
    FindSpec::Agg {
        op,
        over_slot: over.map(|v| plan.slot_of(VarId(v))),
        over_width: over.map_or(1, |v| plan.width_of(VarId(v))),
        signed,
    }
}

/// An Arg carry's spec.
fn arg_spec(plan: &ValidatedPlan, over: u16, key: u16, max: bool) -> FindSpec {
    FindSpec::Arg {
        slot: plan.slot_of(VarId(over)),
        width: plan.width_of(VarId(over)),
        key_slot: plan.slot_of(VarId(key)),
        max,
    }
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
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {
        self.skips += 1;
    }
}
