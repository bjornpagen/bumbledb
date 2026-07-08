use super::*;

use crate::encoding::encode_u64;
use crate::error::{CorruptionError, Error, FkViolation};
use crate::schema::{ConstraintId, FieldId, RelationId};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};
use crate::testutil::TempDir;

// ---------- the 40-storage doc: full commit ----------

fn commit_facts(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (rel, fact) in facts {
        delta.insert(&view, *rel, fact).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

#[test]
fn insert_referencing_same_delta_target_commits() {
    let dir = TempDir::new("commit8-order-irrelevance");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    // Referrer inserted before its target: order is irrelevant.
    delta
        .insert(&view, SOURCE, &source_fact(&schema, 1, 5))
        .expect("insert");
    delta
        .insert(&view, TARGET, &target_fact(&schema, 5))
        .expect("insert");
    drop(view);
    let report = commit(delta, &env).expect("commit succeeds");
    assert!(report.changed);
    assert_eq!(report.new_generation, 1);
}

#[test]
fn insert_referencing_missing_target_aborts_with_fk_violation() {
    let dir = TempDir::new("commit8-missing-target");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let before = committed_data(&env);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let orphan = source_fact(&schema, 1, 99);
    delta.insert(&view, SOURCE, &orphan).expect("insert");
    drop(view);
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            &err,
            Error::ForeignKeyViolation {
                relation: SOURCE,
                constraint: ConstraintId(1),
                violation: FkViolation::MissingTarget { fact_bytes },
            } if **fact_bytes == orphan[..]
        ),
        "{err:?}"
    );
    assert_eq!(committed_data(&env), before);
}

#[test]
fn deleting_a_referenced_target_alone_is_a_restrict_violation() {
    let dir = TempDir::new("commit8-restrict");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(
        &env,
        &schema,
        &[
            (TARGET, target_fact(&schema, 5)),
            (SOURCE, source_fact(&schema, 1, 5)),
        ],
    );
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, TARGET, &target_fact(&schema, 5))
        .expect("delete");
    drop(view);
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            err,
            Error::ForeignKeyViolation {
                relation: TARGET,
                constraint: C0,
                violation: FkViolation::RemainingReference {
                    source_relation: SOURCE,
                    ..
                },
            }
        ),
        "{err:?}"
    );
}

#[test]
fn deleting_target_and_all_referrers_in_one_delta_commits() {
    let dir = TempDir::new("commit8-cascade-by-hand");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(
        &env,
        &schema,
        &[
            (TARGET, target_fact(&schema, 5)),
            (SOURCE, source_fact(&schema, 1, 5)),
        ],
    );
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, TARGET, &target_fact(&schema, 5))
        .expect("delete");
    delta
        .delete(&view, SOURCE, &source_fact(&schema, 1, 5))
        .expect("delete");
    drop(view);
    commit(delta, &env).expect("deleting target and referrers together passes");
}

#[test]
fn delete_and_reinsert_of_referenced_unique_key_commits() {
    let dir = TempDir::new("commit8-reestablish");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(
        &env,
        &schema,
        &[
            (TARGET, target_fact(&schema, 5)),
            (SOURCE, source_fact(&schema, 1, 5)),
        ],
    );
    // Replace the target fact wholesale, re-supplying its unique key —
    // Restrict sees the re-established key and passes.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, TARGET, &target_fact(&schema, 5))
        .expect("delete");
    delta
        .insert(&view, TARGET, &target_fact(&schema, 5))
        .expect("insert");
    drop(view);
    // Net effect: delete then insert of the same fact is a no-op delta
    // (last disposition wins → Insert of a base-present fact).
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);
}

#[test]
fn tx_id_advances_once_per_state_changing_commit_only() {
    let dir = TempDir::new("commit8-tx-id");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let f = target_fact(&schema, 5);
    commit_facts(&env, &schema, &[(TARGET, f.clone())]);
    {
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(rtxn.generation().expect("generation"), 1);
    }

    // All-no-op delta: re-inserting an existing fact records nothing.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert!(!delta.insert(&view, TARGET, &f).expect("insert"));
    drop(view);
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);
    assert_eq!(report.new_generation, 1);
    {
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(rtxn.generation().expect("generation"), 1);
    }

    // A second state-changing commit bumps exactly once.
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 6))]);
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation"), 2);
}

#[test]
fn counters_after_reopen_match_a_recount_of_f_entries() {
    let dir = TempDir::new("commit8-reopen-counters");
    let schema = schema();
    {
        let env = Environment::create(dir.path(), &schema).expect("create");
        commit_facts(
            &env,
            &schema,
            &[
                (TARGET, target_fact(&schema, 1)),
                (TARGET, target_fact(&schema, 2)),
                (TARGET, target_fact(&schema, 3)),
            ],
        );
        // Mixed insert/delete commit.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .delete(&view, TARGET, &target_fact(&schema, 2))
            .expect("delete");
        delta
            .insert(&view, TARGET, &target_fact(&schema, 4))
            .expect("insert");
        drop(view);
        commit(delta, &env).expect("commit");
    }

    // Reopen: the flushed counters are the only test that can catch a
    // never-persisted high-water.
    let env = Environment::open(dir.path(), &schema).expect("open");
    let rtxn = env.read_txn().expect("txn");
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::stat_key(&mut key, TARGET, StatKind::RowCount);
    let count = u64::from_le_bytes(
        env.data()
            .get(rtxn.raw(), &key[..len])
            .expect("get")
            .expect("row count present")
            .try_into()
            .expect("u64"),
    );
    let prefix_len = keys::fact_prefix(&mut key, TARGET);
    let scanned = env
        .data()
        .prefix_iter(rtxn.raw(), &key[..prefix_len])
        .expect("iter")
        .count() as u64;
    assert_eq!(count, scanned);
    assert_eq!(count, 3); // 3 inserted + 1 inserted - 1 deleted

    // The high-water also survived: row ids 0..=3 were assigned, so the
    // stored next id is 4.
    let hw_len = keys::stat_key(&mut key, TARGET, StatKind::RowIdHighWater);
    let high_water = u64::from_le_bytes(
        env.data()
            .get(rtxn.raw(), &key[..hw_len])
            .expect("get")
            .expect("high water present")
            .try_into()
            .expect("u64"),
    );
    assert_eq!(high_water, 4);
}

#[test]
fn a_noop_commit_flushes_escaped_serials_and_nothing_else() {
    let dir = TempDir::new("commit-noop-serial-flush");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 5))]);

    // An empty delta that allocated (ids the closure could have
    // returned) and interned (ids that never escape): the commit
    // persists exactly the dirty Q marks — no generation bump, no
    // intern flush, no dict counter.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 6);
    assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 7);
    delta.intern_str(&view, "ghost").expect("intern");
    drop(view);
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);
    assert_eq!(report.new_generation, 1);

    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation"), 1, "no bump");
    // The escaped serials persisted: a fresh delta continues past them.
    let mut fresh = WriteDelta::new(&schema);
    assert_eq!(fresh.alloc(&rtxn, TARGET, FieldId(0)).expect("alloc"), 8);
    // The pending intern was dropped, counter untouched.
    assert_eq!(
        crate::storage::dict::lookup_str(&rtxn, "ghost").expect("lookup"),
        None
    );
    assert_eq!(rtxn.dict_next_id().expect("dict next"), 0);
}

#[test]
fn a_noop_commit_with_clean_serial_marks_touches_nothing() {
    let dir = TempDir::new("commit-noop-clean-marks");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 5))]);
    let before = committed_data(&env);

    // Re-inserting the existing fact reads the serial base (mark 6,
    // base 6 — clean) and records no disposition: the commit must
    // write nothing at all.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert!(!delta
        .insert(&view, TARGET, &target_fact(&schema, 5))
        .expect("insert"));
    drop(view);
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);
    assert_eq!(committed_data(&env), before);
}

/// PRD 06 (docs/hardening): a bare-prefix R key (the audit's shape —
/// no 12-byte source tail) is typed Corruption from the delete's
/// commit, and the store stays reopenable — nothing torn.
#[test]
fn a_malformed_r_key_is_typed_corruption_at_delete() {
    let dir = TempDir::new("commit-corrupt-r-key");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 5))]);

    // Plant an R key that is exactly the restrict prefix for the
    // target's guard — a well-formed key carries 12 more bytes.
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let p_len = keys::restrict_prefix(&mut key, TARGET, C0, &encode_u64(5));
        env.data()
            .put(wtxn.raw_mut(), &key[..p_len], [].as_slice())
            .expect("plant");
        wtxn.commit().expect("commit");
    }

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, TARGET, &target_fact(&schema, 5))
        .expect("record delete");
    drop(view);
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("R key length"))
        ),
        "{err:?}"
    );
    // The commit aborted cleanly: the fact is still live and the
    // store keeps working.
    let view = env.read_txn().expect("txn");
    let delta = WriteDelta::new(&schema);
    assert_eq!(
        delta.disposition(TARGET, &target_fact(&schema, 5)),
        None,
        "nothing recorded"
    );
    drop(view);
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 6))]);
}

#[test]
fn serials_allocated_in_an_aborted_txn_are_reissued() {
    let dir = TempDir::new("commit8-serial-abort");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 0);
        assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 1);
        // Abort: drop the delta without committing.
    }
    // The committed sequence is untouched: the next transaction
    // re-issues the same values.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let id = delta.alloc(&view, TARGET, FieldId(0)).expect("alloc");
    assert_eq!(id, 0);
    delta
        .insert(&view, TARGET, &target_fact(&schema, id))
        .expect("insert");
    drop(view);
    commit(delta, &env).expect("commit");

    // After a *committed* allocation, the sequence advances past it.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 1);
}

#[test]
fn pending_interns_flush_at_commit_and_advance_the_counter() {
    let dir = TempDir::new("commit8-pending-interns");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let id = delta.intern_str(&view, "holder-name").expect("intern");
    assert_eq!(delta.intern_str(&view, "holder-name").expect("intern"), id);
    // The delta must record a state change for the commit to flush; a
    // fact carrying the fresh id plays that role.
    delta
        .insert(&view, TARGET, &target_fact(&schema, 7))
        .expect("insert");
    drop(view);
    commit(delta, &env).expect("commit");

    let rtxn = env.read_txn().expect("txn");
    assert_eq!(
        crate::storage::dict::lookup_str(&rtxn, "holder-name").expect("lookup"),
        Some(id)
    );
    assert_eq!(
        crate::storage::dict::resolve(&rtxn, id, crate::storage::dict::TAG_STRING)
            .expect("resolve"),
        b"holder-name"
    );
    drop(rtxn);
    // A later direct intern continues past the flushed counter.
    let mut wtxn = env.write_txn().expect("txn");
    let next = crate::storage::dict::intern_str(&mut wtxn, "other").expect("intern");
    assert_eq!(next, id + 1);
}
