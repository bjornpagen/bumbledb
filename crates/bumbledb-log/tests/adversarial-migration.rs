//! P12 adversarial integration: REAL process kill schedules over the landed
//! native migration executor (P09 authored the deterministic in-process
//! halves and routed the real process-kill schedules here — MIG-03/MIG-08
//! kill arms, MIG-09/MIG-14 one-lineage/activation-once under crash-resume,
//! OPS-001 "kill at each cutover step").
//!
//! Arms:
//! - SIGKILL while `migrate` is staging (polled on the private staging
//!   namespace appearing): the durable freeze survives, no timer thaws the
//!   source, staging is never adopted, and the resumed operation converges
//!   to exactly one target lineage with one explicit activation;
//! - a parked `ReadyToSwitch` holder is `SIGSTOPped` (a merely suspended owner
//!   still fences competing source opens), then `SIGKILLed`: the successor
//!   re-derives the SAME ready target by verified reuse and activates once;
//!   a matching retry is evidence, not mutation.
//!
//! Child arms re-exec this binary with owned subprocesses and protocol readers.

#![cfg(unix)]

#[path = "migration_support/mod.rs"]
mod support;

#[path = "lane_support/process.rs"]
mod process;

use process::TestChild;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{ChangeSet, Db, RelationId, Uuid, Value};

use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{CommandId, CommandResult, Condition, ReceiptEpoch, RequestId};
use bumbledb_log::migration::executor::{
    LocalMigration, MigrateOutcome, MigrationStatus, StepInput, SuffixRequest, activate_target,
};
use bumbledb_log::migration::manifest::Manifest;
use bumbledb_log::writer::{LocalHistory, SubmitOutcome};

use support::{
    LIMITS, base_schema, db_id, incarnation, manifest, op, pinned_schema, plan_pinned, plan_tagged,
    tagged_schema, work,
};

const CHILD_ENV: &str = "BDB_P12_MIGRATION_CHILD";
const DIR_ENV: &str = "BDB_P12_MIGRATION_DIR";
const WAIT: Duration = Duration::from_secs(30);

fn fresh_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root =
        std::env::temp_dir().join(format!("bdb-p12-mig-{}-{name}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create test root");
    root
}

fn spawn_child(mode: &str, dir: &Path) -> TestChild {
    TestChild::spawn(
        ProcessCommand::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "child_process_entry",
                "--ignored",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env(CHILD_ENV, mode)
            .env(DIR_ENV, dir),
        WAIT,
    )
}

fn park() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

/// The fixed source: two Note rows under the recorded base schema.
fn build_source(dir: &Path) -> (Arc<Db<SchemaDescriptor>>, LocalHistory<SchemaDescriptor>) {
    let db = Arc::new(
        Db::create(&dir.join("source"), base_schema(), work())
            .expect("create store")
            .expect("empty store admits"),
    );
    let history = LocalHistory::create(
        Arc::clone(&db),
        db_id(0xa1),
        incarnation(0xb1),
        op(0xc1),
        LIMITS,
        &work(),
    )
    .expect("local history creates");
    let mut draft = ChangeSet::builder(history.db().schema(), work());
    draft
        .insert(
            RelationId(0),
            &[Value::U64(1), Value::String("alpha".into())],
        )
        .expect("insert");
    draft
        .insert(
            RelationId(0),
            &[Value::U64(2), Value::String("beta".into())],
        )
        .expect("insert");
    let changes = draft.finish().expect("draft finishes");
    let command = Command::seal(
        CommandMetadata {
            identity: history.identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Uuid::from_bytes([0x01; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .expect("command seals");
    match history.submit(&command, &work()) {
        SubmitOutcome::Decided { .. } => {}
        other => panic!("seed submit failed: {other:?}"),
    }
    (db, history)
}

fn steps_full() -> Vec<StepInput> {
    vec![
        StepInput {
            plan: plan_pinned(),
            to_descriptor: pinned_schema(),
        },
        StepInput {
            plan: plan_tagged(),
            to_descriptor: tagged_schema(),
        },
    ]
}

fn request<'a>(manifest: &'a Manifest, steps: &'a [StepInput]) -> SuffixRequest<'a> {
    SuffixRequest {
        operation: op(0xd1),
        manifest,
        source_descriptor: base_schema(),
        steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe1),
    }
}

/// Reopen the killed child's source and drive the SAME operation to one
/// activated lineage; assert the freeze/one-lineage/activation-once rules.
fn resume_and_activate(root: &Path) {
    let start = Instant::now();
    let db = loop {
        match Db::open(&root.join("source"), base_schema(), work()) {
            Ok(db) => break Arc::new(db),
            Err(error) => {
                assert!(
                    start.elapsed() < WAIT,
                    "death releases the source store: {error}"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let history = LocalHistory::open(Arc::clone(&db), LIMITS).expect("successor opens the source");
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let chain = manifest();
    let steps = steps_full();
    let req = request(&chain, &steps);

    // Whatever the kill point was, no timer thawed the source: it is either
    // still Pending (killed before the durable freeze) or Frozen under the
    // child's exact operation — never silently Active-with-new-writes.
    match runner.status(&chain, &work()).expect("status reads") {
        MigrationStatus::Pending { applied, pending } => {
            assert_eq!(applied, 0);
            assert_eq!(pending, 2);
        }
        MigrationStatus::Frozen { operation, .. } => assert_eq!(operation, op(0xd1)),
        MigrationStatus::UpToDate { .. } => panic!("a killed migration cannot be UpToDate"),
    }

    // The resumed SAME operation converges: fresh staging or verified reuse
    // of the child's published target — one lineage either way.
    let outcome = runner
        .migrate(&req, &work())
        .expect("resumed migrate converges");
    let activation_ref = match outcome {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("expected ReadyToSwitch, got {other:?}"),
    };
    // Still frozen until the explicit activation.
    match runner.status(&chain, &work()).expect("status reads") {
        MigrationStatus::Frozen {
            operation,
            target_present,
            target_cancelled,
            ..
        } => {
            assert_eq!(operation, op(0xd1));
            assert!(target_present, "the verified target is installed");
            assert!(!target_cancelled);
        }
        other => panic!("the source stays frozen after ReadyToSwitch: {other:?}"),
    }

    // Exactly one explicit activation; the matching retry is recorded
    // evidence, never a second transition.
    let first = activate_target(
        &root.join("targets"),
        &activation_ref,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .expect("explicit activation");
    let retry = activate_target(
        &root.join("targets"),
        &activation_ref,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .expect("matching activation retry returns evidence");
    assert_eq!(
        first.activation, retry.activation,
        "one-time activation evidence is stable"
    );

    // A repeated migrate reports the recorded activation, not a new lineage.
    match runner
        .migrate(&req, &work())
        .expect("post-activation migrate")
    {
        MigrateOutcome::AlreadyActivated { .. } => {}
        other => panic!("expected AlreadyActivated, got {other:?}"),
    }
}

/// The child arms.
#[test]
#[ignore = "subprocess entrypoint invoked by the parent crash tests"]
fn child_process_entry() {
    let mode = std::env::var(CHILD_ENV).expect("parent selects the child mode");
    let dir = PathBuf::from(std::env::var(DIR_ENV).expect("child dir"));
    match mode.as_str() {
        // Announce, then migrate; the parent kills us while staging (polled
        // on the targets namespace appearing) or right after completion.
        "migrate-and-park" => {
            let (_db, history) = build_source(&dir);
            let runner = LocalMigration::new(&history, &dir.join("targets"), LIMITS);
            let chain = manifest();
            let steps = steps_full();
            println!("MIGRATING");
            let _ = std::io::stdout().flush();
            let _ = runner.migrate(&request(&chain, &steps), &work());
            println!("DONE");
            let _ = std::io::stdout().flush();
            park();
        }
        // Complete to ReadyToSwitch, hold the frozen source, then park.
        "ready-and-park" => {
            let (_db, history) = build_source(&dir);
            let runner = LocalMigration::new(&history, &dir.join("targets"), LIMITS);
            let chain = manifest();
            let steps = steps_full();
            match runner.migrate(&request(&chain, &steps), &work()) {
                Ok(MigrateOutcome::ReadyToSwitch { .. }) => {}
                other => panic!("child migrate: {other:?}"),
            }
            println!("READY");
            let _ = std::io::stdout().flush();
            park();
        }
        other => panic!("unknown child mode {other}"),
    }
}

/// MIG-03/MIG-08 kill arm: SIGKILL during staged execution. The durable
/// freeze survives real death, abandoned staging is never adopted, and the
/// resumed operation converges to one activated lineage (OPS-001 "kill at
/// each cutover step, invalid destination never current").
#[test]
fn a_kill_during_staging_leaves_a_frozen_resumable_source() {
    let root = fresh_root("stagekill");
    let mut child = spawn_child("migrate-and-park", &root);
    child.await_marker("MIGRATING");
    // Kill as soon as the target namespace materializes on disk (mid-work),
    // or after a bounded delay if the tiny fixture already finished — the
    // safety assertions below hold under either race outcome.
    let start = Instant::now();
    while !root.join("targets").exists() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(2));
    }
    child.process.kill().expect("kill staging child");
    let _ = child.process.wait().expect("reap");
    resume_and_activate(&root);
    let _ = std::fs::remove_dir_all(&root);
}

/// MIG-09/MIG-14 crash-resume arm: a `ReadyToSwitch` holder is first merely
/// suspended (still the exclusive source owner — a competing open refuses),
/// then killed. The successor re-derives the SAME published target by
/// verified reuse and activates exactly once.
#[test]
fn a_killed_ready_to_switch_holder_resumes_to_one_activated_lineage() {
    let root = fresh_root("readykill");
    let mut child = spawn_child("ready-and-park", &root);
    child.await_marker("READY");
    // Suspended, not dead: the source store stays fenced (REP-005 shape).
    child.signal("STOP");
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        Db::open(&root.join("source"), base_schema(), work()).is_err(),
        "a paused ReadyToSwitch holder still owns its frozen source"
    );
    child.signal("CONT");
    child.process.kill().expect("kill ready child");
    let _ = child.process.wait().expect("reap");
    resume_and_activate(&root);
    let _ = std::fs::remove_dir_all(&root);
}
