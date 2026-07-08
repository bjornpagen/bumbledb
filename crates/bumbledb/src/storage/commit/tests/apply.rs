use super::*;
use std::collections::BTreeSet;

use crate::encoding::encode_u64;
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
    let s = source_fact(&schema, 9, 5);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t).expect("insert");
        delta.insert(&view, SOURCE, &s).expect("insert");
        drop(view);
        let applied = apply(delta, &env).expect("apply");

        let t_hash = crate::encoding::fact_hash(&t);
        let s_hash = crate::encoding::fact_hash(&s);
        let expected: BTreeSet<Vec<u8>> = [
            key(|b| keys::fact_key(b, TARGET, 0)),
            key(|b| keys::membership_key(b, TARGET, &t_hash)),
            key(|b| keys::unique_key(b, TARGET, C0, &encode_u64(5))),
            key(|b| keys::fact_key(b, SOURCE, 0)),
            key(|b| keys::membership_key(b, SOURCE, &s_hash)),
            key(|b| keys::unique_key(b, SOURCE, C0, &encode_u64(9))),
            key(|b| keys::restrict_key(b, TARGET, C0, &encode_u64(5), SOURCE, 0)),
        ]
        .into_iter()
        .collect();
        assert_eq!(all_data_keys(&applied.txn, &env), expected);

        // Bookkeeping: one forward probe for the FK, no deleted guards,
        // the inserted target guard recorded for the FK-targeted
        // constraint.
        assert_eq!(applied.fk_probes.len(), 1);
        let (target_rel, target_cid, guard) =
            applied.fk_probes.keys().next().expect("one probe");
        assert_eq!((*target_rel, *target_cid), (TARGET, C0));
        assert_eq!(guard.as_slice(), encode_u64(5));
        assert!(applied.deleted_guards.is_empty());
        assert!(applied
            .inserted_guards
            .contains(&(TARGET, C0, encode_u64(5).to_vec())));
        assert!(applied.changed);
        // Abort: drop the txn without committing.
    }
    assert!(committed_data(&env).is_empty());
}

#[test]
fn deleting_a_fact_with_a_scrubbed_f_row_is_corruption() {
    // Craft the M/F disagreement: commit a fact, raw-delete its F row
    // behind the codec's back, then delta-delete it. The write path
    // must raise the hard corruption error, never silently scrub the
    // M entry (docs/architecture/40-storage.md).
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
fn delete_removes_exactly_its_entries() {
    let dir = TempDir::new("commit-delete-keys");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let t5 = target_fact(&schema, 5);
    let t6 = target_fact(&schema, 6);
    let s = source_fact(&schema, 9, 5);
    // Commit a base state: two targets, one source referencing t5.
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, TARGET, &t5).expect("insert");
        delta.insert(&view, TARGET, &t6).expect("insert");
        delta.insert(&view, SOURCE, &s).expect("insert");
        drop(view);
        apply(delta, &env)
            .expect("apply")
            .txn
            .commit()
            .expect("commit");
    }
    let before = committed_data(&env);

    // Delete the source fact: exactly its F/M/U/R entries disappear.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, SOURCE, &s).expect("delete");
    drop(view);
    let applied = apply(delta, &env).expect("apply");

    let s_hash = crate::encoding::fact_hash(&s);
    let removed: BTreeSet<Vec<u8>> = [
        key(|b| keys::fact_key(b, SOURCE, 0)),
        key(|b| keys::membership_key(b, SOURCE, &s_hash)),
        key(|b| keys::unique_key(b, SOURCE, C0, &encode_u64(9))),
        key(|b| keys::restrict_key(b, TARGET, C0, &encode_u64(5), SOURCE, 0)),
    ]
    .into_iter()
    .collect();
    let expected: BTreeSet<Vec<u8>> = before
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| !removed.contains(k))
        .collect();
    assert_eq!(all_data_keys(&applied.txn, &env), expected);
    // Source's own serial unique is not FK-targeted; nothing to scan.
    assert!(applied.deleted_guards.is_empty());
}

#[test]
fn deleting_an_fk_targeted_fact_records_its_guard() {
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
        .contains(&(TARGET, C0, encode_u64(5).to_vec())));
}

#[test]
fn delete_plus_insert_of_same_unique_key_succeeds_in_either_user_order() {
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
    let u = key(|b| keys::unique_key(b, KEYED, C0, &encode_u64(1)));
    assert!(all_data_keys(&applied.txn, &env).contains(&u));
}

#[test]
fn two_facts_claiming_one_unique_key_is_a_violation_and_base_stays_intact() {
    let dir = TempDir::new("commit-unique-violation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let before = committed_data(&env);

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let a = keyed_fact(&schema, 1, 10);
    let b = keyed_fact(&schema, 1, 20);
    delta.insert(&view, KEYED, &a).expect("insert");
    delta.insert(&view, KEYED, &b).expect("insert");
    drop(view);
    let Err(err) = apply(delta, &env) else {
        panic!("expected a unique violation");
    };
    assert!(
        matches!(
            err,
            Error::UniqueViolation {
                relation: KEYED,
                constraint: C0,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(committed_data(&env), before);
}

#[test]
fn rederived_guard_keys_match_independent_computation() {
    let dir = TempDir::new("commit-guard-derivation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let s = source_fact(&schema, 42, 7);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.insert(&view, SOURCE, &s).expect("insert");
    drop(view);
    let applied = apply(delta, &env).expect("apply");

    // The serial auto-unique guard is the canonical encoding of `id`,
    // and the FK guard is the canonical encoding of `t` — computed here
    // independently of `derive_guard`.
    let keys_present = all_data_keys(&applied.txn, &env);
    assert!(keys_present.contains(&key(|b| keys::unique_key(b, SOURCE, C0, &encode_u64(42)))));
    assert!(keys_present.contains(&key(|b| keys::restrict_key(
        b,
        TARGET,
        C0,
        &encode_u64(7),
        SOURCE,
        0
    ))));
    assert_eq!(
        applied.fk_probes.keys().next().expect("one probe").2,
        encode_u64(7).to_vec()
    );
}
