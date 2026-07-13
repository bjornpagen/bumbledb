use super::*;

use crate::error::{Error, Violation};
use crate::schema::{FieldId, RelationId};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY, StatKind};
use crate::testutil::TempDir;

// ---------- 50-storage § Write path: full commit ----------

fn commit_facts(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) {
    apply_delta(env, schema, &[], facts).expect("commit");
}

#[test]
fn scalar_key_conflict_in_one_delta_aborts_with_the_statement_id() {
    let dir = TempDir::new("commit-scalar-in-delta");
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
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            &err,
            Error::CommitRejected { violations } if matches!(
                violations.as_slice(),
                [Violation::Functionality {
                    statement: KEYED_KEY,
                    incumbent: None,
                    fact,
                }] if **fact == a[..] || **fact == b[..]
            )
        ),
        "{err:?}"
    );
    assert_eq!(committed_data(&env), before);
}

#[test]
fn scalar_key_conflict_across_deltas_aborts_with_the_statement_id() {
    let dir = TempDir::new("commit-scalar-cross-delta");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(KEYED, keyed_fact(&schema, 1, 10))]);
    let before = committed_data(&env);

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let contender = keyed_fact(&schema, 1, 20);
    delta.insert(&view, KEYED, &contender).expect("insert");
    drop(view);
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            &err,
            Error::CommitRejected { violations } if matches!(
                violations.as_slice(),
                [Violation::Functionality {
                    statement: KEYED_KEY,
                    incumbent: None,
                    fact,
                }] if **fact == contender[..]
            )
        ),
        "{err:?}"
    );
    assert_eq!(committed_data(&env), before);
}

#[test]
fn delete_and_reinsert_of_a_committed_fact_commits_as_an_empty_delta() {
    // The net-disposition algebra: the re-insert cancels the pending
    // Delete, the delta is empty, and the commit is a no-op — the storage
    // tx id stays put (docs/architecture/50-storage.md).
    let dir = TempDir::new("commit-reestablish");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(KEYED, keyed_fact(&schema, 1, 10))]);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, KEYED, &keyed_fact(&schema, 1, 10))
        .expect("delete");
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 10))
        .expect("insert");
    drop(view);
    assert!(delta.is_empty());
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);
    assert_eq!(report.new_generation, 1);
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation"), 1);
}

#[test]
fn insert_and_delete_of_an_absent_fact_commits_as_an_empty_delta() {
    // The mirror case of the algebra: the delete cancels the pending
    // Insert of a fact base never held — empty delta, no tx id movement.
    let dir = TempDir::new("commit-cancel-absent");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(KEYED, keyed_fact(&schema, 1, 10))]);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 2, 20))
        .expect("insert");
    delta
        .delete(&view, KEYED, &keyed_fact(&schema, 2, 20))
        .expect("delete");
    drop(view);
    assert!(delta.is_empty());
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);
    assert_eq!(report.new_generation, 1);
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation"), 1);
}

#[test]
fn tx_id_advances_once_per_state_changing_commit_only() {
    let dir = TempDir::new("commit-tx-id");
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
    let dir = TempDir::new("commit-reopen-counters");
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
fn a_noop_commit_flushes_escaped_fresh_ids_and_nothing_else() {
    let dir = TempDir::new("commit-noop-fresh-flush");
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
    // The escaped fresh ids persisted: a later delta continues past them.
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
fn a_pure_noop_transaction_touches_neither_tx_id_nor_q_marks() {
    // The invariant pinned at `delta/insert.rs`'s advance site: the
    // committed `Q` high-water covers every committed fresh value, so
    // a transaction whose EVERY op is a no-op — even ones carrying
    // explicit fresh values — advances each mark exactly to its base
    // (clean) and never triggers the counters-only commit: the storage
    // tx id and the `Q` marks both come out byte-identical.
    let dir = TempDir::new("commit-pure-noop-clean-marks");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 5))]);
    let before = committed_data(&env);
    let q_key = key(|buf| keys::fresh_key(buf, TARGET, FieldId(0)));
    let q_before = {
        let rtxn = env.read_txn().expect("txn");
        env.data()
            .get(rtxn.raw(), &q_key)
            .expect("get")
            .map(<[u8]>::to_vec)
    };
    assert_eq!(
        q_before.as_deref(),
        Some(6u64.to_le_bytes().as_slice()),
        "the original commit advanced Q past the explicit value"
    );

    // Every op a no-op: re-inserting the committed fact (its explicit
    // fresh value 5 is already covered by Q = 6 — mark lands on the
    // base, clean) and deleting a fact base never held (records
    // nothing). The delta is empty and no mark is dirty: the commit
    // must write nothing at all.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert!(
        !delta
            .insert(&view, TARGET, &target_fact(&schema, 5))
            .expect("insert")
    );
    assert!(
        !delta
            .delete(&view, TARGET, &target_fact(&schema, 9))
            .expect("delete")
    );
    drop(view);
    let report = commit(delta, &env).expect("commit");
    assert!(!report.changed);

    let rtxn = env.read_txn().expect("txn");
    assert_eq!(
        rtxn.generation().expect("generation"),
        1,
        "the storage tx id did not advance"
    );
    assert_eq!(
        env.data()
            .get(rtxn.raw(), &q_key)
            .expect("get")
            .map(<[u8]>::to_vec),
        q_before,
        "the Q mark is byte-identical"
    );
    drop(rtxn);
    assert_eq!(committed_data(&env), before, "nothing else moved either");
}

#[test]
fn fresh_ids_allocated_in_an_aborted_txn_are_reissued() {
    let dir = TempDir::new("commit-fresh-abort");
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

// ---------- 50-storage § Write path, phase 5: the durability boundary ----------
//
// PRD 22: the one-observed bulk-load EINVAL (`fcntl(F_FULLFSYNC)` /
// commit-path `pwrite` surfacing a raw errno under I/O pressure). The
// boundary is typed (`Error::CommitSync`) and the transient class gets
// the bounded, observable retry — asserted here on the mechanism
// directly, since a real transient sync failure is not provokable from
// safe code (the stress harness in `bumbledb-bench` covers the live
// path).

/// The raw-errno commit failure as heed delivers it (`mdb_txn_commit`'s
/// EINVAL crossing `MdbError::Other` into `heed::Error::Io`).
fn einval() -> Error {
    Error::from_commit(heed::Error::Io(std::io::Error::from_raw_os_error(22)))
}

#[test]
fn from_commit_types_the_raw_errno_class_and_nothing_else() {
    // The one-observed class: a raw OS errno is the typed boundary fact.
    assert!(
        matches!(einval(), Error::CommitSync { retries: 0, error } if error.raw_os_error() == Some(22))
    );
    // LMDB-coded failures keep their established mapping.
    assert!(matches!(
        Error::from_commit(heed::Error::Mdb(heed::MdbError::MapFull)),
        Error::Lmdb(heed::Error::Mdb(heed::MdbError::MapFull))
    ));
    assert!(matches!(
        Error::from_commit(heed::Error::Mdb(heed::MdbError::ReadersFull)),
        Error::ReadersFull { .. }
    ));
}

#[test]
fn commit_bounded_absorbs_a_transient_sync_failure() {
    let mut attempts = 0u32;
    let out = super::super::write::commit_bounded(|| {
        attempts += 1;
        if attempts < 3 {
            return Err(einval());
        }
        Ok(attempts)
    });
    assert_eq!(out.expect("recovers"), 3, "two retries, then success");
}

#[test]
fn commit_bounded_escapes_typed_with_the_retry_count() {
    let mut attempts = 0u32;
    let err = super::super::write::commit_bounded::<()>(|| {
        attempts += 1;
        Err(einval())
    })
    .unwrap_err();
    assert_eq!(attempts, 4, "one try plus the bounded three retries");
    assert!(
        matches!(&err, Error::CommitSync { retries: 3, error } if error.raw_os_error() == Some(22)),
        "{err:?}"
    );
}

#[test]
fn commit_bounded_passes_every_other_error_through_on_the_first_throw() {
    let mut attempts = 0u32;
    let err = super::super::write::commit_bounded::<()>(|| {
        attempts += 1;
        Err(Error::Corruption(
            crate::error::CorruptionError::MetaMissing,
        ))
    })
    .unwrap_err();
    assert_eq!(attempts, 1, "non-sync errors are deterministic — no retry");
    assert!(matches!(err, Error::Corruption(_)), "{err:?}");
}

#[test]
fn pending_interns_flush_at_commit_and_advance_the_counter() {
    let dir = TempDir::new("commit-pending-interns");
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
        crate::storage::dict::resolve(&rtxn, id).expect("resolve"),
        b"holder-name"
    );
    drop(rtxn);
    // A later direct intern continues past the flushed counter.
    let mut wtxn = env.write_txn().expect("txn");
    let next = crate::storage::dict::intern_str(&mut wtxn, "other").expect("intern");
    assert_eq!(next, id + 1);
}
