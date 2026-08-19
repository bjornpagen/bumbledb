use super::super::{Fact, InstanceBuilder, OwnedInstance};
use crate::error::{Admission, Direction, Error, Violation};
use crate::ir::{Atom, FindTerm, Query, Rule, Term, Value, VarId};
use crate::schema::tests::{closed, containment, fd, field, row, side};
use crate::schema::{SchemaDescriptor, ValidateDescriptor as _};
use crate::{AnswerValue, Answers, AtomSource};
use bumbledb_theory::schema::{FieldId, RelationId};

crate::schema! {
    pub Ledger;

    relation Account {
        id: u64 as AccountId, fresh,
        holder: str,
        balance: i64,
    }
}

crate::schema! {
    pub WithClosed;
    closed relation Kind as KindId = { Checking };
    relation Item { id: u64 as ItemId, fresh }
}

const KIND: RelationId = RelationId(0);

/// Q(holder, balance) :- Account(holder, balance).
fn all_accounts() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Account::RELATION),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn admit_ada() -> (OwnedInstance<Ledger>, AccountId, Account<'static>) {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    builder.load([&acct]).expect("load");
    let instance = builder.admit().expect("admit").expect("accepted");
    (instance, id, acct)
}

fn closed_source_ordinary_target() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![],
                vec![row("Soft", vec![]), row("Hard", vec![])],
            ),
            crate::schema::RelationDescriptor {
                extension: None,
                name: "Bucket".into(),
                fields: vec![field("id", crate::schema::ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    }
}

#[test]
fn admitted_instance_prepares_executes_gets_and_scans() {
    let (instance, id, acct) = admit_ada();
    assert!(instance.peek_image(Account::RELATION).is_none());

    assert!(instance.contains(&acct).expect("contains"));
    assert_eq!(instance.get(id).expect("get"), Some(acct));
    assert_eq!(instance.row_count(Account::RELATION).expect("count"), 1);

    let scanned: Vec<Account<'_>> = instance
        .scan_facts::<Account>()
        .expect("scan_facts")
        .map(|row| row.expect("row"))
        .collect();
    assert_eq!(scanned, vec![acct]);

    let dyn_rows: Vec<Vec<Value>> = instance
        .scan(Account::RELATION)
        .expect("scan")
        .map(|row| row.expect("row"))
        .collect();
    assert_eq!(
        dyn_rows,
        vec![vec![
            Value::U64(id.0),
            Value::String("ada".into()),
            Value::I64(10)
        ]]
    );
    assert!(instance.peek_image(Account::RELATION).is_none());

    let mut prepared = instance.prepare(&all_accounts()).expect("prepare");
    let mut out = Answers::new();
    instance
        .execute(&mut prepared, &[], &mut out)
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert!(matches!(out.get(0, 0), AnswerValue::String("ada")));
    assert!(matches!(out.get(0, 1), AnswerValue::I64(10)));
    assert!(instance.peek_image(Account::RELATION).is_some());
}

#[test]
fn rejected_builder_never_yields_an_instance() {
    closed_source_ordinary_target()
        .validate()
        .expect("validates");
    let builder = InstanceBuilder::new(closed_source_ordinary_target()).expect("valid");
    match builder.admit().expect("admit") {
        Admission::Rejected(violations) => {
            assert!(
                violations.iter().any(|v| matches!(
                    v,
                    Violation::Containment {
                        direction: Direction::SourceUnsatisfied,
                        ..
                    }
                )),
                "{violations}"
            );
        }
        Admission::Accepted(_) => {
            panic!("rejected candidate must not occupy an OwnedInstance slot")
        }
    }
}

#[test]
fn foreign_prepared_query_is_rejected() {
    let (left, _, _) = admit_ada();
    let (right, _, _) = admit_ada();
    let mut prepared = left.prepare(&all_accounts()).expect("prepare");
    let mut out = Answers::new();
    let err = right
        .execute(&mut prepared, &[], &mut out)
        .expect_err("foreign");
    assert!(matches!(err, Error::ForeignPreparedQuery), "{err:?}");
}

#[test]
fn closed_relation_scans_without_building_an_image() {
    let instance = InstanceBuilder::new(WithClosed)
        .expect("valid")
        .admit()
        .expect("admit")
        .expect("accepted");
    assert_eq!(instance.row_count(KIND).expect("count"), 1);
    let kinds: Vec<Vec<Value>> = instance
        .scan(KIND)
        .expect("scan")
        .map(|row| row.expect("row"))
        .collect();
    assert_eq!(kinds, vec![vec![Value::U64(0)]]);
    assert!(instance.peek_image(KIND).is_none());
}
