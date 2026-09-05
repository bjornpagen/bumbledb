//! Authored sweeper tests over the successor store (F1: written, executed
//! only in F3). Gate mapping: ENG-003/ENG-006 closure evidence (the sweep
//! itself reads one coherent snapshot and knows no dictionary), G06
//! verify-family children, E-ADMIT's offline re-judgment half (a store
//! holding an unlawful state is convicted by the same production judge).
//!
//! Corruption fixtures write impossible bytes through the store's own
//! crate-private handles — the exact desync each finding names, never a
//! second key-derivation implementation.

use std::time::Duration;

use bumbledb_theory::schema::{
    FieldDescriptor, RelationId, SchemaDescriptor, StatementDescriptor, ValueType,
};

use crate::storage::store::verify::{VerifyCorruption, VerifyFinding};
use crate::storage::store::{
    CandidateJudge, CandidateState, Judgment as StoreJudgment, StoreResult, UnindexedRows,
};
use crate::testutil::TempDir;
use crate::work::{ExecutionPolicy, WorkContext};
use crate::{Db, StoreVerdict, Theory, Value};

const ENTRY: RelationId = RelationId(0);

#[derive(Clone, Copy)]
struct Ledger;

impl Theory for Ledger {
    fn descriptor(self) -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                name: "entry".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                    },
                ],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: ENTRY,
                projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
            }],
        }
    }
}

fn row(name: &str, amount: i64) -> Vec<Value> {
    vec![Value::String(name.into()), Value::I64(amount)]
}

fn create(dir: &TempDir) -> Db<Ledger> {
    let work = ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 30,
        work_units: 1 << 30,
        timeout: Duration::from_secs(3600),
    }
    .start()
    .expect("work");
    Db::create(dir.path(), Ledger, work)
        .expect("create")
        .expect("empty theory admits")
}

#[test]
fn a_lawful_store_sweeps_coherent_after_mixed_commits() {
    let dir = TempDir::new("verify-coherent");
    let db = create(&dir);
    db.write(|tx| {
        tx.insert_dyn(ENTRY, [row("a", 1), row("b", 2)])?;
        Ok(())
    })
    .expect("write")
    .unwrap();
    db.write(|tx| {
        tx.delete_dyn(ENTRY, [row("a", 1)])?;
        tx.insert_dyn(ENTRY, [row("c", 3)])?;
        Ok(())
    })
    .expect("write")
    .unwrap();
    let report = db.verify_store().expect("sweep");
    assert_eq!(report.verdict, StoreVerdict::Coherent, "{report:?}");
    assert!(report.findings().is_empty());
}

#[test]
fn a_dangling_membership_entry_is_a_typed_finding() {
    let dir = TempDir::new("verify-dangling-membership");
    let db = create(&dir);
    db.write(|tx| tx.insert_dyn(ENTRY, [row("a", 1)]).map(|_| ()))
        .expect("write")
        .unwrap();
    // Forge a membership entry pointing at a row id that does not exist.
    let store = db.integration_store();
    {
        let inner = &store.inner;
        let mut wtxn = inner.env.write_txn().expect("fixture txn");
        let fake = crate::storage::store::keys::membership_key(
            ENTRY,
            &[0xAB; crate::storage::store::FP_LEN],
            crate::storage::store::RowId(9_999),
        );
        inner
            .data
            .put(&mut wtxn, fake.as_slice(), &[])
            .expect("fixture put");
        wtxn.commit().expect("fixture commit");
    }
    let report = db.verify_store().expect("sweep");
    assert!(
        report.findings().iter().any(|finding| matches!(
            finding,
            VerifyFinding::Corruption(VerifyCorruption::DanglingMembership { relation, row })
                if *relation == ENTRY && row.0 == 9_999
        )),
        "{report:?}"
    );
}

#[test]
fn a_row_without_membership_and_a_wrong_bucket_are_distinct_findings() {
    let dir = TempDir::new("verify-membership-shape");
    let db = create(&dir);
    db.write(|tx| tx.insert_dyn(ENTRY, [row("a", 1)]).map(|_| ()))
        .expect("write")
        .unwrap();
    let store = db.integration_store();
    // Move the real membership entry into a foreign bucket: the row loses
    // its exact-fingerprint backing (MissingMembership) and the moved entry
    // convicts separately (ForeignMembership).
    {
        let inner = &store.inner;
        let (row_id, fp) = {
            let rtxn = inner.env.read_txn().expect("fixture read");
            let mut iter = inner
                .data
                .prefix_iter(
                    &rtxn,
                    [crate::storage::store::keys::TAG_MEMBERSHIP].as_slice(),
                )
                .expect("fixture iter");
            let (key, _) = iter.next().expect("one membership entry").expect("entry");
            let mut fp = [0u8; crate::storage::store::FP_LEN];
            fp.copy_from_slice(&key[5..5 + crate::storage::store::FP_LEN]);
            (
                crate::storage::store::keys::row_id_from_suffix(
                    key,
                    crate::storage::store::keys::MEMBERSHIP_KEY_LEN,
                )
                .expect("row id"),
                fp,
            )
        };
        let mut wtxn = inner.env.write_txn().expect("fixture txn");
        let real = crate::storage::store::keys::membership_key(ENTRY, &fp, row_id);
        inner
            .data
            .delete(&mut wtxn, real.as_slice())
            .expect("fixture delete");
        let mut wrong = fp;
        wrong[0] ^= 0xFF;
        let forged = crate::storage::store::keys::membership_key(ENTRY, &wrong, row_id);
        inner
            .data
            .put(&mut wtxn, forged.as_slice(), &[])
            .expect("fixture put");
        wtxn.commit().expect("fixture commit");
    }
    let report = db.verify_store().expect("sweep");
    let findings = report.findings();
    assert!(
        findings.iter().any(|finding| matches!(
            finding,
            VerifyFinding::Corruption(VerifyCorruption::MissingMembership { relation, .. })
                if *relation == ENTRY
        )),
        "{report:?}"
    );
    assert!(
        findings.iter().any(|finding| matches!(
            finding,
            VerifyFinding::Corruption(VerifyCorruption::ForeignMembership { relation, .. })
                if *relation == ENTRY
        )),
        "{report:?}"
    );
}

#[test]
fn a_stale_row_count_and_a_behind_ratchet_are_convicted() {
    let dir = TempDir::new("verify-counters");
    let db = create(&dir);
    db.write(|tx| tx.insert_dyn(ENTRY, [row("a", 1), row("b", 2)]).map(|_| ()))
        .expect("write")
        .unwrap();
    let store = db.integration_store();
    {
        let inner = &store.inner;
        let mut wtxn = inner.env.write_txn().expect("fixture txn");
        // Stored count lies.
        inner
            .meta
            .put(
                &mut wtxn,
                crate::storage::store::format::row_count_key(ENTRY).as_slice(),
                &7u64.to_be_bytes(),
            )
            .expect("fixture count");
        // Ratchet behind the allocated ids.
        inner
            .meta
            .put(
                &mut wtxn,
                crate::storage::store::format::K_NEXT_ROW_ID,
                &1u64.to_be_bytes(),
            )
            .expect("fixture ratchet");
        wtxn.commit().expect("fixture commit");
    }
    let report = db.verify_store().expect("sweep");
    let findings = report.findings();
    assert!(
        findings.iter().any(|finding| matches!(
            finding,
            VerifyFinding::Corruption(VerifyCorruption::RowCountMismatch {
                relation,
                stored: 7,
                counted: 2,
            }) if *relation == ENTRY
        )),
        "{report:?}"
    );
    assert!(
        findings.iter().any(|finding| matches!(
            finding,
            VerifyFinding::Corruption(VerifyCorruption::RowIdRatchetBehind { next: 1, .. })
        )),
        "{report:?}"
    );
}

/// Admits everything: builds the unlawful fixture the global re-judgment
/// must convict (the sweeper never trusts commit-time judgment).
struct AdmitAll;

impl CandidateJudge for AdmitAll {
    type Rejection = std::convert::Infallible;

    fn judge(
        &self,
        _candidate: &CandidateState<'_, '_>,
        _work: &WorkContext,
    ) -> StoreResult<StoreJudgment<Self::Rejection>> {
        Ok(StoreJudgment::Admitted)
    }
}

fn lawful_work() -> WorkContext {
    crate::work::ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 24,
        work_units: 1 << 40,
        timeout: std::time::Duration::from_secs(120),
    }
    .start()
    .expect("work")
}

#[test]
fn the_global_re_judgment_convicts_a_state_the_writer_never_judged() {
    let dir = TempDir::new("verify-judgment");
    let db = create(&dir);
    // Commit two rows sharing one key THROUGH the store with a bypassing
    // judge — exactly the corruption class an incremental verifier once
    // preserved forever; the sweeper re-judges globally.
    let work = lawful_work();
    let changes = {
        let mut builder = crate::ChangeSet::builder(db.schema(), work.clone());
        builder.insert(ENTRY, &row("dup", 1)).expect("draft");
        builder.insert(ENTRY, &row("dup", 2)).expect("draft");
        builder.finish().expect("sealed")
    };
    {
        let store = db.integration_store();
        let mut owner = store.writer(&work).expect("writer");
        let prepared = match owner
            .prepare(&changes, &UnindexedRows, &AdmitAll)
            .expect("prepare")
        {
            crate::storage::store::Prepared::Admitted(prepared) => prepared,
            crate::storage::store::Prepared::Rejected(impossible) => match impossible {},
        };
        let sealed = prepared
            .seal(crate::storage::store::HostChanges {
                records: &[],
                attachment: crate::storage::store::AttachmentChange::Keep,
            })
            .expect("seal");
        sealed.commit().expect("commit");
    }
    let report = db.verify_store().expect("sweep");
    assert!(
        report.findings().iter().any(|finding| matches!(
            finding,
            VerifyFinding::Judgment(violation)
                if violation.kind == bumbledb_theory::schema::StatementKind::Functionality
        )),
        "{report:?}"
    );
}
