//! Prepared-query suite root: the shared fixtures over the successor
//! substrate. Two harnesses, one dynamic test theory:
//!
//! - [`Fix`] — the heap harness ([`InstanceBuilder`] → [`OwnedInstance`]);
//!   no LMDB, no directories. Most denotation suites run here.
//! - [`StoreFix`] — the store harness ([`Db`] over one temp directory);
//!   generation/epoch/latch/snapshot behavior runs here.
//!
//! Both execute through the C05 entries (`prepare_on`/`prepare_owned`,
//! `execute`/`execute_owned`), so every suite exercises the real seam.
use super::*;

use crate::api::db::{InstanceBuilder, OwnedInstance};
use crate::error::Error;
use crate::ir::{
    Atom, AtomSource, CmpOp, Comparison, ConditionTree, FindTerm, HeadTerm, Interior, InteriorId,
    Query, Rule, Term, Value, VarId,
};
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor,
};

mod aggregates;
mod answers;
mod correctness;
mod disjoint;
mod float_aggregates;
mod folded;
mod ground;
mod key_probe;
mod latch;
mod pack;
mod params;
mod reach;
mod rules;
mod selection;
mod sets;
mod snapshot;
mod statically_empty;
mod view_memo;

/// The one dynamic test theory: any descriptor as a `Theory`.
#[derive(Clone)]
pub(super) struct T(pub(super) SchemaDescriptor);

impl crate::schema::Theory for T {
    fn descriptor(self) -> SchemaDescriptor {
        self.0
    }
}

/// The heap harness: an admitted instance plus the C05 owned entries.
pub(super) struct Fix {
    pub(super) instance: OwnedInstance<T>,
}

impl Fix {
    /// Build from a descriptor and dynamic rows per relation.
    pub(super) fn heap(
        descriptor: SchemaDescriptor,
        rows: &[(RelationId, Vec<Vec<Value>>)],
    ) -> Self {
        let mut builder = InstanceBuilder::new(T(descriptor)).expect("valid fixture schema");
        for (relation, facts) in rows {
            builder
                .load_dyn(*relation, facts.iter())
                .expect("fixture rows load");
        }
        let instance = builder
            .admit()
            .expect("fixture admission")
            .expect("fixture rows are law-abiding");
        Self { instance }
    }

    pub(super) fn prepare(&self, query: &Query) -> crate::error::Result<PreparedQuery<T>> {
        self.instance.prepare(query)
    }

    pub(super) fn execute_into<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<T>,
        params: P,
        out: &mut Answers,
    ) -> crate::error::Result<()> {
        prepared.execute_owned(&self.instance, params, out)
    }

    pub(super) fn execute<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<T>,
        params: P,
    ) -> crate::error::Result<Answers> {
        let mut out = Answers::new();
        self.execute_into(prepared, params, &mut out)?;
        Ok(out)
    }
}

/// The store harness: one real successor store in a temp directory.
pub(super) struct StoreFix {
    pub(super) db: crate::api::db::Db<T>,
    _dir: TempDir,
}

impl StoreFix {
    pub(super) fn store(name: &'static str, descriptor: SchemaDescriptor) -> Self {
        let dir = TempDir::new(name);
        let db = crate::api::db::Db::create(dir.path(), T(descriptor))
            .expect("create store")
            .expect("empty state admits");
        Self { db, _dir: dir }
    }

    pub(super) fn insert_dyn(&self, relation: RelationId, facts: &[Vec<Value>]) {
        self.db
            .write(|tx| {
                for fact in facts {
                    tx.insert_dyn(relation, [fact.as_slice()])?;
                }
                Ok(())
            })
            .expect("write")
            .expect("fixture rows are law-abiding");
    }

    pub(super) fn prepare(&self, query: &Query) -> crate::error::Result<PreparedQuery<T>> {
        self.db.read(|instance| instance.prepare(query))
    }

    pub(super) fn execute_into<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<T>,
        params: P,
        out: &mut Answers,
    ) -> crate::error::Result<()> {
        self.db
            .read(|instance| instance.execute(prepared, params, out))
    }

    pub(super) fn execute<'p, P: BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<T>,
        params: P,
    ) -> crate::error::Result<Answers> {
        let mut out = Answers::new();
        self.execute_into(prepared, params, &mut out)?;
        Ok(out)
    }
}

fn descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
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
                    name: "memo".into(),
                    value_type: ValueType::String,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                },
            ],
        }],
        // The declared id key: fixture ids are unique, and the key-probe
        // suite needs a Functionality statement to classify against (the
        // old fresh auto-key is deleted with the mechanism).
        statements: vec![
            bumbledb_theory::schema::StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },
        ],
    }
}

const POSTING: RelationId = RelationId(0);

fn posting_rows(rows: &[(u64, u64, &str, i64)]) -> Vec<Vec<Value>> {
    rows.iter()
        .map(|(id, account, memo, amount)| {
            vec![
                Value::U64(*id),
                Value::U64(*account),
                Value::String((*memo).into()),
                Value::I64(*amount),
            ]
        })
        .collect()
}

/// The default heap fixture over the Posting relation.
fn postings(rows: &[(u64, u64, &str, i64)]) -> Fix {
    Fix::heap(descriptor(), &[(POSTING, posting_rows(rows))])
}

/// The default store fixture over the Posting relation.
fn posting_store(name: &'static str, rows: &[(u64, u64, &str, i64)]) -> StoreFix {
    let fix = StoreFix::store(name, descriptor());
    fix.insert_dyn(POSTING, &posting_rows(rows));
    fix
}

fn by_account_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Param(crate::ir::ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(crate::ir::ParamId(1)),
        })],
    })
}

fn answers_of(buffer: &Answers) -> Vec<(String, i64)> {
    let mut answers: Vec<(String, i64)> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::String(memo) = buffer.get(answer, 0) else {
                panic!("column 0 is a string");
            };
            let AnswerValue::I64(amount) = buffer.get(answer, 1) else {
                panic!("column 1 is an i64");
            };
            (memo.to_owned(), amount)
        })
        .collect();
    answers.sort();
    answers
}

fn by_memo_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn memo_param(text: &str) -> Vec<BindValue<'_>> {
    vec![BindValue::Str(text)]
}

fn amounts_of(buffer: &Answers) -> Vec<i64> {
    let mut amounts: Vec<i64> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::I64(amount) = buffer.get(answer, 0) else {
                panic!("column 0 is an i64");
            };
            amount
        })
        .collect();
    amounts.sort_unstable();
    amounts
}
