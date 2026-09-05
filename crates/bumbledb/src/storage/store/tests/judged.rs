//! E-ADMIT / ENG-005 through the FULL physical path: the production
//! `SchemaJudge` (P01's reference final-state judge, C03) bound to the
//! store's private candidate (C04). The judged view is the proposed final
//! state in the candidate transaction; competing rows are all visible and
//! all cited, and a rejection retains the writer session.

use super::*;
use crate::schema::judge::JudgeBudget;
use crate::schema::{FieldId, StatementDescriptor};
use crate::storage::store::judge_bridge::{SchemaJudge, UnindexedRows};

const USER: RelationId = RelationId(0);

/// `User { id: u64, email: str }` with two declared keys: id and email.
fn user_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "User".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "email".into(),
                    value_type: ValueType::String,
                },
            ],
            extension: None,
        }],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: USER,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: USER,
                projection: Box::from([FieldId(1)]),
            },
        ],
    }
    .validate()
    .expect("user schema validates")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(email.into())]
}

fn user_store(path: &std::path::Path) -> Store {
    Store::create(path, &user_schema(), MapPolicy::default())
        .expect("user store")
        .0
}

fn user_changes(adds: &[Vec<Value>], removes: &[Vec<Value>]) -> ChangeSet {
    let schema = user_schema();
    let mut builder = ChangeSet::builder(&schema, work());
    for values in removes {
        builder.delete(USER, values).expect("stage delete");
    }
    for values in adds {
        builder.insert(USER, values).expect("stage insert");
    }
    builder.finish().expect("sealed user changes")
}

fn judged_commit(
    store: &Store,
    changes: &ChangeSet,
) -> Result<StoreCommit, Vec<crate::schema::judge::JudgedViolation>> {
    let schema = user_schema();
    let judge = SchemaJudge {
        schema: &schema,
        budget: JudgeBudget::default(),
    };
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare_incremental(
            crate::schema::judge::LawfulParent::established(),
            changes,
            &UnindexedRows,
            &judge,
        )
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => Ok(prepared
            .seal(NO_HOST)
            .expect("seal")
            .commit()
            .expect("commit")),
        Prepared::Rejected(violations) => Err(violations.into_vec()),
    }
}

#[test]
fn two_fresh_rows_sharing_an_email_reject_with_the_key_statement() {
    // The historical E-ADMIT counterexample, physical edition: two new
    // users with distinct ids but one email. Both proposed rows exist in
    // the candidate multimap; the judge cites the email key with both
    // facts. No first-installed-wins ambiguity is representable.
    let (_dir, path) = store_dir("judged-shared-email");
    let store = user_store(&path);
    let violations = judged_commit(
        &store,
        &user_changes(&[user(1, "a@example"), user(2, "a@example")], &[]),
    )
    .expect_err("the shared email must reject");
    assert_eq!(violations.len(), 1);
    let violation = &violations[0];
    assert_eq!(violation.statement, StatementId(1), "the email key");
    assert_eq!(violation.examples.len(), 2, "both competing rows cited");
    // The losing candidate left no trace.
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(USER).expect("count"), 0);
}

#[test]
fn both_violated_statements_are_reported_together() {
    // Duplicate id AND duplicate email in one proposal: the completed
    // rejection names the complete set of violated statements, not the
    // first physical conflict encountered.
    let (_dir, path) = store_dir("judged-both-statements");
    let store = user_store(&path);
    let violations = judged_commit(
        &store,
        &user_changes(
            &[
                user(7, "x@example"),
                user(7, "y@example"),
                user(8, "x@example"),
            ],
            &[],
        ),
    )
    .expect_err("both keys must reject");
    let statements: Vec<_> = violations
        .iter()
        .map(|violation| violation.statement)
        .collect();
    assert_eq!(statements, vec![StatementId(0), StatementId(1)]);
}

#[test]
fn a_key_replacement_admits_in_either_spelling_order() {
    // delete(old) + insert(new) under one key admits: the judge sees the
    // final state, not statement order (chapter 10 §1 point 2).
    let (_dir, path) = store_dir("judged-replacement");
    let store = user_store(&path);
    judged_commit(&store, &user_changes(&[user(1, "old@example")], &[])).expect("initial admit");
    judged_commit(
        &store,
        &user_changes(&[user(1, "new@example")], &[user(1, "old@example")]),
    )
    .expect("replacement admits");
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(USER).expect("count"), 1);
}

#[test]
fn a_conflict_with_a_committed_row_rejects_and_leaves_it_intact() {
    let (_dir, path) = store_dir("judged-committed-conflict");
    let store = user_store(&path);
    judged_commit(&store, &user_changes(&[user(1, "held@example")], &[])).expect("initial admit");
    let violations = judged_commit(&store, &user_changes(&[user(2, "held@example")], &[]))
        .expect_err("email is held");
    assert_eq!(violations[0].statement, StatementId(1));
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(USER).expect("count"), 1);
}

#[test]
fn judgment_over_the_store_agrees_with_the_reference_map_state() {
    // The physical candidate view and P01's owned MapState are the same
    // denotation: one delta, two states, one verdict.
    use crate::schema::judge::{Judgment as SchemaJudgment, MapState, judge_final_state};
    let (_dir, path) = store_dir("judged-map-agreement");
    let store = user_store(&path);
    judged_commit(&store, &user_changes(&[user(1, "a@example")], &[])).expect("seed admit");
    let schema = user_schema();
    // Proposed final state: {user1, user2-with-same-email}.
    let mut reference = MapState::new();
    reference.insert(USER, user(1, "a@example"));
    reference.insert(USER, user(2, "a@example"));
    let reference_verdict = judge_final_state(&schema, &reference, &work(), JudgeBudget::default())
        .expect("reference judgment");
    let store_verdict = judged_commit(&store, &user_changes(&[user(2, "a@example")], &[]));
    match (reference_verdict, store_verdict) {
        (SchemaJudgment::Rejected(reference_violations), Err(store_violations)) => {
            let reference_ids: Vec<_> = reference_violations
                .iter()
                .map(|violation| violation.statement)
                .collect();
            let store_ids: Vec<_> = store_violations
                .iter()
                .map(|violation| violation.statement)
                .collect();
            assert_eq!(reference_ids, store_ids);
        }
        other => panic!("both judges must reject identically, got {other:?}"),
    }
}
