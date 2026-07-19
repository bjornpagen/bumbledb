use super::*;
use crate::encoding::{ValueRef, encode_fact, encode_u64};
use crate::error::Error;
use crate::schema::KeyId;
use crate::schema::ValidateDescriptor as _;
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};

/// R(id fresh, amount i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
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
fn insert_then_delete_of_absent_fact_cancels_to_an_empty_delta() {
    let dir = TempDir::new("delta-insert-delete");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);
    assert!(delta.insert(&view, R, &f).expect("insert"));
    // The delete cancels the pending Insert: the net effect against
    // committed state is nothing, so nothing is recorded
    // (docs/architecture/50-storage.md net dispositions).
    assert!(delta.delete(&view, R, &f).expect("delete"));
    assert_eq!(delta.disposition(R, &f), None);
    assert!(delta.is_empty());
}

#[test]
fn delete_then_insert_of_a_committed_fact_cancels_to_an_empty_delta() {
    let dir = TempDir::new("delta-delete-insert");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let f = fact(&schema, 1, 100);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &f).expect("insert");
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert!(delta.delete(&view, R, &f).expect("delete"));
    // The re-insert cancels the pending Delete — a no-op insert is
    // unrepresentable, never recorded and never judged
    // (docs/architecture/50-storage.md net dispositions).
    assert!(delta.insert(&view, R, &f).expect("insert"));
    assert_eq!(delta.disposition(R, &f), None);
    assert!(delta.is_empty());
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
fn long_alternating_sequences_net_against_committed_state() {
    let dir = TempDir::new("delta-net-sequences");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);
    // Each insert/delete pair on an absent fact cancels: the delta stays
    // net-empty however long the sequence runs.
    for _ in 0..7 {
        assert!(delta.insert(&view, R, &f).expect("insert"));
        assert!(delta.delete(&view, R, &f).expect("delete"));
        assert_eq!(delta.disposition(R, &f), None);
    }
    // An unpaired trailing op records its genuine net effect.
    assert!(delta.insert(&view, R, &f).expect("insert"));
    assert_eq!(delta.disposition(R, &f), Some(Disposition::Insert));
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
        let mut buf = [0u8; keys::FRESH_KEY_LEN];
        let len = keys::fresh_key(&mut buf, R, ID);
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
    assert!(
        delta
            .insert(&view, R, &fact(&schema, 50, 1))
            .expect("insert")
    );
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
            Error::FreshExhausted {
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
    assert!(
        fresh
            .resolve_str(&view, "committed")
            .expect("resolve")
            .is_some()
    );
    assert_eq!(fresh.dict_next(), None);
}

#[test]
fn dirty_fresh_marks_are_exactly_the_advanced_sequences() {
    let dir = TempDir::new("delta-dirty-marks");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Committed base: Q = 6.
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut buf = [0u8; keys::FRESH_KEY_LEN];
        let len = keys::fresh_key(&mut buf, R, ID);
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
    assert_eq!(clean.fresh_marks().count(), 1, "the mark was read");
    assert_eq!(clean.dirty_fresh_marks().count(), 0, "but never advanced");

    // An allocation advances past the base: dirty.
    let mut dirty = WriteDelta::new(&schema);
    assert_eq!(dirty.alloc(&view, R, ID).expect("alloc"), 6);
    assert_eq!(
        dirty.dirty_fresh_marks().collect::<Vec<_>>(),
        vec![(R, ID, 7)]
    );
}

#[test]
fn determinant_map_mirrors_the_fact_dispositions() {
    // The fresh auto-key on `id` is the first typed key witness.
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-determinant-map");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 7, 700);
    let determinant = encode_u64(7);

    // Untouched tuple: no overlay — the committed state answers.
    assert_eq!(delta.determinant_overlay(KEY, &determinant), None);

    // Insert records the establishing fact; the canceling delete restores
    // the tuple's pre-insert overlay — nothing touched it, so the overlay
    // vanishes and the committed state answers (here: absent).
    delta.insert(&view, R, &f).expect("insert");
    assert_eq!(
        delta.determinant_overlay(KEY, &determinant),
        Some(DeterminantOverlay::Present(f.as_slice()))
    );
    delta.delete(&view, R, &f).expect("delete");
    assert_eq!(delta.determinant_overlay(KEY, &determinant), None);

    // Insert again under the same key with a changed non-key field:
    // the tuple is re-established by the *new* fact (the upsert shape).
    let g = fact(&schema, 7, 999);
    delta.insert(&view, R, &g).expect("insert");
    assert_eq!(
        delta.determinant_overlay(KEY, &determinant),
        Some(DeterminantOverlay::Present(g.as_slice()))
    );

    // A no-op operation records nothing: deleting an absent fact must
    // not shadow another fact's live key tuple.
    let mut idle = WriteDelta::new(&schema);
    assert!(
        !idle
            .delete(&view, R, &fact(&schema, 9, 900))
            .expect("delete")
    );
    assert_eq!(idle.determinant_overlay(KEY, &encode_u64(9)), None);
}

#[test]
fn deleting_the_old_fact_never_erases_the_new_facts_determinant_record() {
    // `delete(old); insert(new)` is blessed in either order — a point
    // read of the shared key tuple must see `new` whichever ran last.
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-determinant-order");
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
            delta.determinant_overlay(KEY, &encode_u64(7)),
            Some(DeterminantOverlay::Present(new.as_slice())),
            "insert_first = {insert_first}"
        );
    }
}

#[test]
fn a_cancelled_insert_never_shadows_the_committed_owner_of_its_key_tuple() {
    // Regression: committed {7,700}; a pending insert of a same-key fact
    // is cancelled by its compensating delete — net nothing. The tuple's
    // overlay must vanish so the committed owner answers; recording
    // `Absent` would deny a live committed row to every point read in the
    // transaction (the blocker repro shape).
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-cancel-committed-owner");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = fact(&schema, 7, 700);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &old).expect("insert");
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let transient = fact(&schema, 7, 999);
    assert!(delta.insert(&view, R, &transient).expect("insert"));
    assert!(delta.delete(&view, R, &transient).expect("delete"));
    assert!(delta.is_empty(), "the pair cancelled to nothing");
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        None,
        "no overlay: the committed owner of key 7 answers"
    );
}

#[test]
fn a_cancelled_insert_restores_an_earlier_pending_owner() {
    // Two pending inserts share the key tuple (commit-doomed, but
    // representable pre-commit); cancelling the later one re-establishes
    // the earlier — the surviving pending fact is the final-state owner.
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-cancel-pending-owner");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let first = fact(&schema, 7, 700);
    let second = fact(&schema, 7, 999);
    delta.insert(&view, R, &first).expect("insert");
    delta.insert(&view, R, &second).expect("insert");
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        Some(DeterminantOverlay::Present(second.as_slice()))
    );
    delta.delete(&view, R, &second).expect("delete");
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        Some(DeterminantOverlay::Present(first.as_slice())),
        "the earlier pending insert owns the tuple again"
    );
}

#[test]
fn a_cancelled_insert_keeps_a_pending_deletes_absence() {
    // delete(old); insert(new); delete(new): the cancel must restore the
    // pending delete's absence, not erase it — the final state drops old.
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-cancel-keeps-absence");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = fact(&schema, 7, 700);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &old).expect("insert");
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let new = fact(&schema, 7, 999);
    delta.delete(&view, R, &old).expect("delete");
    delta.insert(&view, R, &new).expect("insert");
    delta.delete(&view, R, &new).expect("delete");
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        Some(DeterminantOverlay::Absent),
        "the pending delete of the committed owner still stands"
    );
}

#[test]
fn determinant_overwrites_never_reclone_the_scratch() {
    // The scratch's no-per-key-statement allocation contract, pinned:
    // the determinant map clones the scratch exactly once per distinct
    // resident tuple; every later disposition that moves a RESIDENT
    // entry — the upsert shape's overwrite and cancel-restore — takes
    // the no-clone path. (A cancellation with no prior disposition
    // removes the entry instead; the cancel trio above pins that law.)
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-determinant-clone");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = fact(&schema, 7, 700);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &old).expect("insert");
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let new = fact(&schema, 7, 999); // same key tuple, changed non-key field

    delta.delete(&view, R, &old).expect("delete");
    assert_eq!(delta.determinant_scratch_clones, 1, "first record clones");

    // The tuple stays resident across the upsert shape — dispositions
    // move in place, clones do not.
    delta.insert(&view, R, &new).expect("insert");
    delta.delete(&view, R, &new).expect("delete");
    assert_eq!(
        delta.determinant_scratch_clones, 1,
        "resident re-dispositions take the no-insert path"
    );
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        Some(DeterminantOverlay::Absent),
        "correctness unchanged: the pending delete stands"
    );

    // A distinct tuple is a genuine first record: one more clone.
    delta
        .insert(&view, R, &fact(&schema, 8, 800))
        .expect("insert");
    assert_eq!(delta.determinant_scratch_clones, 2);
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

const A: RelationId = RelationId(0);
const B: RelationId = RelationId(1);

/// A(v u64) + B(v u64) — two ordinary relations for the per-relation
/// delete classification.
fn two_relation_schema() -> Schema {
    let rel = |name: &str| RelationDescriptor {
        extension: None,
        name: name.into(),
        fields: vec![FieldDescriptor {
            name: "v".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        }],
    };
    SchemaDescriptor {
        relations: vec![rel("A"), rel("B")],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn u64_fact(schema: &Schema, rel: RelationId, v: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::U64(v)],
        schema.relation(rel).layout(),
        &mut bytes,
    );
    bytes
}

/// The image cache's dirty classification is the delta's net delete set
/// projected to relations: deduplicated, ascending, and exactly the
/// relations a fact is removed from — the discriminator the copy-on-append
/// path stands on (docs/architecture/50-storage.md § the image cache).
#[test]
fn dirty_relations_are_the_deleted_from_relations_deduped_ascending() {
    let dir = TempDir::new("delta-dirty");
    let schema = two_relation_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for v in 0..3 {
            delta
                .insert(&view, A, &u64_fact(&schema, A, v))
                .expect("insert");
            delta
                .insert(&view, B, &u64_fact(&schema, B, v))
                .expect("insert");
        }
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");

    // Insert-only: nothing is dirty, whatever the volume.
    let mut delta = WriteDelta::new(&schema);
    for v in 10..20 {
        delta
            .insert(&view, A, &u64_fact(&schema, A, v))
            .expect("insert");
    }
    assert_eq!(delta.dirty_relations(), vec![]);

    // Multiple deletes in one relation dedup to one entry; both
    // relations deleted-from report ascending; inserts never dirty.
    let mut delta = WriteDelta::new(&schema);
    delta
        .insert(&view, A, &u64_fact(&schema, A, 10))
        .expect("insert");
    delta
        .delete(&view, B, &u64_fact(&schema, B, 0))
        .expect("delete");
    delta
        .delete(&view, A, &u64_fact(&schema, A, 0))
        .expect("delete");
    delta
        .delete(&view, A, &u64_fact(&schema, A, 1))
        .expect("delete");
    assert_eq!(delta.dirty_relations(), vec![A, B]);
}

/// Cancellation is exact: a delete-then-reinsert of the same committed
/// fact nets to no entry, so its relation is NOT dirty — no false
/// positives from cancelled pairs (the delta's net-disposition
/// invariant), and the untouched relation's image survives as an append
/// base.
#[test]
fn a_cancelled_delete_reinsert_pair_dirties_nothing() {
    let dir = TempDir::new("delta-dirty-cancel");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let f = fact(&schema, 1, 100);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &f).expect("insert");
        drop(view);
        crate::storage::commit::commit(delta, &env).expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert!(delta.delete(&view, R, &f).expect("delete"));
    assert_eq!(
        delta.dirty_relations(),
        vec![R],
        "a live pending delete dirties its relation"
    );
    assert!(delta.insert(&view, R, &f).expect("insert"));
    assert_eq!(
        delta.dirty_relations(),
        vec![],
        "the reinsert cancelled the delete — nothing is removed from R"
    );
}
