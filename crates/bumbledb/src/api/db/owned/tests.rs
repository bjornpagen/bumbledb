use super::super::{Fact, InstanceBuilder, OwnedInstance};
use crate::Db;
use crate::error::{Admission, Direction, Error, Violation};
use crate::ir::{Atom, FindTerm, Query, Rule, Term, Value, VarId};
use crate::schema::tests::{closed, containment, fd, field, row, side};
use crate::schema::{SchemaDescriptor, ValidateDescriptor as _};
use crate::testutil::TempDir;
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
    assert_eq!(instance.count(Account::RELATION).expect("count"), 1);

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

/// The exact-count law on the heap surface (one-representation PRD 40):
/// after mixed loads and deletes, `count` equals the scan length — the
/// admitted-instance twin of the storage pin
/// (`storage/read/tests.rs::row_count_equals_scan_count_after_mixed_commits`).
/// Both reads observe the one frozen catalog, so agreement is by
/// construction; pinned anyway.
#[test]
fn count_equals_scan_length_after_mixed_loads_and_deletes() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let ids: Vec<AccountId> = (0..3)
        .map(|_| {
            builder
                .reserve::<AccountId>(1)
                .expect("reserve")
                .start()
                .expect("nonempty")
        })
        .collect();
    let account = |id: AccountId, holder: &'static str| Account {
        id,
        holder,
        balance: 0,
    };
    builder
        .load([
            &account(ids[0], "ada"),
            &account(ids[1], "grace"),
            &account(ids[2], "kurt"),
        ])
        .expect("load");
    builder.delete([&account(ids[1], "grace")]).expect("delete");
    let instance = builder.admit().expect("admit").expect("accepted");
    let mut scanned = 0u64;
    for row in instance.scan(Account::RELATION).expect("scan") {
        row.expect("row");
        scanned += 1;
    }
    assert_eq!(
        instance.count(Account::RELATION).expect("count"),
        scanned,
        "count is the scan length"
    );
    assert_eq!(scanned, 2);
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
fn execute_on_owned_and_lease_agrees() {
    let (owned, _, _) = admit_ada();
    let mut owned_prepared = owned.prepare(&all_accounts()).expect("prepare");
    let mut owned_answers = Answers::new();
    owned
        .execute(&mut owned_prepared, &[], &mut owned_answers)
        .expect("owned execute");

    let dir = TempDir::new("execute-both-arms");
    let db = Db::from_instance(&dir.path().join("store"), &owned).expect("publish");
    let lease_answers = db
        .read(|instance| {
            let mut prepared = instance.prepare(&all_accounts())?;
            instance.execute_collect(&mut prepared, &[] as &[crate::BindValue])
        })
        .expect("lease execute");

    assert_eq!(owned_answers.len(), lease_answers.len());
}

#[test]
fn closed_relation_scans_without_building_an_image() {
    let instance = InstanceBuilder::new(WithClosed)
        .expect("valid")
        .admit()
        .expect("admit")
        .expect("accepted");
    assert_eq!(
        instance.count(KIND).expect("count"),
        1,
        "a closed relation's count IS its sealed extension length"
    );
    let kinds: Vec<Vec<Value>> = instance
        .scan(KIND)
        .expect("scan")
        .map(|row| row.expect("row"))
        .collect();
    assert_eq!(kinds, vec![vec![Value::U64(0)]]);
    assert!(instance.peek_image(KIND).is_none());
}
