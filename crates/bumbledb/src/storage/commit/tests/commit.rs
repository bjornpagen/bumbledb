use super::*;

use crate::error::{Conflict, Error, LmdbFailure, Violation};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY, StatKind};
use crate::testutil::TempDir;
use crate::testutil::expect_rejected;
use bumbledb_theory::schema::{FieldId, RelationId};

// ---------- 50-storage § Write path: full commit ----------

fn commit_facts(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) {
    apply_delta(env, schema, &[], facts)
        .expect("commit")
        .expect("admitted");
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
    let violations = expect_rejected(commit(delta, &env));
    assert!(
        matches!(
            violations.as_slice(),
            [Violation::Functionality {
                id: KEYED_KEY,
                fact,
                conflict: Conflict::Scalar,
                ..
            }] if **fact == a[..] || **fact == b[..]
        ),
        "{violations:?}"
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
    let violations = expect_rejected(commit(delta, &env));
    assert!(
        matches!(
            violations.as_slice(),
            [Violation::Functionality {
                id: KEYED_KEY,
                fact,
                conflict: Conflict::Scalar,
                ..
            }] if **fact == contender[..]
        ),
        "{violations:?}"
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
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(!report.changed());
    assert_eq!(report.generation().value(), 1);
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation").value(), 1);
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
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(!report.changed());
    assert_eq!(report.generation().value(), 1);
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation").value(), 1);
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
        assert_eq!(rtxn.generation().expect("generation").value(), 1);
    }

    // All-no-op delta: re-inserting an existing fact records nothing.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(
        delta.insert(&view, TARGET, &f).expect("insert"),
        crate::storage::delta::DeltaEffect::NoOp
    );
    drop(view);
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(!report.changed());
    assert_eq!(report.generation().value(), 1);
    {
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(rtxn.generation().expect("generation").value(), 1);
    }

    // A second state-changing commit bumps exactly once.
    commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 6))]);
    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation").value(), 2);
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
        commit(delta, &env).expect("commit").expect("admitted");
    }

    // Reopen: the flushed counters are the only test that can catch a
    // never-persisted high-water.
    let env = Environment::open(dir.path(), &schema).expect("open");
    let rtxn = env.read_txn().expect("txn");
    let count_key = keys::stat_key(TARGET, StatKind::RowCount);
    let count = u64::from_le_bytes(
        env.data()
            .get(rtxn.raw(), &count_key)
            .expect("get")
            .expect("row count present")
            .try_into()
            .expect("u64"),
    );
    let mut prefix_buf: KeyBuf = [0; MAX_KEY];
    let prefix = keys::fact_prefix(&mut prefix_buf, TARGET);
    let scanned = env
        .data()
        .prefix_iter(rtxn.raw(), prefix)
        .expect("iter")
        .count() as u64;
    assert_eq!(count, scanned);
    assert_eq!(count, 3); // 3 inserted + 1 inserted - 1 deleted

    // Target is fresh-keyed, so no S high-water exists (the one id
    // allocator, R16): the mint is Q, ratcheted past the explicit fresh
    // values 1..=4 — the stored next value is 5.
    let hw_key = keys::stat_key(TARGET, StatKind::RowIdHighWater);
    assert_eq!(
        env.data().get(rtxn.raw(), &hw_key).expect("get"),
        None,
        "a fresh-keyed relation owns no S high-water"
    );
    let q_key = keys::fresh_key(TARGET, FieldId(0));
    let q_next = u64::from_le_bytes(
        env.data()
            .get(rtxn.raw(), &q_key)
            .expect("get")
            .expect("Q next present")
            .try_into()
            .expect("u64"),
    );
    assert_eq!(q_next, 5);
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
    assert_eq!(
        delta
            .reserve(
                &view,
                TARGET,
                FieldId(0),
                std::num::NonZeroU64::new(1).unwrap()
            )
            .expect("reserve"),
        6
    );
    assert_eq!(
        delta
            .reserve(
                &view,
                TARGET,
                FieldId(0),
                std::num::NonZeroU64::new(1).unwrap()
            )
            .expect("reserve"),
        7
    );
    delta.intern_str(&view, "ghost").expect("intern");
    drop(view);
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(!report.changed());
    assert_eq!(report.generation().value(), 1);

    let rtxn = env.read_txn().expect("txn");
    assert_eq!(rtxn.generation().expect("generation").value(), 1, "no bump");
    // The escaped fresh ids persisted: a later delta continues past them.
    let mut fresh = WriteDelta::new(&schema);
    assert_eq!(
        fresh
            .reserve(
                &rtxn,
                TARGET,
                FieldId(0),
                std::num::NonZeroU64::new(1).unwrap()
            )
            .expect("reserve"),
        8
    );
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
    let q_key = keys::fresh_key(TARGET, FieldId(0)).to_vec();
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
    assert_eq!(
        delta
            .insert(&view, TARGET, &target_fact(&schema, 5))
            .expect("insert"),
        crate::storage::delta::DeltaEffect::NoOp
    );
    assert_eq!(
        delta
            .delete(&view, TARGET, &target_fact(&schema, 9))
            .expect("delete"),
        crate::storage::delta::DeltaEffect::NoOp
    );
    drop(view);
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(!report.changed());

    let rtxn = env.read_txn().expect("txn");
    assert_eq!(
        rtxn.generation().expect("generation").value(),
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
fn fresh_ids_reserved_in_a_rejected_txn_are_burned() {
    // The never-reissue law is unconditional: an id `reserve` handed the
    // host is burned even when the commit is REJECTED — `reserve` returns it
    // before the commit's fate is known, and a rejection carries the
    // offending facts back as data, so re-issue would break observability
    // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`;
    // `commit`'s escaped-flush on the reject exit). The other abort paths
    // — a failing or PANICKING `Db::write` closure — burn through
    // `Db::write`'s `EscapedIdBurn` drop guard.
    let dir = TempDir::new("commit-fresh-reject");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    // Mint a fresh TARGET id and use it...
    let id = delta
        .reserve(
            &view,
            TARGET,
            FieldId(0),
            std::num::NonZeroU64::new(1).unwrap(),
        )
        .expect("reserve");
    assert_eq!(id, 0);
    delta
        .insert(&view, TARGET, &target_fact(&schema, id))
        .expect("insert");
    // ...while staging a KEYED functionality conflict so the commit aborts.
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 10))
        .expect("insert");
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 20))
        .expect("insert");
    drop(view);
    let _violations = expect_rejected(commit(delta, &env));

    // The rejected commit rolled back every fact — but it burned the id it
    // handed out: the next transaction mints past 0, never re-issuing it.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    assert_eq!(
        delta
            .reserve(
                &view,
                TARGET,
                FieldId(0),
                std::num::NonZeroU64::new(1).unwrap()
            )
            .expect("reserve"),
        1,
        "0 was handed to the host and is gone forever, the abort notwithstanding"
    );
}

// ---------- 50-storage § Write path, phase 5: the durability boundary ----------
//
// PRD 22: the one-observed write-path EINVAL (`fcntl(F_FULLFSYNC)` /
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
        Error::Lmdb(LmdbFailure::Mdb(heed::MdbError::MapFull))
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
    commit(delta, &env).expect("commit").expect("admitted");

    let rtxn = env.read_txn().expect("txn");
    assert_eq!(
        crate::storage::dict::lookup_str(&rtxn, "holder-name").expect("lookup"),
        Some(id)
    );
    assert_eq!(
        crate::storage::dict::resolve(&rtxn, id).expect("resolve"),
        b"holder-name"
    );
    // A later transaction's provisional mint continues past the flushed
    // counter (the production writer — no direct-write path exists).
    let mut later = WriteDelta::new(&schema);
    let next = later.intern_str(&rtxn, "other").expect("intern");
    assert_eq!(next.raw(), id.raw() + 1);
}

/// The never-reissue law on `commit()`'s own pre-plan exits
/// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`): by the
/// time the delta reaches `commit()`, the write region's drop guard has
/// disarmed — commit owns the flush on EVERY termination, including the
/// fallible work BEFORE `commit_bounded` (the plan block's snapshot
/// read, the empty path's generation read). The injected fault is a
/// full reader table (`MDB_NOTLS` binds slots to transaction objects,
/// so one thread exhausts it alone): `env.read_txn()` fails typed while
/// the burn's own WRITE transaction still succeeds. Both pre-plan exits
/// are pinned: the plan block (non-empty delta) and the no-op path's
/// generation read (reserve-only delta).
#[test]
fn a_pre_plan_infra_failure_still_burns_the_escaped_fresh_ids() {
    use crate::storage::env::MAX_READERS;
    let dir = TempDir::new("commit-preplan-burn");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");

    let mint = |env: &Environment, insert: bool| {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let id = delta
            .reserve(
                &view,
                TARGET,
                FieldId(0),
                std::num::NonZeroU64::new(1).unwrap(),
            )
            .expect("reserve");
        if insert {
            delta
                .insert(&view, TARGET, &target_fact(&schema, id))
                .expect("insert");
        }
        drop(view);
        (id, delta)
    };

    // The plan-block exit: a non-empty delta whose escaped id must
    // survive the failed `env.read_txn()` before `plan_commit`.
    let (id, delta) = mint(&env, true);
    assert_eq!(id, 0, "the mint's first issue");
    let held: Vec<_> = (0..MAX_READERS)
        .map(|_| env.read_txn().expect("slot within the table"))
        .collect();
    let err = commit(delta, &env).unwrap_err();
    assert!(matches!(err, Error::ReadersFull { .. }), "{err:?}");
    drop(held);

    // The no-op path's generation-read exit: a reserve-only (empty)
    // delta, same injected fault. Its mint continuing past `id` also
    // proves the first abort burned.
    let (id, delta) = mint(&env, false);
    assert_eq!(id, 1, "the plan-block abort burned its escaped id");
    assert!(delta.is_empty(), "reserve-only deltas take the no-op path");
    let held: Vec<_> = (0..MAX_READERS)
        .map(|_| env.read_txn().expect("slot within the table"))
        .collect();
    let err = commit(delta, &env).unwrap_err();
    assert!(matches!(err, Error::ReadersFull { .. }), "{err:?}");
    drop(held);

    // Never re-issued: both aborted transactions' ids are gone.
    let (id, _delta) = mint(&env, false);
    assert_eq!(id, 2, "the no-op-path abort burned its escaped id");
}

#[test]
fn a_failed_escaped_flush_still_never_reissues_in_process() {
    let dir = TempDir::new("commit-flush-fail-in-process");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let id = delta
        .reserve(
            &view,
            TARGET,
            FieldId(0),
            std::num::NonZeroU64::new(1).unwrap(),
        )
        .expect("reserve");
    assert_eq!(id, 0);
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 10))
        .expect("insert");
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 20))
        .expect("insert");
    drop(view);
    env.fail_next_fresh_flushes(1);
    let _violations = expect_rejected(commit(delta, &env));
    let q_key = keys::fresh_key(TARGET, FieldId(0)).to_vec();
    {
        let rtxn = env.read_txn().expect("txn");
        assert!(
            env.data().get(rtxn.raw(), &q_key).expect("get").is_none(),
            "the failed burn left disk Q untouched"
        );
    }
    let view = env.read_txn().expect("txn");
    let mut next = WriteDelta::new(&schema);
    assert_eq!(
        next.reserve(
            &view,
            TARGET,
            FieldId(0),
            std::num::NonZeroU64::new(1).unwrap()
        )
        .expect("reserve"),
        1,
        "in-process high-water forbids reissuing 0"
    );
}

#[test]
fn s_row_count_overflow_is_labeled_overflow() {
    let dir = TempDir::new("commit-s-overflow");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let mut wtxn = env.write_txn().expect("txn");
        let key = keys::stat_key(KEYED, StatKind::RowCount);
        env.data()
            .put(wtxn.raw_mut(), &key, u64::MAX.to_le_bytes().as_slice())
            .expect("put S");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 10))
        .expect("insert");
    drop(view);
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "S row count overflow"
            ))
        ),
        "{err:?}"
    );
}

#[test]
fn s_row_count_underflow_keeps_the_underflow_label() {
    let dir = TempDir::new("commit-s-underflow");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let fact = keyed_fact(&schema, 1, 10);
    apply_delta(&env, &schema, &[], &[(KEYED, fact.clone())])
        .expect("base")
        .expect("admitted");
    {
        let mut wtxn = env.write_txn().expect("txn");
        let key = keys::stat_key(KEYED, StatKind::RowCount);
        env.data()
            .put(wtxn.raw_mut(), &key, 0u64.to_le_bytes().as_slice())
            .expect("zero S");
        wtxn.commit().expect("commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, KEYED, &fact).expect("delete");
    drop(view);
    let err = commit(delta, &env).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "S row count underflow"
            ))
        ),
        "{err:?}"
    );
}
