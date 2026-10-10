//! Determinant entries stay in step with rows: every non-home key
//! projection keeps one `det` entry per live row, groups resolve through
//! buckets, and the sweeper finds nothing after any mix of mutations.

use super::*;
use crate::schema::{FieldId, StatementDescriptor};

const USER: RelationId = RelationId(0);

/// `User { id: u64, email: str, region: u64 }`, keyed on id, email and
/// `(region, id)`.
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
                FieldDescriptor {
                    name: "region".into(),
                    value_type: ValueType::U64,
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
            StatementDescriptor::Functionality {
                relation: USER,
                projection: Box::from([FieldId(2), FieldId(0)]),
            },
        ],
    }
    .validate()
    .expect("user schema validates")
}

fn user(id: u64, email: &str, region: u64) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::String(email.into()),
        Value::U64(region),
    ]
}

fn det_entries(store: &Store) -> u64 {
    let snapshot = store.snapshot(&work()).expect("snapshot");
    store
        .inner
        .dets
        .len(snapshot.read_txn())
        .expect("det count")
}

fn sweep_is_clean(store: &Store, schema: &Schema) {
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let findings = super::super::verify::sweep(&snapshot, schema, &work()).expect("sweep");
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
#[cfg_attr(miri, ignore)]
fn every_non_home_key_keeps_one_entry_per_row_through_replacement() {
    let schema = user_schema();
    let (_dir, path) = store_dir("indexed-entries");
    let store = create_with(&path, &schema);
    commit_changes(
        &store,
        &change_set(
            &schema,
            &[(USER, user(1, "a@x", 7)), (USER, user(2, "b@x", 7))],
            &[],
        ),
    );
    // The id key is the home; email and (region, id) keep entries.
    assert_eq!(det_entries(&store), 4);
    sweep_is_clean(&store, &schema);
    commit_changes(
        &store,
        &change_set(
            &schema,
            &[(USER, user(1, "c@x", 8))],
            &[(USER, user(1, "a@x", 7))],
        ),
    );
    assert_eq!(det_entries(&store), 4);
    sweep_is_clean(&store, &schema);
    commit_changes(
        &store,
        &change_set(
            &schema,
            &[],
            &[(USER, user(1, "c@x", 8)), (USER, user(2, "b@x", 7))],
        ),
    );
    assert_eq!(det_entries(&store), 0);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert!(
        store
            .inner
            .rows
            .is_empty(snapshot.read_txn())
            .expect("rows")
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn groups_resolve_through_buckets_and_long_text_never_enters_a_key() {
    let schema = user_schema();
    let (_dir, path) = store_dir("indexed-groups");
    let store = create_with(&path, &schema);
    let long = "e".repeat(4096);
    commit_changes(
        &store,
        &change_set(
            &schema,
            &[(USER, user(1, &long, 3)), (USER, user(2, "short", 3))],
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let theory = schema.compiled_theory().expect("compiled");
    let email = theory
        .projection_of_statement(bumbledb_theory::schema::StatementId(1))
        .expect("email projection");
    let projected = super::super::det_index::determinant_bytes(
        email,
        &[Value::String(long.as_str().into())],
        &work(),
    )
    .expect("projected");
    let mut hits = Vec::new();
    snapshot
        .visit_projection(email.id, &projected, &work(), &mut |_, bytes| {
            hits.push(crate::canonical::decode(
                schema.relation(USER).fields(),
                bytes,
                &work(),
            )?);
            Ok(true)
        })
        .expect("visit");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].values()[0], Value::U64(1));
    let mut keys = Vec::new();
    for entry in store.inner.dets.iter(snapshot.read_txn()).expect("iter") {
        keys.push(entry.expect("entry").0.len());
    }
    assert!(keys.iter().all(|len| *len == super::super::keys::KEY_LEN));
}

#[test]
#[cfg_attr(miri, ignore)]
fn randomized_mutations_keep_the_store_coherent() {
    let schema = user_schema();
    let (_dir, path) = store_dir("indexed-random");
    let store = create_with(&path, &schema);
    let mut live: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..64 {
        let id = next() % 24;
        let region = next() % 4;
        let row = (USER, user(id, &format!("{id}@{region}"), region));
        let changes = if live.contains(&(id, region)) {
            live.remove(&(id, region));
            change_set(&schema, &[], &[row])
        } else {
            live.insert((id, region));
            change_set(&schema, &[row], &[])
        };
        commit_changes(&store, &changes);
    }
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(USER).expect("count"), live.len() as u64);
    drop(snapshot);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let findings = super::super::verify::sweep(&snapshot, &schema, &work()).expect("sweep");
    assert!(
        findings
            .iter()
            .all(|finding| matches!(finding, super::super::verify::VerifyFinding::Judgment(_))),
        "unjudged commits may break laws, never the physical index: {findings:?}"
    );
}
