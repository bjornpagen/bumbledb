use super::*;
use std::collections::BTreeSet;

use crate::encoding::{encode_interval_u64, encode_u64};
use crate::error::{CorruptionError, Error};
use crate::storage::commit::apply;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::testutil::TempDir;

#[test]
fn insert_lands_exactly_the_expected_key_set() {
    let dir = TempDir::new("commit-insert-keys");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t = target_fact(&schema, 5);
    let k = keyed_fact(&schema, 9, -3);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t).expect("insert");
        delta.insert(&view, KEYED, &k).expect("insert");
        drop(view);
        let applied = apply(delta, &env).expect("apply");

        let t_hash = crate::encoding::fact_hash(&t);
        let k_hash = crate::encoding::fact_hash(&k);
        let expected: BTreeSet<Vec<u8>> = [
            key(|b| keys::fact_key(b, TARGET, 0)),
            key(|b| keys::membership_key(b, TARGET, &t_hash)),
            key(|b| keys::guard_key(b, TARGET, TARGET_KEY, &encode_u64(5))),
            key(|b| keys::fact_key(b, KEYED, 0)),
            key(|b| keys::membership_key(b, KEYED, &k_hash)),
            key(|b| keys::guard_key(b, KEYED, KEYED_KEY, &encode_u64(9))),
        ]
        .into_iter()
        .collect();
        assert_eq!(all_data_keys(&applied.txn, &env), expected);

        // Bookkeeping: no deleted guards; the inserted Target guard is
        // recorded because a containment targets its key, and Keyed's
        // guard is not (no dependents).
        assert!(applied.deleted_guards.is_empty());
        assert_eq!(applied.inserted_guards.len(), 1);
        assert!(applied
            .inserted_guards
            .contains(&(TARGET_KEY, encode_u64(5).to_vec())));
        // Abort: drop the txn without committing.
    }
    assert!(committed_data(&env).is_empty());
}

#[test]
fn deleting_a_fact_with_a_scrubbed_f_row_is_corruption() {
    // Craft the M/F disagreement: commit a fact, raw-delete its F row
    // behind the codec's back, then delta-delete it. The write path
    // must raise the hard corruption error, never silently scrub the
    // M entry (docs/architecture/50-storage.md).
    let dir = TempDir::new("commit-desync");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t5 = target_fact(&schema, 5);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t5).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    // Scrub the F row (row id 0) directly.
    {
        let mut wtxn = env.write_txn().expect("wtxn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let f_len = keys::fact_key(&mut key, TARGET, 0);
        assert!(env
            .data()
            .delete(wtxn.raw_mut(), &key[..f_len])
            .expect("del"));
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, TARGET, &t5).expect("record delete");
    drop(view);
    let Err(err) = apply(delta, &env).map(|_| ()) else {
        panic!("apply must fail on a scrubbed F row");
    };
    assert!(matches!(
        err,
        Error::Corruption(CorruptionError::MembershipDesync {
            relation: TARGET,
            row_id: 0
        })
    ));
}

#[test]
fn deleting_a_fact_with_a_scrubbed_interval_guard_is_corruption() {
    // The same desync class on a 16-byte-field guard: scrub the Booking
    // key's U entry (scalar prefix ‖ whole interval) and delta-delete
    // the fact — the guard re-derivation must land on the missing key
    // and hard-error.
    let dir = TempDir::new("commit-desync-interval-guard");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let booked = booking_fact(&schema, 1, 10, 20, 0);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, BOOKING, &booked).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    {
        let mut guard = Vec::new();
        guard.extend_from_slice(&encode_u64(1));
        guard.extend_from_slice(&encode_interval_u64(10, 20));
        let mut wtxn = env.write_txn().expect("wtxn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let u_len = keys::guard_key(&mut key, BOOKING, BOOKING_KEY, &guard);
        assert!(env
            .data()
            .delete(wtxn.raw_mut(), &key[..u_len])
            .expect("del"));
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, BOOKING, &booked)
        .expect("record delete");
    drop(view);
    let Err(err) = apply(delta, &env).map(|_| ()) else {
        panic!("apply must fail on a scrubbed U guard");
    };
    assert!(matches!(
        err,
        Error::Corruption(CorruptionError::MembershipDesync {
            relation: BOOKING,
            row_id: 0
        })
    ));
}

#[test]
fn base_state_disagreeing_with_a_proved_disposition_is_corruption() {
    // The delta proves its net dispositions against committed state at op
    // time, and the single-writer mutex keeps that proof valid — so base
    // state contradicting an entry at apply time is unambiguously
    // corruption. Craft both directions by committing behind the delta's
    // back (exactly the discipline violation the probe names).
    let dir = TempDir::new("commit-disposition-desync");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t5 = target_fact(&schema, 5);

    // Insert direction: the delta proved t5 absent; land it underneath.
    let mut insert_delta = WriteDelta::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        insert_delta.insert(&view, TARGET, &t5).expect("insert");
    }
    {
        let view = env.read_txn().expect("txn");
        let mut sneak = WriteDelta::new(&schema);
        sneak.insert(&view, TARGET, &t5).expect("insert");
        drop(view);
        apply(sneak, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    let Err(err) = apply(insert_delta, &env).map(|_| ()) else {
        panic!("apply must fail on a base state the delta disproved");
    };
    assert!(matches!(
        err,
        Error::Corruption(CorruptionError::DispositionDesync { relation: TARGET })
    ));

    // Delete direction: the delta proved t5 present; scrub its M entry.
    let mut delete_delta = WriteDelta::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        delete_delta.delete(&view, TARGET, &t5).expect("delete");
    }
    {
        let mut wtxn = env.write_txn().expect("wtxn");
        let hash = crate::encoding::fact_hash(&t5);
        let mut key: KeyBuf = [0; MAX_KEY];
        let m_len = keys::membership_key(&mut key, TARGET, &hash);
        assert!(env
            .data()
            .delete(wtxn.raw_mut(), &key[..m_len])
            .expect("del"));
        wtxn.commit().expect("commit");
    }
    let Err(err) = apply(delete_delta, &env).map(|_| ()) else {
        panic!("apply must fail on a base state the delta disproved");
    };
    assert!(matches!(
        err,
        Error::Corruption(CorruptionError::DispositionDesync { relation: TARGET })
    ));
}

#[test]
fn delete_removes_exactly_its_entries() {
    let dir = TempDir::new("commit-delete-keys");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t5 = target_fact(&schema, 5);
    let t6 = target_fact(&schema, 6);
    let k = keyed_fact(&schema, 9, 4);
    // Commit a base state: two targets and one keyed fact.
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t5).expect("insert");
        delta.insert(&view, TARGET, &t6).expect("insert");
        delta.insert(&view, KEYED, &k).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    let before = committed_data(&env);

    // Delete the keyed fact: exactly its F/M/U entries disappear.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, KEYED, &k).expect("delete");
    drop(view);
    let applied = apply(delta, &env).expect("apply");

    let k_hash = crate::encoding::fact_hash(&k);
    let removed: BTreeSet<Vec<u8>> = [
        key(|b| keys::fact_key(b, KEYED, 0)),
        key(|b| keys::membership_key(b, KEYED, &k_hash)),
        key(|b| keys::guard_key(b, KEYED, KEYED_KEY, &encode_u64(9))),
    ]
    .into_iter()
    .collect();
    let expected: BTreeSet<Vec<u8>> = before
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| !removed.contains(k))
        .collect();
    assert_eq!(all_data_keys(&applied.txn, &env), expected);
    // Keyed's key has no containment dependents; nothing to record.
    assert!(applied.deleted_guards.is_empty());
}

#[test]
fn deleting_a_containment_targeted_key_records_its_guard() {
    let dir = TempDir::new("commit-deleted-guard");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t5 = target_fact(&schema, 5);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t5).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, TARGET, &t5).expect("delete");
    drop(view);
    let applied = apply(delta, &env).expect("apply");
    assert!(applied
        .deleted_guards
        .contains(&(TARGET_KEY, encode_u64(5).to_vec())));
}

#[test]
fn inserting_a_source_fact_writes_its_reverse_edge() {
    let dir = TempDir::new("commit-insert-reverse-edge");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t = target_fact(&schema, 5);
    let c = claim_fact(&schema, 5);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.insert(&view, TARGET, &t).expect("insert");
    delta.insert(&view, CLAIM, &c).expect("insert");
    drop(view);
    let applied = apply(delta, &env).expect("apply");

    // R | statement | key_bytes | source_rel | source_row: key_bytes is
    // the claim's projection in Target's guard order, the source row is
    // the claim's own row id (0, first fact of its relation).
    let r = key(|b| keys::reverse_key(b, CLAIM_TARGET, &encode_u64(5), CLAIM, 0));
    assert!(all_data_keys(&applied.txn, &env).contains(&r));
}

#[test]
fn deleting_a_source_fact_removes_the_same_reverse_edge() {
    let dir = TempDir::new("commit-delete-reverse-edge");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t = target_fact(&schema, 5);
    let c = claim_fact(&schema, 5);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t).expect("insert");
        delta.insert(&view, CLAIM, &c).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    let before = committed_data(&env);
    let r = key(|b| keys::reverse_key(b, CLAIM_TARGET, &encode_u64(5), CLAIM, 0));
    assert!(before.iter().any(|(k, _)| *k == r));

    // The delete re-derives the identical key bytes: exactly the claim's
    // F/M/R entries disappear (Claim has no key statements, so no U).
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, CLAIM, &c).expect("delete");
    drop(view);
    let applied = apply(delta, &env).expect("apply");
    let c_hash = crate::encoding::fact_hash(&c);
    let removed: BTreeSet<Vec<u8>> = [
        key(|b| keys::fact_key(b, CLAIM, 0)),
        key(|b| keys::membership_key(b, CLAIM, &c_hash)),
        r,
    ]
    .into_iter()
    .collect();
    let expected: BTreeSet<Vec<u8>> = before
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| !removed.contains(k))
        .collect();
    assert_eq!(all_data_keys(&applied.txn, &env), expected);
}

#[test]
fn delete_plus_insert_of_same_key_succeeds_in_either_user_order() {
    let dir = TempDir::new("commit-swap-order");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = keyed_fact(&schema, 1, 10);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, KEYED, &old).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    // The "wrong" user order: insert the replacement before deleting the
    // old fact. Commit-time semantics make order irrelevant.
    let new = keyed_fact(&schema, 1, 20);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.insert(&view, KEYED, &new).expect("insert");
    delta.delete(&view, KEYED, &old).expect("delete");
    drop(view);
    let applied = apply(delta, &env).expect("apply");
    // The guard key survives, now pointing at the new row.
    let u = key(|b| keys::guard_key(b, KEYED, KEYED_KEY, &encode_u64(1)));
    assert!(all_data_keys(&applied.txn, &env).contains(&u));
}

#[test]
fn rederived_guard_keys_match_independent_computation() {
    let dir = TempDir::new("commit-guard-derivation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let k = keyed_fact(&schema, 42, 7);
    let booked = booking_fact(&schema, 3, 100, 200, 1);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.insert(&view, KEYED, &k).expect("insert");
    delta.insert(&view, BOOKING, &booked).expect("insert");
    drop(view);
    let applied = apply(delta, &env).expect("apply");

    // The scalar guard is the canonical encoding of `x`; the pointwise
    // guard is `room ‖ during` with the interval's whole 16 bytes —
    // computed here independently of the applier's slicing.
    let keys_present = all_data_keys(&applied.txn, &env);
    assert!(keys_present.contains(&key(|b| keys::guard_key(
        b,
        KEYED,
        KEYED_KEY,
        &encode_u64(42)
    ))));
    let mut booking_guard = Vec::new();
    booking_guard.extend_from_slice(&encode_u64(3));
    booking_guard.extend_from_slice(&encode_interval_u64(100, 200));
    assert!(keys_present.contains(&key(|b| keys::guard_key(
        b,
        BOOKING,
        BOOKING_KEY,
        &booking_guard
    ))));
}
