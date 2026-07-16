//! The WRITEMAP commit-window kill sweep: random-timing SIGKILL against
//! a child committing in a loop — the one crash surface the
//! deterministic crashpoint sweep structurally cannot reach.
//!
//! The gap (`storage/commit.rs` CRASHPOINTS): the named points bracket
//! `mdb_txn_commit` (`after-judgment` before it, `after-commit` after
//! it) but nothing cuts INSIDE it — and inside that window is exactly
//! where `MDB_WRITEMAP` changes the write pattern (dirty pages live in
//! the shared page cache, the meta update is a plain memcpy into the
//! map a signal can tear mid-struct, not a single `pwrite`). This
//! harness fills the gap with the only instrument that can: a real
//! SIGKILL at a uniformly random moment of a commit loop, so kills land
//! at every instruction boundary of the pipeline, `mdb_txn_commit`'s
//! interior included.
//!
//! The shape (the `crates/bumbledb/tests/crash.rs` precedent, house
//! plumbing as `fuzz/src/crash.rs`): the CHILD is this same test binary
//! re-entered on the ignored `kill_child` body (`fuzz/tests/kill.rs`),
//! env-var steered. It creates a store of the drawn kind on the scratch
//! volume (`BUMBLEDB_SCRATCH_DIR`, the ramdisk sanction), prints a
//! ready marker, and commits batch after batch forever — commit `k`
//! inserts the [`BATCH`] facts `(k, j, pad(k, j))`, so EVERY committed
//! state is a pure function of the high-water commit number. The parent
//! sleeps a seeded uniform delay in `[0, KILL_WINDOW_US)` and SIGKILLs;
//! the delay window spans hundreds of commit periods, so the kill's
//! phase within the pipeline is uniform and the in-commit-window hit
//! rate equals the commit duty cycle (measured first, logged per
//! session — the calibration).
//!
//! The invariant on the corpse (both kinds — the ephemeral admission's
//! claim is that `WRITEMAP|NOSYNC` changes machine-crash durability and
//! NOTHING about process-kill atomicity):
//!
//! 1. the store REOPENS through its kind's constructor — no panic, no
//!    refusal, no wedged environment;
//! 2. `verify_store` is green;
//! 3. the contents are a complete batch prefix `1..=N` for some `N` —
//!    every batch below the high-water whole, every pad byte-exact, no
//!    torn batch, no gap, no third state;
//! 4. the store WORKS: batch `N + 1` commits and lands whole.
//!
//! On any violation the round panics with the full reproduction context
//! (kind, seed, round, delay, store path) and the store directory is
//! PRESERVED for autopsy (the success path deletes it; the ramdisk
//! note: a preserved ephemeral corpse holds the full 4 GiB data file,
//! so copy it off the volume before the next long session).
//!
//! Two lanes, run by `fuzz/tests/kill.rs`: the EPHEMERAL lane
//! (`WRITEMAP|NOSYNC` — the surface under test) and the DURABLE lane as
//! the control (default flags, `pwrite`-based commits). The long lane
//! (>= 2,000 kills each) is `#[ignore]`d; the ~30-round smoke runs in
//! `scripts/check.sh`. Sessions are recorded in `fuzz/SESSIONS.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bumbledb::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};
use bumbledb::{Db, RelationId, StoreKind, Value};
use bumbledb_bench::corpus_gen::Rng;

/// The store directory the child creates and the parent autopsies.
const STORE_VAR: &str = "BUMBLEDB_KILL_STORE";
/// The store-kind axis: set (to any value) for a `Db::ephemeral` child.
const EPHEMERAL_VAR: &str = "BUMBLEDB_KILL_EPHEMERAL";
/// The long lane's round count (default [`LONG_ROUNDS`]).
const ROUNDS_VAR: &str = "BUMBLEDB_KILL_ROUNDS";
/// The session seed (default: wall-clock nanos; printed either way).
const SEED_VAR: &str = "BUMBLEDB_KILL_SEED";
/// The child's readiness marker: store created, commit loop entered.
const READY_MARKER: &str = "kill-child ready";

/// Facts per commit: enough to dirty several pages per transaction
/// (widening the mid-`mdb_txn_commit` page-write surface) while keeping
/// per-round verification scans cheap.
pub const BATCH: u64 = 32;
/// The kill-delay window FLOOR in microseconds. The working window is
/// derived per kind from the calibration — at least
/// [`WINDOW_COMMIT_PERIODS`] mean commit periods — so a slow durable
/// commit (macOS `F_FULLFSYNC` on `/tmp` runs 5–20+ ms) cannot turn the
/// whole session vacuous by outlasting a fixed window. At ~0.05 ms per
/// ephemeral commit the floor spans hundreds of commit periods, so the
/// kill's phase within the commit pipeline is uniform.
const KILL_WINDOW_US: u64 = 20_000;
/// The derived window's width in mean commit periods (see
/// [`KILL_WINDOW_US`]): enough room that most rounds survive at least
/// one commit, keeping the vacuity assert honest on any device.
const WINDOW_COMMIT_PERIODS: u64 = 4;
/// The readiness deadline: a child that cannot create its store and
/// enter the commit loop within this is a failure, not a hang.
const READY_TIMEOUT: Duration = Duration::from_secs(30);
/// The long lane's default rounds per kind (override: [`ROUNDS_VAR`]).
const LONG_ROUNDS: u64 = 2_000;
/// Calibration commits per kind (after [`CALIBRATION_WARMUP`]).
const CALIBRATION_COMMITS: u64 = 128;
const CALIBRATION_WARMUP: u64 = 16;
/// SIGKILL — the only death the parent inflicts, and the only one it
/// accepts (spelled numerically: the harness carries no libc dep).
const SIGKILL: i32 = 9;

/// The kill ledger's one relation.
const REL: RelationId = RelationId(0);
/// The pad column width: multi-word payloads spread batches across
/// pages, and a byte-exact pad check convicts torn fact bodies.
const PAD_LEN: usize = 64;

/// The kill ledger: one relation, zero dependency statements — every
/// insert commits, the judgment phase is near-nothing, and the commit
/// loop's wall time concentrates in the LMDB pipeline this sweep aims
/// kills at (the duty cycle the calibration measures).
fn descriptor() -> SchemaDescriptor {
    let field = |name: &str, value_type| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "KillLedger".into(),
            fields: vec![
                field("commit", ValueType::U64),
                field("row", ValueType::U64),
                field(
                    "pad",
                    ValueType::FixedBytes {
                        len: PAD_LEN as u16,
                    },
                ),
            ],
            extension: None,
        }],
        statements: vec![],
    }
}

/// The deterministic pad: a seeded [`Rng`] stream keyed by the fact's
/// identity, recomputed byte-exact by the verifier.
fn pad(commit: u64, row: u64) -> [u8; PAD_LEN] {
    let mut rng = Rng::new(commit.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ row);
    let mut bytes = [0u8; PAD_LEN];
    for chunk in bytes.as_chunks_mut::<8>().0 {
        *chunk = rng.u64().to_le_bytes();
    }
    bytes
}

/// Fact `(k, j)`: commit `k`'s `j`-th row, pure function of identity.
fn fact(commit: u64, row: u64) -> [Value; 3] {
    [
        Value::U64(commit),
        Value::U64(row),
        Value::FixedBytes(pad(commit, row).into()),
    ]
}

/// The store constructor per kind: `create` for the fresh durable
/// store, the create-or-open `ephemeral` for the ephemeral one.
fn create_kind(store: &Path, kind: StoreKind) -> Db<SchemaDescriptor> {
    let opened = match kind {
        StoreKind::Durable => Db::create(store, descriptor()),
        StoreKind::Ephemeral => Db::ephemeral(store, descriptor()),
    };
    match opened {
        Ok(db) => db,
        Err(err) => panic!("the kill child failed to create its {kind} store: {err:?}"),
    }
}

/// One whole batch through the real write path. Statement-free schema:
/// any refusal here is a finding, never a judged rejection.
fn commit_batch(db: &Db<SchemaDescriptor>, commit: u64) {
    let verdict = db.write(|tx| {
        for row in 0..BATCH {
            tx.insert_dyn(REL, &fact(commit, row))?;
        }
        Ok(())
    });
    if let Err(err) = verdict {
        panic!("kill-ledger commit {commit} refused: {err:?}");
    }
}

/// The child body, dispatched by the ignored `kill_child` test: create
/// the store, announce readiness (the parent's kill timer starts at the
/// marker), then commit batches until the SIGKILL lands. Absent
/// steering (a bare `--ignored` sweep) it does nothing.
pub fn child_entry() {
    let Some(store) = std::env::var_os(STORE_VAR) else {
        return; // ran directly: nothing to do
    };
    let kind = if std::env::var_os(EPHEMERAL_VAR).is_some() {
        StoreKind::Ephemeral
    } else {
        StoreKind::Durable
    };
    let db = create_kind(&PathBuf::from(store), kind);
    // libtest pipes are block-buffered: flush or the parent never sees
    // the marker.
    println!("{READY_MARKER}");
    std::io::stdout().flush().expect("flush the ready marker");
    // The loop only ever ends by the parent's SIGKILL (the wrap is
    // 2^64 commits away — the kill lands within milliseconds).
    for commit in 1..=u64::MAX {
        commit_batch(&db, commit);
    }
}

/// The session seed: [`SEED_VAR`] when set (the reproduction hook — the
/// kill DELAYS replay exactly; the kill instants themselves are OS
/// scheduling), wall-clock nanos otherwise. Printed at session start
/// and carried in every failure message.
#[must_use]
pub fn session_seed() -> u64 {
    std::env::var(SEED_VAR).map_or_else(
        |_| {
            u64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock after epoch")
                    .as_nanos()
                    & u128::from(u64::MAX),
            )
            .expect("masked to 64 bits")
        },
        |seed| seed.parse().expect("a numeric BUMBLEDB_KILL_SEED"),
    )
}

/// The long lane's round count: [`ROUNDS_VAR`] or [`LONG_ROUNDS`].
#[must_use]
pub fn long_rounds() -> u64 {
    std::env::var(ROUNDS_VAR).map_or(LONG_ROUNDS, |rounds| {
        rounds.parse().expect("a numeric BUMBLEDB_KILL_ROUNDS")
    })
}

/// One lane of the sweep: calibrate (and log the in-window estimate),
/// then `rounds` seeded random-timing kills against `kind`, each round
/// autopsied under the four-point invariant.
///
/// # Panics
///
/// On every invariant violation, with the reproduction context in the
/// message and the store directory preserved.
pub fn sweep(kind: StoreKind, rounds: u64, seed: u64) {
    refuse_stale_corpses();
    let calibration = calibrate();
    // The working window: at least WINDOW_COMMIT_PERIODS mean commit
    // periods (loop period = commit time / duty), never below the floor
    // — derived, so the device's sync cost cannot vacate the session.
    let period_us = calibration.commit(kind).as_secs_f64() * 1e6 / calibration.duty(kind);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let window_us = KILL_WINDOW_US.max((period_us * WINDOW_COMMIT_PERIODS as f64).ceil() as u64);
    // The estimate log stays honest per kind: the durable number is the
    // sync-surplus estimate of kills inside mdb_txn_commit itself; the
    // ephemeral duty covers the WHOLE write call (mostly put staging —
    // trivially recoverable), and the torn-meta sliver inside it is
    // O(µs) — expect O(1) sliver hits per thousands of rounds, not per
    // smoke. The invariants convict a torn meta whenever one lands.
    let window = match kind {
        StoreKind::Durable => "inside mdb_txn_commit (sync-surplus estimate)",
        StoreKind::Ephemeral => {
            "across the whole write call (the torn-meta sliver is a µs-scale \
             subset — sliver hits are O(1) per thousands of rounds)"
        }
    };
    eprintln!(
        "kill sweep ({kind}): seed {seed}, {rounds} rounds, window {window_us} us \
         (~{WINDOW_COMMIT_PERIODS} commit periods) — expected in-commit-call kills \
         ~{:.0} ({:.0}% duty), {window} ~{:.0}",
        calibration.duty(kind) * rounds as f64,
        calibration.duty(kind) * 100.0,
        calibration.in_window_fraction(kind) * rounds as f64,
    );
    let mut rng = Rng::new(seed);
    let mut committed_batches: u64 = 0;
    let mut empty_rounds: u64 = 0;
    for round in 0..rounds {
        let delay = Duration::from_micros(rng.range(window_us));
        let survived = run_round(kind, seed, round, delay);
        committed_batches += survived;
        empty_rounds += u64::from(survived == 0);
    }
    eprintln!(
        "kill sweep ({kind}): {rounds} kills, 0 violations; {committed_batches} surviving \
         batches ({:.1} mean), {empty_rounds} pre-first-commit kills",
        committed_batches as f64 / rounds as f64,
    );
    assert!(
        committed_batches > 0,
        "kill sweep ({kind}, seed {seed}): every kill landed before the first commit — \
         the session was vacuous, no commit window was ever exercised"
    );
}

/// The artifacts discipline, extended to corpses (the `fuzz.sh`
/// precedent: a session refuses to start over untriaged evidence): any
/// `bumbledb-kill-*` entry under the scratch root from ANOTHER process
/// is either a preserved violation or debris from an interrupted
/// session — and on the 5 GiB ramdisk a single 4 GiB ephemeral corpse
/// also starves every following store. Autopsy it
/// ([`autopsy`], the `BUMBLEDB_KILL_AUTOPSY` operator test) or remove
/// it before sweeping.
fn refuse_stale_corpses() {
    let root =
        std::env::var_os("BUMBLEDB_SCRATCH_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
    let own = format!("bumbledb-kill-{}-", std::process::id());
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    let stale: Vec<String> = entries
        .filter_map(|entry| Some(entry.ok()?.file_name().to_string_lossy().into_owned()))
        .filter(|name| name.starts_with("bumbledb-kill-") && !name.starts_with(&own))
        .collect();
    assert!(
        stale.is_empty(),
        "kill sweep: stale corpse(s) under {} — autopsy (BUMBLEDB_KILL_AUTOPSY) or remove \
         before a new session: {stale:?}",
        root.display()
    );
}

/// The operator autopsy: the four-point corpse invariant
/// ([`verify_round`]'s reopen / auditor / all-or-nothing prefix /
/// working post-recovery commit) over a PRESERVED store directory.
/// Returns the surviving high-water batch count. Panics exactly where
/// the sweep would — a clean return means the corpse holds no finding.
pub fn autopsy(store: &Path, kind: StoreKind) -> u64 {
    let ctx = format!("kill autopsy ({kind}, store {})", store.display());
    verify_round(store, kind, &ctx)
}

/// One round: spawn, await readiness, sleep the drawn delay, SIGKILL,
/// classify the death, autopsy. Returns the surviving batch count `N`.
fn run_round(kind: StoreKind, seed: u64, round: u64, delay: Duration) -> u64 {
    let mut store = KillStore::new(kind, &round.to_string());
    let ctx = format!(
        "kill sweep ({kind}, seed {seed}, round {round}, delay {} us, store {})",
        delay.as_micros(),
        store.path().display()
    );
    let mut child = ChildGuard(spawn_child(store.path(), kind));
    let ready = BufReader::new(child.0.stdout.take().expect("piped child stdout"));
    await_ready(ready, &mut child, &ctx);
    std::thread::sleep(delay);
    child
        .0
        .kill()
        .unwrap_or_else(|err| panic!("{ctx}: SIGKILL refused: {err}"));
    let status = child
        .0
        .wait()
        .unwrap_or_else(|err| panic!("{ctx}: wait refused: {err}"));
    // The one accepted death is OUR kill. Anything else — a panic in
    // the child's commit loop, an abort, a different signal — is a
    // finding on this path, stderr attached.
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            status.signal() == Some(SIGKILL),
            "{ctx}: the child died on its own before the kill (status {status:?}):\n{}",
            drain(child.0.stderr.take())
        );
    }
    let survived = verify_round(store.path(), kind, &ctx);
    store.delete_on_drop();
    survived
}

/// The autopsy — the four-point invariant on the corpse. Returns the
/// surviving high-water batch count.
fn verify_round(store: &Path, kind: StoreKind, ctx: &str) -> u64 {
    // Point 1: the corpse reopens through its kind's constructor. The
    // ephemeral reopen runs the non-mutating probe plus the WRITEMAP
    // reopen — both must cross the killed store without refusal.
    let reopened = match kind {
        StoreKind::Durable => Db::open(store, descriptor()),
        StoreKind::Ephemeral => Db::ephemeral(store, descriptor()),
    };
    let db = match reopened {
        Ok(db) => db,
        Err(err) => panic!("{ctx}: the killed store did not reopen: {err:?}"),
    };
    // Point 2: the store's own auditor agrees.
    crate::assert_green(&db, ctx);
    // Point 3: all-or-nothing — a complete batch prefix, nothing else.
    let survived = assert_prefix(&db, ctx);
    // Point 4: the store WORKS — the next batch commits and lands.
    commit_batch(&db, survived + 1);
    let after = assert_prefix(&db, ctx);
    assert_eq!(
        after,
        survived + 1,
        "{ctx}: the post-recovery batch did not land"
    );
    crate::assert_green(&db, ctx);
    survived
}

/// The prefix oracle: the relation's full contents are EXACTLY the
/// batches `1..=N` for some `N` — every fact shape-legal, every pad
/// byte-exact, every batch below the high-water complete, no gaps.
/// Returns `N`.
fn assert_prefix(db: &Db<SchemaDescriptor>, ctx: &str) -> u64 {
    let scanned: Result<Vec<Vec<Value>>, bumbledb::Error> = db.read(|snap| {
        let mut facts = Vec::new();
        for fact in snap.scan(REL)? {
            facts.push(fact?);
        }
        Ok(facts)
    });
    let facts = match scanned {
        Ok(facts) => facts,
        Err(err) => panic!("{ctx}: the recovery scan refused: {err:?}"),
    };
    let mut batches: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    for fact in facts {
        let [
            Value::U64(commit),
            Value::U64(row),
            Value::FixedBytes(bytes),
        ] = fact.as_slice()
        else {
            panic!("{ctx}: a torn fact shape survived: {fact:?}");
        };
        assert!(
            bytes.as_ref() == pad(*commit, *row),
            "{ctx}: fact ({commit}, {row}) survived with a torn pad"
        );
        batches.entry(*commit).or_default().insert(*row);
    }
    let survived = batches.keys().next_back().copied().unwrap_or(0);
    for commit in 1..=survived {
        let Some(rows) = batches.get(&commit) else {
            panic!(
                "{ctx}: batch {commit} is missing below the high-water {survived} — \
                 a gap, the third state"
            );
        };
        assert!(
            rows.len() as u64 == BATCH && rows.iter().eq(&(0..BATCH).collect::<BTreeSet<u64>>()),
            "{ctx}: batch {commit} is torn — {} of {BATCH} rows survived",
            rows.len()
        );
    }
    assert!(
        batches.len() as u64 == survived,
        "{ctx}: {} distinct batches for high-water {survived} — a batch beyond the prefix",
        batches.len()
    );
    survived
}

/// A kill child: this same test binary re-entered through libtest on
/// the ignored `kill_child` body (`--nocapture` so the ready marker
/// reaches the pipe — libtest's capture buffer never flushes for a
/// process that dies by signal).
fn spawn_child(store: &Path, kind: StoreKind) -> Child {
    let mut command = Command::new(std::env::current_exe().expect("test binary path"));
    command
        .args([
            "kill_child",
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(STORE_VAR, store)
        // The crashpoint feature is compiled into this crate's one
        // bumbledb; a leaked armed variable would turn kills into
        // deterministic aborts.
        .env_remove("BUMBLEDB_CRASHPOINT");
    match kind {
        StoreKind::Ephemeral => command.env(EPHEMERAL_VAR, "1"),
        StoreKind::Durable => command.env_remove(EPHEMERAL_VAR),
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the kill child")
}

/// Reads child stdout until the ready marker. EOF first means the
/// child died before entering its commit loop — a finding (or an
/// environmental refusal, e.g. a scratch volume too small for the
/// ephemeral map), reported with the child's stderr.
fn await_ready(reader: impl BufRead + Send + 'static, child: &mut ChildGuard, ctx: &str) {
    // The blocking read lives on its own thread so readiness carries a
    // deadline: a child wedged inside its store open (an env hang)
    // fails the round instead of hanging the gate forever. On timeout
    // the panic unwinds through [`ChildGuard`], which kills the child;
    // the reader thread then sees EOF and exits.
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                // Suffix match: libtest prints its `test kill_child ...`
                // prefix on the SAME line — the marker is not alone on it.
                Ok(n) if n > 0 => {
                    if line.trim_end().ends_with(READY_MARKER) {
                        let _ = sender.send(true);
                        return;
                    }
                }
                _ => {
                    let _ = sender.send(false);
                    return;
                }
            }
        }
    });
    match receiver.recv_timeout(READY_TIMEOUT) {
        Ok(true) => {}
        Ok(false) => {
            let status = child.0.wait();
            panic!(
                "{ctx}: the child died before readiness (status {status:?}):\n{}",
                drain(child.0.stderr.take())
            );
        }
        Err(_) => panic!(
            "{ctx}: the child never reached readiness within {}s",
            READY_TIMEOUT.as_secs()
        ),
    }
}

fn drain(stderr: Option<impl Read>) -> String {
    let mut text = String::new();
    if let Some(mut stderr) = stderr {
        let _ = stderr.read_to_string(&mut text);
    }
    text
}

/// A child that dies with its round: any panic between spawn and kill
/// must not leak an immortal commit loop.
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The commit duty cycle, measured on THIS machine before the kills:
/// per kind, the mean write-call time and the fraction of loop wall
/// time inside `db.write` (the loop is back-to-back write calls, so the
/// duty cycle IS the probability a uniformly-timed kill lands inside
/// the commit call — staging through `mdb_txn_commit` to the memo
/// update). The `mdb_txn_commit` sub-window is not separately
/// observable from outside the process, so the durable lane's share is
/// estimated from the kind differential: a durable commit is an
/// ephemeral commit plus the page `pwrite`s and the sync boundary, and
/// that surplus lives inside `mdb_txn_commit` (a LOWER bound — the
/// ephemeral commit's own meta write is in the window too). On the
/// EPHEMERAL kind the estimate is the duty cycle itself: under
/// `MDB_WRITEMAP` the dirty pages hit the shared map from the first
/// staged put, so the OS-flushed-dirty-page surface under test spans
/// the whole write call, torn-meta sliver included.
#[derive(Clone, Copy)]
struct Calibration {
    durable_commit: Duration,
    durable_duty: f64,
    ephemeral_commit: Duration,
    ephemeral_duty: f64,
}

impl Calibration {
    fn duty(&self, kind: StoreKind) -> f64 {
        match kind {
            StoreKind::Durable => self.durable_duty,
            StoreKind::Ephemeral => self.ephemeral_duty,
        }
    }

    /// The kind's mean commit-call time.
    fn commit(&self, kind: StoreKind) -> Duration {
        match kind {
            StoreKind::Durable => self.durable_commit,
            StoreKind::Ephemeral => self.ephemeral_commit,
        }
    }

    /// The estimated fraction of uniformly-timed kills landing inside
    /// the kind's window under test: `mdb_txn_commit` itself on the
    /// durable kind, the whole WRITEMAP dirty-page window on the
    /// ephemeral one (see the struct doc for the model).
    fn in_window_fraction(&self, kind: StoreKind) -> f64 {
        match kind {
            StoreKind::Durable => {
                let surplus = self
                    .durable_commit
                    .saturating_sub(self.ephemeral_commit)
                    .as_secs_f64();
                self.durable_duty * (surplus / self.durable_commit.as_secs_f64())
            }
            StoreKind::Ephemeral => self.ephemeral_duty,
        }
    }
}

fn calibrate() -> Calibration {
    // Once per process: the smoke calls `sweep` per kind, and 2×144
    // calibration commits (durable ones under `F_FULLFSYNC`) are dead
    // time worth spending exactly once.
    static CALIBRATION: std::sync::OnceLock<Calibration> = std::sync::OnceLock::new();
    *CALIBRATION.get_or_init(calibrate_uncached)
}

fn calibrate_uncached() -> Calibration {
    let (durable_commit, durable_duty) = calibrate_kind(StoreKind::Durable);
    let (ephemeral_commit, ephemeral_duty) = calibrate_kind(StoreKind::Ephemeral);
    eprintln!(
        "kill sweep calibration ({BATCH}-fact batches, {CALIBRATION_COMMITS} commits/kind): \
         durable {:.1} us/commit (duty {durable_duty:.2}), ephemeral {:.1} us/commit \
         (duty {ephemeral_duty:.2})",
        durable_commit.as_secs_f64() * 1e6,
        ephemeral_commit.as_secs_f64() * 1e6,
    );
    Calibration {
        durable_commit,
        durable_duty,
        ephemeral_commit,
        ephemeral_duty,
    }
}

/// One kind's measurement: the child's exact loop body, timed in
/// process — mean commit-call time and the commit duty cycle.
fn calibrate_kind(kind: StoreKind) -> (Duration, f64) {
    let mut store = KillStore::new(kind, "calibrate");
    let db = create_kind(store.path(), kind);
    for commit in 1..=CALIBRATION_WARMUP {
        commit_batch(&db, commit);
    }
    let loop_start = Instant::now();
    let mut in_commit = Duration::ZERO;
    for commit in CALIBRATION_WARMUP + 1..=CALIBRATION_WARMUP + CALIBRATION_COMMITS {
        let call = Instant::now();
        commit_batch(&db, commit);
        in_commit += call.elapsed();
    }
    let total = loop_start.elapsed();
    store.delete_on_drop();
    (
        in_commit / u32::try_from(CALIBRATION_COMMITS).expect("small constant"),
        in_commit.as_secs_f64() / total.as_secs_f64(),
    )
}

/// A per-round store directory under the scratch root, PRESERVED by
/// default: only a round that passes its whole autopsy calls
/// [`KillStore::delete_on_drop`], so every panic path leaves the corpse
/// for minimization. The success-path drop truncates `data.mdb` before
/// unlinking — the same synchronous-reclamation discipline as
/// [`crate::StoreDir`] (an ephemeral store's 4 GiB WRITEMAP file on the
/// non-sparse HFS+ ramdisk frees asynchronously after a plain unlink,
/// and back-to-back rounds outrun it).
struct KillStore {
    path: PathBuf,
    preserve: bool,
}

impl KillStore {
    fn new(kind: StoreKind, tag: &str) -> Self {
        let root =
            std::env::var_os("BUMBLEDB_SCRATCH_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let path = root.join(format!("bumbledb-kill-{}-{kind}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create kill store dir");
        Self {
            path,
            preserve: true,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn delete_on_drop(&mut self) {
        self.preserve = false;
    }
}

impl Drop for KillStore {
    fn drop(&mut self) {
        if self.preserve {
            eprintln!(
                "kill sweep: store preserved for autopsy at {}",
                self.path.display()
            );
            return;
        }
        if let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .open(self.path.join("data.mdb"))
        {
            let _ = file.set_len(0);
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
