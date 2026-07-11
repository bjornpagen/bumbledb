use super::*;

use crate::encoding::{encode_fact, ValueRef};
use crate::error::Error;
use crate::image::cache::ImageCache;
use crate::ir::{
    Atom, CmpOp, Comparison, FindTerm, PredicateTree, Query, Rule, Term, Value, VarId,
};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

mod aggregates;
mod buffer;
mod chase;
mod correctness;
mod disjoint;
mod explain;
mod guard;
mod measure;
mod params;
mod rules;
mod selection;
mod sets;
mod snapshot;
mod view_memo;

/// The unit-typestate prepare: these tests drive the environment
/// directly, below the `Db<S>` surface where the schema typestate
/// lives, so `S` is uninferable — pin it to `()`.
fn prepare<'s>(
    txn: &crate::storage::env::ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    query: &Query,
) -> crate::error::Result<PreparedQuery<'s, ()>> {
    super::build::prepare(txn, cache, schema, query)
}

/// Posting(id fresh u64, account u64, memo string, amount i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
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
                    name: "memo".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const POSTING: RelationId = RelationId(0);

fn insert_postings(env: &Environment, schema: &Schema, rows: &[(u64, u64, &str, i64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, account, memo, amount) in rows {
        let memo_id = delta.intern_str(&view, memo).expect("intern");
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*account),
                ValueRef::String(memo_id),
                ValueRef::I64(*amount),
            ],
            schema.relation(POSTING).layout(),
            &mut bytes,
        );
        delta.insert(&view, POSTING, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// Q(memo, amount) :- Posting(account = ?0, memo, amount), amount >= ?1.
fn by_account_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(1), Term::Param(crate::ir::ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(crate::ir::ParamId(1)),
        })],
    })
}

fn rows_of(buffer: &ResultBuffer) -> Vec<(String, i64)> {
    let mut rows: Vec<(String, i64)> = (0..buffer.len())
        .map(|row| {
            let ResultValue::String(memo) = buffer.get(row, 0) else {
                panic!("column 0 is a string");
            };
            let ResultValue::I64(amount) = buffer.get(row, 1) else {
                panic!("column 1 is an i64");
            };
            (memo.to_owned(), amount)
        })
        .collect();
    rows.sort();
    rows
}

/// Q(amount) :- Posting(memo = ?0, amount) — the selection shape
/// (docs/architecture/40-execution.md): a param-Eq on a field outside
/// every key.
fn by_memo_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    })
}

fn memo_param(text: &str) -> Vec<BindValue<'_>> {
    vec![BindValue::Str(text)]
}

fn amounts_of(buffer: &ResultBuffer) -> Vec<i64> {
    let mut amounts: Vec<i64> = (0..buffer.len())
        .map(|row| {
            let ResultValue::I64(amount) = buffer.get(row, 0) else {
                panic!("column 0 is an i64");
            };
            amount
        })
        .collect();
    amounts.sort_unstable();
    amounts
}
