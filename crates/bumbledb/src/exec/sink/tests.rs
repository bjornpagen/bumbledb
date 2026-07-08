use super::*;
use crate::encoding::{encode_fact, ValueRef};
use crate::error::Result;
use crate::exec::colt::Colt;
use crate::exec::run::{Counters, Executor};
use crate::image::view::apply;
use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence};
use crate::ir::VarId;
use crate::plan::fj::{binary2fj, factor, validate, ValidatedPlan};
use crate::plan::planner::JoinOrder;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use std::collections::BTreeSet;
use std::sync::Arc;

mod aggregate;
mod projection;
mod semantics;

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
