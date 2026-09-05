use super::*;
use crate::error::Result;
use crate::exec::colt::Colt;
use crate::exec::run::{Counters, Executor};
use crate::image::testsupport::TestSource;
use crate::image::view::{FilterPredicate, OperandAddr, apply};
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, OccBind, OccId, Occurrence, Role, SlotWidth};
use crate::plan::fj::{ValidatedPlan, binary2fj, factor, validate};
use crate::plan::planner::JoinOrder;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, RelationId, SchemaDescriptor,
    ValueType,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod aggregate;
mod pack;
mod projection;
mod semantics;
mod stage_spill;

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
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
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
                    },
                    FieldDescriptor {
                        name: "tag".into(),
                        value_type: ValueType::U64,
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
                    },
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::I64,
                        },
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
    schema: &Schema,
    postings: &[(u64, u64, i64)],
    tags: &[(u64, u64)],
) -> Vec<Arc<crate::image::RelationImage>> {
    let posting_rows: Vec<Vec<crate::ir::Value>> = postings
        .iter()
        .map(|(id, account, amount)| {
            vec![
                crate::ir::Value::U64(*id),
                crate::ir::Value::U64(*account),
                crate::ir::Value::I64(*amount),
            ]
        })
        .collect();
    let tag_rows: Vec<Vec<crate::ir::Value>> = tags
        .iter()
        .map(|(posting, tag)| vec![crate::ir::Value::U64(*posting), crate::ir::Value::U64(*tag)])
        .collect();
    let source = TestSource::new(schema, &[(POSTING, posting_rows), (TAG, tag_rows)]);
    let cache = crate::image::cache::ImageCache::new(schema);
    [POSTING, TAG]
        .iter()
        .map(|rel| source.image(&cache, *rel))
        .collect()
}

fn payroll_views_of(
    schema: &Schema,
    rows: &[(u64, u64, (i64, i64))],
) -> Vec<Arc<crate::image::RelationImage>> {
    let payroll_rows: Vec<Vec<crate::ir::Value>> = rows
        .iter()
        .map(|(id, emp, (start, end))| {
            vec![
                crate::ir::Value::U64(*id),
                crate::ir::Value::U64(*emp),
                crate::ir::Value::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
            ]
        })
        .collect();
    let source = TestSource::new(schema, &[(PAYROLL, payroll_rows)]);
    let cache = crate::image::cache::ImageCache::new(schema);
    [POSTING, TAG, PAYROLL]
        .iter()
        .map(|rel| source.image(&cache, *rel))
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
            let image = &images[usize::try_from(occurrence.bind.edb().expect("fixture").0)
                .expect("small")];
            Colt::new(
                apply(
                    image,
                    &[],
                    &[],
                    Vec::new(),
                    image.generation().text_eq(None),
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
        bind: OccBind::Edb(relation),
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters: vec![],
        point_vars: vec![],
    }
}

fn normalized(
    schema: &Schema,
    occurrences: Vec<Occurrence>,
    residuals: Vec<crate::image::view::FilterPredicate>,
) -> NormalizedQuery {
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| {
            let relation = schema.relation(o.source().edb().expect("fixture"));
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
    validate(&plan, normalized, schema, &sinks).expect("valid plan")
}

fn two_node_plan(
    schema: &Schema,
    normalized: &NormalizedQuery,
    first: &[u16],
    second: &[u16],
    sink_vars: &[u16],
) -> ValidatedPlan {
    let node = |vars: &[u16]| crate::plan::fj::Node {
        estimate: 0,
        subatoms: vec![crate::plan::fj::Subatom {
            occ: OccId(0),
            vars: vars.iter().map(|v| VarId(*v)).collect(),
        }],
    };
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![node(first), node(second)],
    };
    let sinks: BTreeSet<VarId> = sink_vars.iter().map(|v| VarId(*v)).collect();
    validate(&plan, normalized, schema, &sinks).expect("valid plan")
}

fn run_aggregate(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    finds: Vec<FindSpec>,
) -> Result<Vec<Vec<u64>>> {
    run_aggregate_distinct(plan, views, finds, plan.distinct_witness().is_some())
}

fn run_aggregate_distinct(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    finds: Vec<FindSpec>,
    distinct: bool,
) -> Result<Vec<Vec<u64>>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
    let mut sink = aggregate_sink(plan, finds, distinct);
    Executor::new(plan)
        .execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        )
        .expect("execute");
    let mut rows = sink.into_answers()?;
    rows.sort_unstable();
    Ok(rows)
}

fn aggregate_sink(plan: &ValidatedPlan, finds: Vec<FindSpec>, elided: bool) -> AggregateSink {
    if elided {
        AggregateSink::new_distinct(
            finds,
            plan.slot_count(),
            plan.distinct_witness()
                .expect("the test's elided regime requires a proved plan"),
        )
    } else {
        AggregateSink::new(finds, plan.slot_count())
    }
}

fn var_spec(plan: &ValidatedPlan, var: u16) -> FindSpec {
    FindSpec::Var {
        slot: plan.slot_of(VarId(var)),
        width: plan.width_of(VarId(var)),
    }
}

fn agg_spec(plan: &ValidatedPlan, op: FoldOp, over: u16, signed: bool) -> FindSpec {
    FindSpec::Agg(AggSpec::Fold {
        op,
        slot: plan.slot_of(VarId(over)),
        width: plan.width_of(VarId(over)),
        signed,
    })
}

#[derive(Default)]
struct SkipCounter {
    skips: usize,
}

impl Counters for SkipCounter {
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
