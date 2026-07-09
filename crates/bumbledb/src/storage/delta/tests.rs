use super::*;
use crate::encoding::{encode_fact, encode_u64, ValueRef};
use crate::error::Error;
use crate::schema::{FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType};
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;

/// R(id serial, amount i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
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

const R: RelationId = RelationId(0);
const ID: FieldId = FieldId(0);

fn fact(schema: &Schema, id: u64, amount: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::U64(id), ValueRef::I64(amount)],
        schema.relation(R).layout(),
        &mut bytes,
    );
    bytes
}

fn data_snapshot(env: &Environment) -> Vec<(Vec<u8>, Vec<u8>)> {
    let rtxn = env.read_txn().expect("txn");
    env.data()
        .iter(rtxn.raw())
        .expect("iter")
        .map(|kv| {
            let (k, v) = kv.expect("kv");
            (k.to_vec(), v.to_vec())
        })
        .collect()
}

#[test]
fn insert_then_delete_of_absent_fact_nets_noop_and_reports_true_true() {
    let dir = TempDir::new("delta-insert-delete");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);
    assert!(delta.insert(&view, R, &f).expect("insert"));
    assert!(delta.delete(&view, R, &f).expect("delete"));
    // Net disposition is Delete for a fact not in base: apply's base
    // check makes it a no-op (docs/architecture/50-storage.md).
    assert_eq!(delta.disposition(R, &f), Some(Disposition::Delete));
}

#[test]
fn idempotent_double_insert_reports_true_false() {
    let dir = TempDir::new("delta-double-insert");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);
    assert!(delta.insert(&view, R, &f).expect("insert"));
    assert!(!delta.insert(&view, R, &f).expect("insert"));
}

#[test]
fn disposition_last_wins_across_long_sequences() {
    let dir = TempDir::new("delta-last-wins");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);
    for _ in 0..7 {
        delta.insert(&view, R, &f).expect("insert");
        delta.delete(&view, R, &f).expect("delete");
    }
    delta.insert(&view, R, &f).expect("insert");
    assert_eq!(delta.disposition(R, &f), Some(Disposition::Insert));
    delta.delete(&view, R, &f).expect("delete");
    assert_eq!(delta.disposition(R, &f), Some(Disposition::Delete));
}

#[test]
fn alloc_is_strictly_increasing_and_reads_q_once() {
    let dir = TempDir::new("delta-alloc");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 0);
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 1);
    drop(view);

    // Bump the committed Q value behind the delta's back: the cached
    // in-memory next must win — Q is read once per (relation, field).
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut buf = [0u8; keys::SERIAL_KEY_LEN];
        let len = keys::serial_key(&mut buf, R, ID);
        env.data()
            .put(wtxn.raw_mut(), &buf[..len], 100u64.to_le_bytes().as_slice())
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 2);

    // A fresh delta sees the committed value.
    let mut fresh = WriteDelta::new(&schema);
    assert_eq!(fresh.alloc(&view, R, ID).expect("alloc"), 100);
}

#[test]
fn explicit_value_above_mark_advances_generated_successors() {
    let dir = TempDir::new("delta-explicit");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert!(delta
        .insert(&view, R, &fact(&schema, 50, 1))
        .expect("insert"));
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 51);
}

#[test]
fn mixed_explicit_and_generated_allocation_tracks_running_maximum() {
    let dir = TempDir::new("delta-mixed");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 0);
    delta
        .insert(&view, R, &fact(&schema, 10, 1))
        .expect("insert");
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 11);
    // An explicit value *below* the mark must not regress it.
    delta
        .insert(&view, R, &fact(&schema, 3, 2))
        .expect("insert");
    assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 12);
}

#[test]
fn explicit_max_exhausts_the_generator() {
    let dir = TempDir::new("delta-exhausted");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .insert(&view, R, &fact(&schema, u64::MAX, 1))
        .expect("insert");
    let err = delta.alloc(&view, R, ID).unwrap_err();
    assert!(
        matches!(
            err,
            Error::SerialExhausted {
                relation: R,
                field: ID
            }
        ),
        "{err:?}"
    );
}

#[test]
fn resolve_never_mints_and_sees_both_id_sources() {
    let dir = TempDir::new("delta-resolve");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let delta = WriteDelta::new(&schema);

    // A double miss proves the value unknown — and mints nothing.
    assert_eq!(delta.resolve_str(&view, "ghost").expect("resolve"), None);
    assert_eq!(delta.dict_next(), None, "resolve minted a provisional id");
    assert_eq!(delta.pending_interns().count(), 0);

    // A pending hit returns the provisional id (cancellation works).
    let mut delta = delta;
    let pending = delta.intern_str(&view, "novel").expect("intern");
    assert_eq!(
        delta.resolve_str(&view, "novel").expect("resolve"),
        Some(pending)
    );

    // A committed hit returns the committed id.
    drop(view);
    {
        let mut wtxn = env.write_txn().expect("txn");
        crate::storage::dict::intern_str(&mut wtxn, "committed").expect("intern");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let fresh = WriteDelta::new(&schema);
    assert!(fresh
        .resolve_str(&view, "committed")
        .expect("resolve")
        .is_some());
    assert_eq!(fresh.dict_next(), None);
}

#[test]
fn dirty_serial_marks_are_exactly_the_advanced_sequences() {
    let dir = TempDir::new("delta-dirty-marks");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Committed base: Q = 6.
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut buf = [0u8; keys::SERIAL_KEY_LEN];
        let len = keys::serial_key(&mut buf, R, ID);
        env.data()
            .put(wtxn.raw_mut(), &buf[..len], 6u64.to_le_bytes().as_slice())
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");

    // An explicit value below the base reads the mark but advances
    // nothing: clean.
    let mut clean = WriteDelta::new(&schema);
    clean
        .insert(&view, R, &fact(&schema, 3, 1))
        .expect("insert");
    assert_eq!(clean.serial_marks().count(), 1, "the mark was read");
    assert_eq!(clean.dirty_serial_marks().count(), 0, "but never advanced");

    // An allocation advances past the base: dirty.
    let mut dirty = WriteDelta::new(&schema);
    assert_eq!(dirty.alloc(&view, R, ID).expect("alloc"), 6);
    assert_eq!(
        dirty.dirty_serial_marks().collect::<Vec<_>>(),
        vec![(R, ID, 7)]
    );
}

#[test]
fn guard_map_mirrors_the_fact_dispositions() {
    // The serial auto-key on `id` is StatementId(0) (materialized first).
    const KEY: StatementId = StatementId(0);
    let dir = TempDir::new("delta-guard-map");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 7, 700);
    let guard = encode_u64(7);

    // Untouched tuple: no overlay — the committed state answers.
    assert_eq!(delta.guard_overlay(KEY, &guard), None);

    // Insert records the establishing fact; delete records absence;
    // last disposition wins, mirroring the fact map.
    delta.insert(&view, R, &f).expect("insert");
    assert_eq!(
        delta.guard_overlay(KEY, &guard),
        Some(GuardOverlay::Present(f.as_slice()))
    );
    delta.delete(&view, R, &f).expect("delete");
    assert_eq!(delta.guard_overlay(KEY, &guard), Some(GuardOverlay::Absent));

    // Delete + re-insert under the same key with a changed non-key field:
    // the tuple is re-established by the *new* fact (the upsert shape).
    let g = fact(&schema, 7, 999);
    delta.insert(&view, R, &g).expect("insert");
    assert_eq!(
        delta.guard_overlay(KEY, &guard),
        Some(GuardOverlay::Present(g.as_slice()))
    );

    // A no-op operation records nothing: deleting an absent fact must
    // not shadow another fact's live key tuple.
    let mut idle = WriteDelta::new(&schema);
    assert!(!idle
        .delete(&view, R, &fact(&schema, 9, 900))
        .expect("delete"));
    assert_eq!(idle.guard_overlay(KEY, &encode_u64(9)), None);
}

#[test]
fn deleting_the_old_fact_never_erases_the_new_facts_guard_record() {
    // `delete(old); insert(new)` is blessed in either order — a point
    // read of the shared key tuple must see `new` whichever ran last.
    const KEY: StatementId = StatementId(0);
    let dir = TempDir::new("delta-guard-order");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = fact(&schema, 7, 700);
    let new = fact(&schema, 7, 999);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &old).expect("insert");
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");
    for insert_first in [true, false] {
        let mut delta = WriteDelta::new(&schema);
        if insert_first {
            delta.insert(&view, R, &new).expect("insert");
            delta.delete(&view, R, &old).expect("delete");
        } else {
            delta.delete(&view, R, &old).expect("delete");
            delta.insert(&view, R, &new).expect("insert");
        }
        assert_eq!(
            delta.guard_overlay(KEY, &encode_u64(7)),
            Some(GuardOverlay::Present(new.as_slice())),
            "insert_first = {insert_first}"
        );
    }
}

#[test]
fn drop_leaves_lmdb_untouched() {
    let dir = TempDir::new("delta-drop");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let before = data_snapshot(&env);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for i in 0i64..100 {
            delta
                .insert(&view, R, &fact(&schema, i.cast_unsigned(), i))
                .expect("insert");
        }
        delta.alloc(&view, R, ID).expect("alloc");
        delta
            .delete(&view, R, &fact(&schema, 5, 5))
            .expect("delete");
        // Abort = drop: nothing was ever written.
    }
    assert_eq!(before, data_snapshot(&env));
    assert!(before.is_empty());
}
