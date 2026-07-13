//! The crash target (the crucible packet (git ecec1dc3)): durability
//! under torn commits. The engine's commit pipeline carries NAMED phase
//! boundaries under the `crashpoint` feature ([`bumbledb::CRASHPOINTS`]
//! — the table in `storage/commit.rs` is the single authority); this
//! runner generates an ops prefix plus one victim commit, replays them
//! in a CHILD process that arms one drawn point and dies there — a real
//! `abort()`, no unwinding cleanup — then proves recovery on the
//! corpse:
//!
//! 1. the store REOPENS (no wedged environment);
//! 2. `verify_store` is green;
//! 3. full contents equal the naive model at the point's expected side
//!    — the prefix state for every point before `mdb_txn_commit`, the
//!    post-commit state after it (the all-or-nothing oracle);
//! 4. re-running the victim commit succeeds and lands the post state
//!    (recovery is complete, not merely clean).
//!
//! The child-process plumbing lives HERE, not in the engine: the child
//! is the same binary re-entered, env-var-steered. Under `cargo fuzz`
//! that is libFuzzer's single-input mode on the parent's own input
//! bytes (`BUMBLEDB_CRASH_CHILD` selects the child path in [`run`]);
//! under `cargo test` it is the ignored `crash_child` test body in
//! `tests/crash.rs` (the `crates/bumbledb/tests/crash.rs` precedent),
//! which both the deterministic sweep and the corpus replay spawn. The
//! prefix runs UNARMED; the child arms `BUMBLEDB_CRASHPOINT` only
//! between the prefix and the victim, so prefix commits never trip the
//! hooks.
//!
//! Classification is the marker line the engine prints before aborting
//! (`crashpoint <name>: aborting`): a child that dies WITHOUT the
//! marker (a panic, any other signal) is a finding, and a clean exit
//! means the armed point was off the victim's path (a delete-only
//! victim never reaches the insert-side hooks; a rejected victim never
//! reaches the post-commit ones) — verified against the model's final
//! state instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::{CRASHPOINTS, CrashpointSide, Db, RelationId};
use bumbledb_bench::corpus_gen::Rng;
use bumbledb_bench::corpus_gen::opgen::{self, CrashScenario};
use bumbledb_bench::differential::Verdict as WriteVerdict;
use bumbledb_bench::naive::NaiveDb;
use bumbledb_bench::querygen::target;

use crate::{StoreDir, assert_contents, assert_green, engine_write};

/// Marks the calling process as a crash child (set by the parent's
/// spawn, checked by [`run`]).
const CHILD_VAR: &str = "BUMBLEDB_CRASH_CHILD";
/// The store directory the child creates and the parent autopsies.
const STORE_VAR: &str = "BUMBLEDB_CRASH_STORE";
/// The crashpoint name the child arms between prefix and victim.
const POINT_VAR: &str = "BUMBLEDB_CRASH_POINT";
/// The deterministic sweep's matrix cell (test-binary children).
const CELL_VAR: &str = "BUMBLEDB_CRASH_CELL";
/// The replay input file (test-binary children re-deriving from bytes).
const INPUT_VAR: &str = "BUMBLEDB_CRASH_INPUT";
/// The engine hook's own switch — armed by the CHILD, mid-process.
const ARM_VAR: &str = "BUMBLEDB_CRASHPOINT";

/// The fuzz-binary entry: parent by default; the child path when the
/// parent's spawn marked this process. Both derive the identical
/// scenario from the same bytes — the byte string is the reproduction.
///
/// The child takes its bytes from the [`INPUT_VAR`] file, NOT from
/// libFuzzer's `data`: libFuzzer tests the callback with an empty input
/// once at startup before running the argv file, so a child keyed on
/// `data` would run twice — the empty probe would create the store and
/// the real input would refuse (`AlreadyInitialized`). The once-guard
/// makes the startup probe the one real execution and the argv pass a
/// no-op.
pub fn run(data: &[u8]) {
    if std::env::var_os(CHILD_VAR).is_some() {
        use std::sync::atomic::AtomicBool;
        static RAN: AtomicBool = AtomicBool::new(false);
        if RAN.swap(true, Ordering::Relaxed) {
            return;
        }
        let input = std::env::var_os(INPUT_VAR).expect("the child input file");
        let bytes = std::fs::read(input).expect("read the child input file");
        child_body(&derive(&bytes).0);
        return;
    }
    let (case, index) = derive(data);
    let input = TempInput::new(data);
    run_case(&case, index, |store, point| {
        // Re-enter this same fuzz binary in libFuzzer single-input mode.
        // `-handle_abrt=0`: the abort must be a raw death — no crash
        // report, no artifact from the child (the parent's own panics
        // still save artifacts, which is where findings belong).
        // `-rss_limit_mb=0`: no RSS watchdog thread in the child.
        Command::new(std::env::current_exe().expect("fuzz binary path"))
            .args(["-handle_abrt=0", "-rss_limit_mb=0"])
            .arg(input.path())
            .env(CHILD_VAR, "1")
            .env(INPUT_VAR, input.path())
            .env(STORE_VAR, store)
            .env(POINT_VAR, point)
            .env_remove(ARM_VAR)
            .stdin(Stdio::null())
            .output()
            .expect("spawn the crash child")
    });
}

/// The test-binary parent for one checked-in corpus or trophy entry
/// (`tests/crash.rs`): the same case and point as [`run`] derives, with
/// the child spawned through the ignored `crash_child` test body.
pub fn replay(data: &[u8]) {
    let (case, index) = derive(data);
    let input = TempInput::new(data);
    run_case(&case, index, |store, point| {
        spawn_test_child(store, point, INPUT_VAR, input.path().as_os_str())
    });
}

/// One deterministic sweep unit (`tests/crash.rs` iterates the full
/// crashpoint × matrix product): the cell's constructed victim lies on
/// EVERY point's path (pinned in `corpus_gen::opgen`'s tests), so the
/// armed point must actually fire — a clean exit here is a failure,
/// never fuzzer luck deferred.
///
/// # Panics
///
/// When the armed point never fires, and on every recovery-oracle
/// violation ([`run_case`]).
pub fn sweep(cell: usize, index: usize) {
    let case = opgen::crash_matrix_scenario(cell);
    let outcome = run_case(&case, index, |store, point| {
        spawn_test_child(store, point, CELL_VAR, cell.to_string().as_ref())
    });
    assert_eq!(
        outcome,
        Outcome::Aborted,
        "cell {cell}: crashpoint {} never fired on its matrix victim",
        CRASHPOINTS[index].0
    );
}

/// The test-binary child dispatch, called by the ignored `crash_child`
/// test: rebuilds its case from the sweep cell or the replay input
/// file, then runs the child body. Absent steering (a bare `--ignored`
/// sweep) it does nothing.
pub fn child_entry() {
    if std::env::var_os(STORE_VAR).is_none() {
        return; // ran directly: nothing to do
    }
    if let Some(cell) = std::env::var_os(CELL_VAR) {
        let cell: usize = cell
            .to_str()
            .and_then(|s| s.parse().ok())
            .expect("a numeric sweep cell");
        child_body(&opgen::crash_matrix_scenario(cell));
    } else if let Some(input) = std::env::var_os(INPUT_VAR) {
        let bytes = std::fs::read(input).expect("read the crash input file");
        child_body(&derive(&bytes).0);
    }
}

/// The number of crashpoints — the sweep's point axis.
#[must_use]
pub fn point_count() -> usize {
    CRASHPOINTS.len()
}

/// The sweep's cell axis (re-exported so the test loops one authority).
pub const MATRIX_CELLS: usize = opgen::CRASH_MATRIX_CELLS;

/// Scenario + drawn crashpoint index, a pure function of the bytes —
/// parent and child derive identically.
fn derive(data: &[u8]) -> (CrashScenario, usize) {
    let mut rng = Rng::from_bytes(data);
    let case = opgen::random_crash_scenario(&mut rng);
    let index = usize::try_from(rng.range(CRASHPOINTS.len() as u64)).expect("index fits usize");
    (case, index)
}

/// The child body: create the store, commit the prefix UNARMED (judged
/// rejections are legal state — the model replays them identically),
/// arm the drawn point, run the victim. Death happens inside the
/// engine's hook; a victim that never reaches the armed point returns,
/// and the process exits cleanly.
fn child_body(case: &CrashScenario) {
    let store = PathBuf::from(std::env::var_os(STORE_VAR).expect("the child store dir"));
    let point = std::env::var(POINT_VAR).expect("the armed crashpoint name");
    let db = match Db::create(&store, target::Target) {
        Ok(db) => db,
        Err(err) => panic!("the crash child failed to create its store: {err:?}"),
    };
    for delta in &case.prefix {
        let _ = engine_write(&db, delta);
    }
    // Arm the point for the victim commit only. `set_var` is unsafe on
    // edition 2024 because concurrent env reads race it; this harness
    // moment is single-threaded (the store is quiescent, no engine call
    // in flight) and the only reader is the hook this arms.
    unsafe { std::env::set_var(ARM_VAR, &point) };
    let _ = engine_write(&db, &case.victim);
    unsafe { std::env::remove_var(ARM_VAR) };
}

/// A test-binary child: this same binary re-entered through libtest on
/// the ignored `crash_child` body (`--nocapture` so the abort marker
/// reaches the pipe — libtest's capture buffer dies with the process).
fn spawn_test_child(
    store: &Path,
    point: &str,
    steer_var: &str,
    steer_value: &std::ffi::OsStr,
) -> Output {
    Command::new(std::env::current_exe().expect("test binary path"))
        .args([
            "crash_child",
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(STORE_VAR, store)
        .env(POINT_VAR, point)
        .env(steer_var, steer_value)
        .env_remove(CHILD_VAR)
        .env_remove(ARM_VAR)
        .stdin(Stdio::null())
        .output()
        .expect("spawn the crash child")
}

/// How the child died.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The armed crashpoint fired: the marker line is in stderr and the
    /// process did not exit cleanly.
    Aborted,
    /// Clean exit: the armed point was off the victim's path.
    Clean,
}

/// One full case: fresh store, spawn, classify, autopsy.
fn run_case(
    case: &CrashScenario,
    index: usize,
    spawn: impl FnOnce(&Path, &str) -> Output,
) -> Outcome {
    let (name, side) = CRASHPOINTS[index];
    let store = StoreDir::new();
    let output = spawn(store.path(), name);
    let outcome = classify(&output, name);
    verify_recovery(store.path(), case, side, outcome);
    outcome
}

/// The marker classifier: crashpoint deaths announce themselves; any
/// other unclean death — a panic in the child, a different signal — is
/// a finding on this path, stderr attached.
fn classify(output: &Output, point: &str) -> Outcome {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("crashpoint {point}: aborting");
    if stderr.contains(&marker) {
        assert!(
            !output.status.success(),
            "the child printed the crashpoint marker yet exited cleanly"
        );
        return Outcome::Aborted;
    }
    if output.status.success() {
        return Outcome::Clean;
    }
    panic!(
        "the crash child died outside the armed crashpoint ({point}, status {:?}):\n{stderr}",
        output.status
    );
}

/// The four recovery oracles, judged against the naive model — the same
/// model state the child's engine walked through, re-derived here from
/// the shared scenario (rejected prefix commits change neither side).
fn verify_recovery(store: &Path, case: &CrashScenario, side: CrashpointSide, outcome: Outcome) {
    let mut model = NaiveDb::new(&target::descriptor());
    for delta in &case.prefix {
        let _ = model.apply(delta);
    }
    let prefix_model = model.clone();
    let victim_verdict = model.apply(&case.victim); // `model` is now the post state
    let expected = match outcome {
        Outcome::Clean => &model,
        Outcome::Aborted => match side {
            CrashpointSide::Prefix => &prefix_model,
            CrashpointSide::Post => {
                assert!(
                    victim_verdict.is_ok(),
                    "aborted beyond the durability boundary on a victim the model rejects: \
                     {victim_verdict:?}"
                );
                &model
            }
        },
    };
    // Oracle 1: the corpse reopens — no wedged environment, no stale
    // lock the dead writer left behind.
    let db = match Db::open(store, target::Target) {
        Ok(db) => db,
        Err(err) => panic!("the store did not reopen after the crash: {err:?}"),
    };
    // Oracle 2: the store's own auditor agrees.
    assert_audit(&db, expected, "crash recovery");
    // Oracle 3: all-or-nothing — full contents at the expected side.
    assert_contents(&db, expected, "crash recovery");
    // Oracle 4: the victim replays — same strict verdict discipline as
    // the ops target (a rejection IS the complete violation set, so the
    // sealed sets compare whole, order included), landing the model's
    // post state.
    let mut replay_model = expected.clone();
    let model_replay = match replay_model.apply(&case.victim) {
        Ok(()) => WriteVerdict::Committed,
        Err(violations) => WriteVerdict::Aborted(violations),
    };
    let engine_replay = engine_write(&db, &case.victim);
    assert_eq!(
        engine_replay, model_replay,
        "victim replay verdict divergence"
    );
    assert_audit(&db, &replay_model, "the victim replay");
    assert_contents(&db, &replay_model, "the victim replay");
}

/// The auditor oracle, with the one documented exception: the EMPTY
/// target-theory store is verify_store-red by semantics — a domain
/// quantification (closed source) is violated until its backings land,
/// and only the offline sweeper can observe that
/// (`docs/architecture/30-dependencies.md`; the naive model records the
/// same division of authority). So a recovery whose expected state is
/// empty — the very first commit torn at a prefix-side point — is
/// judged EXACTLY: its findings must equal a fresh, never-crashed
/// store's. Every non-empty expected state asserts plain green (every
/// state here carries the seed world, and green-after-commit is the
/// ops target's standing oracle).
fn assert_audit(db: &Db<target::Target>, model: &NaiveDb, when: &str) {
    let empty = (0..target::TARGET_RELATIONS).all(|rel| model.relation(RelationId(rel)).is_empty());
    if !empty {
        assert_green(db, when);
        return;
    }
    let control = StoreDir::new();
    let control_db = match Db::create(control.path(), target::Target) {
        Ok(db) => db,
        Err(err) => panic!("the control store failed to create: {err:?}"),
    };
    let control_findings = match control_db.verify_store() {
        Ok(report) => report.findings,
        Err(err) => panic!("verify_store errored on the control store: {err:?}"),
    };
    let recovered_findings = match db.verify_store() {
        Ok(report) => report.findings,
        Err(err) => panic!("verify_store errored after {when}: {err:?}"),
    };
    assert_eq!(
        recovered_findings, control_findings,
        "the recovered empty store's findings diverge from a fresh store's after {when}"
    );
}

/// The parent's input handoff: the child re-derives the scenario from
/// the exact fuzzer bytes, so the bytes travel as a file (libFuzzer's
/// single-input argv; the replay child's `INPUT_VAR`).
struct TempInput(PathBuf);

static INPUT_SEQ: AtomicU64 = AtomicU64::new(0);

impl TempInput {
    fn new(data: &[u8]) -> Self {
        let seq = INPUT_SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("bumbledb-crash-input-{}-{seq}", std::process::id()));
        std::fs::write(&path, data).expect("write the crash input file");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempInput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
