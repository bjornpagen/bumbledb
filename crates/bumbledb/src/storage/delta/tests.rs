use super::*;
use crate::encoding::{InternId, ValueRef, encode_fact, encode_u64};
use crate::error::Error;
use crate::schema::KeyId;
use crate::schema::ValidateDescriptor as _;
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};
use std::num::NonZeroU64;

fn one() -> NonZeroU64 {
    NonZeroU64::new(1).unwrap()
}

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
    assert_eq!(
        delta.insert(&view, R, &f).expect("insert"),
        DeltaEffect::Recorded
    );

    assert_eq!(
        delta.delete(&view, R, &f).expect("delete"),
        DeltaEffect::Cancelled
    );
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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(
        delta.delete(&view, R, &f).expect("delete"),
        DeltaEffect::Recorded
    );

    assert_eq!(
        delta.insert(&view, R, &f).expect("insert"),
        DeltaEffect::Cancelled
    );
    assert_eq!(delta.disposition(R, &f), None);
    assert!(delta.is_empty());
}

#[test]
fn idempotent_double_insert_reports_recorded_then_noop() {
    let dir = TempDir::new("delta-double-insert");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);
    assert_eq!(
        delta.insert(&view, R, &f).expect("insert"),
        DeltaEffect::Recorded
    );
    assert_eq!(
        delta.insert(&view, R, &f).expect("insert"),
        DeltaEffect::NoOp
    );
}

#[test]
fn long_alternating_sequences_net_against_committed_state() {
    let dir = TempDir::new("delta-net-sequences");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 1, 100);

    for _ in 0..7 {
        assert_eq!(
            delta.insert(&view, R, &f).expect("insert"),
            DeltaEffect::Recorded
        );
        assert_eq!(
            delta.delete(&view, R, &f).expect("delete"),
            DeltaEffect::Cancelled
        );
        assert_eq!(delta.disposition(R, &f), None);
    }

    assert_eq!(
        delta.insert(&view, R, &f).expect("insert"),
        DeltaEffect::Recorded
    );
    assert_eq!(delta.disposition(R, &f), Some(Disposition::Insert));
}

#[test]
fn reserve_is_strictly_increasing_and_reads_q_once() {
    let dir = TempDir::new("delta-alloc");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 0);
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 1);
    drop(view);

    {
        let mut wtxn = env.write_txn().expect("txn");
        let key = keys::fresh_key(R, ID);
        env.data()
            .put(wtxn.raw_mut(), &key, 100u64.to_le_bytes().as_slice())
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 2);

    let mut fresh = WriteDelta::new(&schema);
    assert_eq!(fresh.reserve(&view, R, ID, one()).expect("reserve"), 100);
}

#[test]
fn explicit_value_above_mark_advances_generated_successors() {
    let dir = TempDir::new("delta-explicit");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(
        delta
            .insert(&view, R, &fact(&schema, 50, 1))
            .expect("insert"),
        DeltaEffect::Recorded
    );
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 51);
}

#[test]
fn mixed_explicit_and_generated_reserve_tracks_running_maximum() {
    let dir = TempDir::new("delta-mixed");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 0);
    delta
        .insert(&view, R, &fact(&schema, 10, 1))
        .expect("insert");
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 11);
    // An explicit value *below* the mark must not regress it.
    delta
        .insert(&view, R, &fact(&schema, 3, 2))
        .expect("insert");
    assert_eq!(delta.reserve(&view, R, ID, one()).expect("reserve"), 12);
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
    let err = delta.reserve(&view, R, ID, one()).unwrap_err();
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

    assert_eq!(delta.resolve_str(&view, "ghost").expect("resolve"), None);
    assert_eq!(delta.dict_next(), None, "resolve minted a provisional id");
    assert_eq!(delta.pending_interns().count(), 0);

    let mut delta = delta;
    let pending = delta.intern_str(&view, "novel").expect("intern");
    assert_eq!(
        delta.resolve_str(&view, "novel").expect("resolve"),
        Some(pending)
    );

    drop(view);
    {
        let view = env.read_txn().expect("txn");
        let mut seeder = WriteDelta::new(&schema);
        seeder.intern_str(&view, "committed").expect("intern");
        drop(view);
        let mut wtxn = env.write_txn().expect("txn");
        for (raw, id) in seeder.pending_interns() {
            crate::storage::dict::put_pending(&mut wtxn, raw, id).expect("flush");
        }
        wtxn.put_dict_next_id(seeder.dict_next().expect("minted"))
            .expect("advance");
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

fn seed_committed(env: &Environment, schema: &Schema, value: &str) -> InternId {
    let view = env.read_txn().expect("txn");
    let mut seeder = WriteDelta::new(schema);
    let id = seeder.intern_str(&view, value).expect("intern");
    drop(view);
    let mut wtxn = env.write_txn().expect("txn");
    for (raw, pending) in seeder.pending_interns() {
        crate::storage::dict::put_pending(&mut wtxn, raw, pending).expect("flush");
    }
    wtxn.put_dict_next_id(seeder.dict_next().expect("minted"))
        .expect("advance");
    wtxn.commit().expect("commit");
    id
}

#[test]
fn a_committed_string_interned_twice_probes_the_dict_once() {

    let dir = TempDir::new("delta-memo-committed");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let committed = seed_committed(&env, &schema, "hello");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.intern_str(&view, "hello").expect("intern"), committed);
    assert_eq!(delta.intern_str(&view, "hello").expect("intern"), committed);
    assert_eq!(
        delta.resolve_str(&view, "hello").expect("resolve"),
        Some(committed)
    );
    assert_eq!(delta.dict_next(), None, "a committed hit mints nothing");
}

#[test]
fn a_pending_string_answers_before_the_memo() {

    let dir = TempDir::new("delta-memo-pending-first");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let minted = delta.intern_str(&view, "novel").expect("intern");
    assert_eq!(delta.intern_str(&view, "novel").expect("intern"), minted);
    assert_eq!(delta.dict_next(), Some(minted.raw() + 1), "one mint");
}

#[test]
fn committed_misses_are_never_memoized() {

    let dir = TempDir::new("delta-memo-miss");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let delta = WriteDelta::new(&schema);
    assert_eq!(delta.resolve_str(&view, "ghost").expect("resolve"), None);
    assert_eq!(delta.resolve_str(&view, "ghost").expect("resolve"), None);
    assert_eq!(delta.dict_next(), None, "resolve minted nothing");
}

#[test]
fn a_dropped_deltas_memo_leaves_no_trace() {

    let dir = TempDir::new("delta-memo-drop");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let committed = seed_committed(&env, &schema, "hello");
    let view = env.read_txn().expect("txn");
    {
        let mut delta = WriteDelta::new(&schema);
        delta.intern_str(&view, "hello").expect("intern");
        // Abort = drop: the memo dies with the delta.
    }
    let mut later = WriteDelta::new(&schema);
    assert_eq!(later.intern_str(&view, "hello").expect("intern"), committed);
}

#[test]
fn dirty_fresh_marks_are_exactly_the_advanced_sequences() {
    let dir = TempDir::new("delta-dirty-marks");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");

    {
        let mut wtxn = env.write_txn().expect("txn");
        let key = keys::fresh_key(R, ID);
        env.data()
            .put(wtxn.raw_mut(), &key, 6u64.to_le_bytes().as_slice())
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");

    let mut clean = WriteDelta::new(&schema);
    clean
        .insert(&view, R, &fact(&schema, 3, 1))
        .expect("insert");
    assert_eq!(clean.fresh_marks().count(), 1, "the mark was read");
    assert_eq!(clean.dirty_fresh_marks().count(), 0, "but never advanced");

    let mut dirty = WriteDelta::new(&schema);
    assert_eq!(dirty.reserve(&view, R, ID, one()).expect("reserve"), 6);
    assert_eq!(
        dirty.dirty_fresh_marks().collect::<Vec<_>>(),
        vec![(R, ID, 7)]
    );
}

#[test]
fn determinant_map_mirrors_the_fact_dispositions() {

    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-determinant-map");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let f = fact(&schema, 7, 700);
    let determinant = encode_u64(7);

    assert_eq!(delta.determinant_overlay(KEY, &determinant), None);

    delta.insert(&view, R, &f).expect("insert");
    assert_eq!(
        delta.determinant_overlay(KEY, &determinant),
        Some(DeterminantOverlay::Present(f.as_slice()))
    );
    delta.delete(&view, R, &f).expect("delete");
    assert_eq!(delta.determinant_overlay(KEY, &determinant), None);

    let g = fact(&schema, 7, 999);
    delta.insert(&view, R, &g).expect("insert");
    assert_eq!(
        delta.determinant_overlay(KEY, &determinant),
        Some(DeterminantOverlay::Present(g.as_slice()))
    );

    let mut idle = WriteDelta::new(&schema);
    assert_eq!(
        idle.delete(&view, R, &fact(&schema, 9, 900))
            .expect("delete"),
        DeltaEffect::NoOp
    );
    assert_eq!(idle.determinant_overlay(KEY, &encode_u64(9)), None);
}

#[test]
fn deleting_the_old_fact_never_erases_the_new_facts_determinant_record() {

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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let transient = fact(&schema, 7, 999);
    assert_eq!(
        delta.insert(&view, R, &transient).expect("insert"),
        DeltaEffect::Recorded
    );
    assert_eq!(
        delta.delete(&view, R, &transient).expect("delete"),
        DeltaEffect::Cancelled
    );
    assert!(delta.is_empty(), "the pair cancelled to nothing");
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        None,
        "no overlay: the committed owner of key 7 answers"
    );
}

#[test]
fn a_cancelled_insert_restores_an_earlier_pending_owner() {

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
fn cancelling_a_replaced_insert_does_not_restore_it() {

    // replaced stack. Overlay stays B; cancelling B must not resurrect A.
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-cancel-replaced-insert");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let first = fact(&schema, 7, 700);
    let second = fact(&schema, 7, 999);
    delta.insert(&view, R, &first).expect("insert");
    delta.insert(&view, R, &second).expect("insert");
    assert_eq!(
        delta.delete(&view, R, &first).expect("delete first"),
        DeltaEffect::Cancelled
    );
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        Some(DeterminantOverlay::Present(second.as_slice())),
        "the later insert still owns the tuple"
    );
    assert_eq!(
        delta.delete(&view, R, &second).expect("delete second"),
        DeltaEffect::Cancelled
    );
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        None,
        "both pending inserts cancelled; committed state answers"
    );
    assert!(delta.is_empty());
}

#[test]
fn a_cancelled_insert_keeps_a_pending_deletes_absence() {

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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let new = fact(&schema, 7, 999); 

    delta.delete(&view, R, &old).expect("delete");
    assert_eq!(delta.determinant_scratch_clones, 1, "first record clones");

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

    delta
        .insert(&view, R, &fact(&schema, 8, 800))
        .expect("insert");
    assert_eq!(delta.determinant_scratch_clones, 2);
}

#[test]
fn a_second_insert_of_one_tuple_replaces_the_overlay_owner() {
    const KEY: KeyId = KeyId(0);
    let dir = TempDir::new("delta-overlay-one-insert");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let a = fact(&schema, 7, 1);
    let b = fact(&schema, 7, 2);
    delta.insert(&view, R, &a).expect("insert a");
    delta.insert(&view, R, &b).expect("insert b");
    assert_eq!(
        delta.determinant_overlay(KEY, &encode_u64(7)),
        Some(DeterminantOverlay::Present(b.as_slice())),
        "at most one Insert per tuple — the later fact answers"
    );
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
        delta.reserve(&view, R, ID, one()).expect("reserve");
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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
    }
    let view = env.read_txn().expect("txn");

    let mut delta = WriteDelta::new(&schema);
    for v in 10..20 {
        delta
            .insert(&view, A, &u64_fact(&schema, A, v))
            .expect("insert");
    }
    assert_eq!(delta.dirty_relations(), vec![]);

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

/// Cancellation is exact: a delete-then-reinsert of the same committed fact
/// nets to no entry, so its relation is NOT dirty — no false positives from
/// cancelled pairs (the delta's net-disposition invariant), and the untouched
/// relation's image survives as an append base.
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
        crate::storage::commit::commit(delta, &env)
            .expect("commit")
            .expect("admitted");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(
        delta.delete(&view, R, &f).expect("delete"),
        DeltaEffect::Recorded
    );
    assert_eq!(
        delta.dirty_relations(),
        vec![R],
        "a live pending delete dirties its relation"
    );
    assert_eq!(
        delta.insert(&view, R, &f).expect("insert"),
        DeltaEffect::Cancelled
    );
    assert_eq!(
        delta.dirty_relations(),
        vec![],
        "the reinsert cancelled the delete — nothing is removed from R"
    );
}
