//! P12 adversarial integration: REAL process pause/kill/restart schedules
//! over the landed store + history machine — no mocked transports, no
//! emulated schedules. Owned by P12; no subsystem packet owns these
//! cross-boundary arms (P05 routed its REC-04/05/07, PROTO-12-shape and
//! GC-04/12 crash arms here).
//!
//! Arms:
//! - a SIGSTOP-suspended `LocalHistory` owner versus a competing successor
//!   open (REP-005/REP-009/SDK-006 successor property; FS-01/FS-04, RUN-05);
//! - SIGKILL exactly at the hosted publication boundary (head CAS landed,
//!   response never observed, local commit never ran): reopen resolves the
//!   lost publication from durable evidence (PROTO-05/PROTO-12, REP-010,
//!   SDK-001 root, OPS-005);
//! - SIGKILL mid-GC-sweep at the durable-progress CAS: resumed collection
//!   converges, protected closure survives, orphans go (GC-04/GC-07/GC-12,
//!   REP-007/REP-013/REP-019, REC-04 shape);
//! - a hydration hold revoked (named root released) while a hydrate is
//!   mid-flight in another SIGSTOP-suspended process, frozen at a
//!   deterministic first-chunk-GET boundary: the registered hold protects
//!   the closure through a full collection; after the release the resumed
//!   hydrate refuses whole and typed (never a partial or wrong snapshot),
//!   the killed variant adopts nothing, and both successors converge on the
//!   complete current state (REC-05, REP-003, GC-03 shape).
//!
//! Child arms re-exec this binary with a mode environment variable, exactly
//! the repository's `local_ownership.rs` harness pattern. Each test owns an
//! exclusive temporary tree and signals only its own children.
//! Verification: `NotRun` (F2 authors, does not execute).

#![cfg(unix)]

mod lane_support;

use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bumbledb::{Db, RelationId, Value};

use bumbledb_log::admin;
use bumbledb_log::checkpointer::{
    CheckpointKind, CheckpointPolicy, publish_checkpoint, read_live_head,
};
use bumbledb_log::codec::StreamLimits;
use bumbledb_log::gc::{GcPolicy, close_epoch, mark, run_collection};
use bumbledb_log::history::{Condition, TerminalOutcome};
use bumbledb_log::manifest::{GcPhase, RootKind, RootPolicy};
use bumbledb_log::recovery::{self, materialization_path};
use bumbledb_log::store::fs::{FsError, FsStore, Inject, Phase};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion, ListPage, ObjectKind, ObjectRead,
    ObjectRef, PutOutcome, ReceiveLimits, TransportContext, get_verified, put_verified,
};
use bumbledb_log::writer::{HostedHistory, LocalHistory, ResolveOutcome};

use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, theory, work};

const CHILD_ENV: &str = "BDB_P12_PROCESS_CHILD";
const DIR_ENV: &str = "BDB_P12_PROCESS_DIR";
const TENANT_ENV: &str = "BDB_P12_PROCESS_TENANT";
const WAIT: Duration = Duration::from_secs(30);

fn fresh_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "bdb-p12-proc-{}-{name}-{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create test root");
    root
}

fn spawn_child(mode: &str, dir: &Path) -> Child {
    spawn_child_in(mode, dir, None)
}

fn spawn_child_in(mode: &str, dir: &Path, tenant: Option<&str>) -> Child {
    let mut command = ProcessCommand::new(std::env::current_exe().expect("test binary"));
    command
        .args([
            "--exact",
            "child_process_entry",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(CHILD_ENV, mode)
        .env(DIR_ENV, dir)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(tenant) = tenant {
        command.env(TENANT_ENV, tenant);
    }
    command.spawn().expect("spawn child")
}

fn await_marker(child: &mut Child, marker: &str) {
    let stdout = child.stdout.as_mut().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let start = Instant::now();
    let mut line = String::new();
    loop {
        assert!(start.elapsed() < WAIT, "child never printed {marker}");
        line.clear();
        let read = reader.read_line(&mut line).expect("read child line");
        assert!(read > 0, "child stdout closed before {marker}");
        // Suffix match, not equality: libtest prints `test <name> ... `
        // WITHOUT a trailing newline before the test body runs, so the
        // child's FIRST marker glues onto that line. Markers are distinct
        // uppercase tokens the children print alone, never suffixes of one
        // another or of ordinary output.
        if line.trim().ends_with(marker) {
            return;
        }
    }
}

/// Read exactly one line of child stdout (its declared outcome). Only used
/// when the protocol guarantees exactly one further line; a closed pipe is a
/// failed child.
fn next_line(child: &mut Child) -> String {
    let stdout = child.stdout.as_mut().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .expect("read child outcome line");
    assert!(read > 0, "child stdout closed before its outcome line");
    line.trim().to_string()
}

fn signal(child: &Child, name: &str) {
    let status = ProcessCommand::new("kill")
        .args([format!("-{name}"), child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{name} failed");
}

fn park() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

fn try_verified(
    store: &FsStore,
    reference: &ObjectRef,
) -> Result<bumbledb::work::ChargedBytes, bumbledb_log::store::ObjectError> {
    get_verified(
        store,
        "t",
        reference,
        TransportContext::new(&work(), ReceiveLimits::exact(reference.length)),
    )
}

fn scan_user_ids(db: &Db<bumbledb::schema::SchemaDescriptor>) -> Vec<u64> {
    let mut ids = Vec::new();
    db.read(work(), |read| {
        for row in read.scan(RelationId(0))? {
            let row = row?;
            if let Some(Value::U64(id)) = row.first() {
                ids.push(*id);
            }
        }
        Ok(())
    })
    .expect("scan reads");
    ids.sort_unstable();
    ids
}

fn gc_policy() -> GcPolicy {
    GcPolicy {
        head_cap: HEAD_CAP,
        ..GcPolicy::DEFAULT
    }
}

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

/// A delegating store that gates the process at its FIRST checkpoint-chunk
/// GET: the marker is printed, then the verb blocks until the parent writes
/// `GO` on stdin. The landed `FsStore` `Phase` hooks intercept mutations only,
/// so the deterministic MID-HYDRATE read boundary (head + manifest observed,
/// zero chunk bytes collected) is pinned here instead — same marker-park
/// discipline as the Phase-hook arms, resumable so the parent can also drive
/// the survived-suspension variant.
struct ParkAtFirstChunkGet {
    inner: FsStore,
    tripped: AtomicBool,
}

impl ParkAtFirstChunkGet {
    fn new(inner: FsStore) -> Self {
        Self {
            inner,
            tripped: AtomicBool::new(false),
        }
    }
}

impl ConditionalStore for ParkAtFirstChunkGet {
    type Error = FsError;

    fn read_head(&self, head_key: &str) -> Result<HeadRead, FsError> {
        self.inner.read_head(head_key)
    }

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, FsError> {
        self.inner.create_head(head_key, body)
    }

    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, FsError> {
        self.inner.replace_head(head_key, expected, body)
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, FsError> {
        self.inner.put_object(key, body)
    }

    fn get_object(&self, key: &str) -> Result<ObjectRead, FsError> {
        if key.contains("/chunk/") && !self.tripped.swap(true, Ordering::SeqCst) {
            println!("CHUNKPARK");
            let _ = std::io::stdout().flush();
            let mut line = String::new();
            std::io::stdin()
                .lock()
                .read_line(&mut line)
                .expect("parent gate line");
            assert_eq!(line.trim(), "GO", "the parent resumes the gate with GO");
        }
        self.inner.get_object(key)
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, FsError> {
        self.inner.list_objects(prefix, after)
    }

    fn delete_object(&self, key: &str) -> Result<(), FsError> {
        self.inner.delete_object(key)
    }
}

/// The child arms. An ordinary run (no env) is an immediate no-op pass; a
/// re-executed child performs its mode and never returns normally.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one flat child-mode dispatch; each arm is a distinct crash script"
)]
fn child_process_entry() {
    let Ok(mode) = std::env::var(CHILD_ENV) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(DIR_ENV).expect("child dir"));
    match mode.as_str() {
        // A full-stack LocalHistory owner: durable committed decision, then
        // park holding the store's kernel lock.
        "hold-local-history" => {
            let db = Arc::new(
                Db::create(&dir.join("db"), theory(), work())
                    .expect("create store")
                    .expect("empty store admits"),
            );
            let history = LocalHistory::create(
                Arc::clone(&db),
                lane_support::test_identity(&db).database_id,
                lane_support::test_identity(&db).incarnation_id,
                op(0xc3),
                LIMITS,
                &work(),
            )
            .expect("local history creates");
            let identity = history.identity();
            let command = insert_user(history.db(), identity, 0x01, 7);
            match history.submit(&command, &work()) {
                bumbledb_log::writer::SubmitOutcome::Decided { .. } => {}
                other => panic!("seed decision failed: {other:?}"),
            }
            println!("READY");
            let _ = std::io::stdout().flush();
            park();
        }
        // Publish through the hosted machine and park exactly AFTER the head
        // CAS landed (Phase::Published on the HEAD key), before the response
        // is observed and before the local materialization commit.
        "park-after-publish" => {
            let db = Arc::new(
                Db::create(&dir.join("localdb"), theory(), work())
                    .expect("create store")
                    .expect("empty store admits"),
            );
            let store = FsStore::new(dir.join("store"));
            let mut head_publishes = 0u32;
            store.set_hook(move |phase, key| {
                if phase == Phase::Published && key.ends_with("HEAD") {
                    head_publishes += 1;
                    // 1st HEAD publish = genesis create; 2nd = the decision.
                    if head_publishes == 2 {
                        println!("PUBLISHED");
                        let _ = std::io::stdout().flush();
                        park();
                    }
                }
                Inject::Continue
            });
            let identity = lane_support::test_identity(&db);
            let history = HostedHistory::create(
                Arc::clone(&db),
                store,
                "t".to_string(),
                0,
                identity.database_id,
                identity.incarnation_id,
                op(0xc3),
                LIMITS,
                &work(),
            )
            .expect("hosted history creates");
            let command = insert_user(history.db(), history.identity(), 0x01, 7);
            let _ = history.submit(&command, &work());
            unreachable!("the parent kills the published child");
        }
        // Reach GC sweep, then park at the durable-progress head CAS.
        "park-mid-sweep" => {
            let store = FsStore::new(dir.join("store"));
            let mut mirror = Mirror::create("p12-sweep", &store, "t");
            let identity = mirror.identity;
            for request in 1..=3u8 {
                mirror.submit(&insert_user(
                    mirror.db(),
                    identity,
                    request,
                    u64::from(request) * 10,
                ));
            }
            publish_checkpoint(
                mirror.db(),
                &store,
                "t",
                LIMITS,
                CheckpointKind::Ordinary,
                &ckpt_policy(),
                &work(),
            )
            .expect("checkpoint publishes");
            // An unreferenced old-epoch object: the sweep's lawful prey.
            let old_epoch = mirror.head().object_epoch;
            put_verified(&store, "t", old_epoch, ObjectKind::Chunk, b"p12-orphan")
                .expect("orphan stages");
            close_epoch(&store, "t", op(0xee), &gc_policy(), &work()).expect("barrier");
            mark(&store, "t", LIMITS, &gc_policy(), &work()).expect("mark evidence");
            // Park at the FIRST head staging after mark: sweep's progress CAS.
            store.set_hook(|phase, key| {
                if phase == Phase::Staged && key.ends_with("HEAD") {
                    println!("SWEEPCAS");
                    let _ = std::io::stdout().flush();
                    park();
                }
                Inject::Continue
            });
            let _ = bumbledb_log::gc::sweep(&store, "t", &gc_policy(), &work());
            unreachable!("the parent kills the sweeping child");
        }
        // Publish a checkpointed history, then hydrate a fresh tenant cache;
        // the parent kills us while the owned staging directory exists.
        "hydrate-and-park" => {
            let store = FsStore::new(dir.join("store"));
            let mut mirror = Mirror::create("p12-hydrate", &store, "t");
            let identity = mirror.identity;
            // One wide command plus two singles: enough bytes that hydration
            // has a real staging window under a 4 KiB chunk target.
            mirror.submit(&lane_support::command(
                mirror.db(),
                identity,
                0x01,
                Condition::Unconditional,
                |draft| {
                    for id in 1_000u64..1_500 {
                        draft
                            .insert(bumbledb::RelationId(0), &[Value::U64(id)])
                            .expect("insert");
                    }
                },
            ));
            mirror.submit(&insert_user(mirror.db(), identity, 0x02, 10));
            publish_checkpoint(
                mirror.db(),
                &store,
                "t",
                LIMITS,
                CheckpointKind::Ordinary,
                &ckpt_policy(),
                &work(),
            )
            .expect("checkpoint publishes");
            mirror.submit(&insert_user(mirror.db(), identity, 0x03, 20));
            println!("HYDRATING");
            let _ = std::io::stdout().flush();
            let recovered = recovery::open_hosted(
                &dir.join("tenant"),
                theory(),
                &store,
                "test-origin",
                "t",
                LIMITS,
                StreamLimits::DEFAULT,
                HEAD_CAP,
                &work(),
            )
            .expect("child hydration");
            let _hold = recovered;
            println!("DONE");
            let _ = std::io::stdout().flush();
            park();
        }
        // Hydrate a fresh tenant cache (named by TENANT_ENV) from the head's
        // pinned closure and freeze at the FIRST chunk GET — a deterministic
        // mid-hydrate boundary the parent SIGSTOPs, revokes the hydration
        // hold around, and then resumes or kills. On resume the only lawful
        // outcomes are a WHOLE typed refusal (nothing adopted) or a COMPLETE
        // closure at the head this open identified — never a partial or
        // mixed snapshot.
        "hydrate-hold-revoked" => {
            let tenant = std::env::var(TENANT_ENV).expect("child tenant name");
            let store = ParkAtFirstChunkGet::new(FsStore::new(dir.join("store")));
            if let Ok(recovered) = recovery::open_hosted(
                &dir.join(tenant),
                theory(),
                &store,
                "test-origin",
                "t",
                LIMITS,
                StreamLimits::DEFAULT,
                HEAD_CAP,
                &work(),
            ) {
                // Complete AT THE IDENTIFIED HEAD means exactly the
                // pre-park state: 500 wide rows plus ids 10 and 20 —
                // never the post-park tail, never a partial import.
                let ids = scan_user_ids(&recovered.db);
                let complete = ids.len() == 502
                    && ids.contains(&10)
                    && ids.contains(&20)
                    && ids.contains(&1_000)
                    && ids.contains(&1_499);
                drop(recovered);
                println!(
                    "{}",
                    if complete {
                        "HYDRATED-COMPLETE"
                    } else {
                        "HYDRATED-WRONG"
                    }
                );
                let _ = std::io::stdout().flush();
            } else {
                // A typed whole refusal: no snapshot was returned.
                println!("REFUSED");
                let _ = std::io::stdout().flush();
            }
            std::process::exit(0);
        }
        other => panic!("unknown child mode {other}"),
    }
}

/// REP-005/REP-009/SDK-006 successor property over the FULL landed stack: a
/// merely suspended `LocalHistory` owner remains the exclusive owner; a
/// competing open refuses and mutates nothing; only real process death
/// releases the store, after which the successor resolves the retained
/// receipt and reads the committed facts (FS-01/FS-04, RUN-05, PROTO-02).
#[test]
fn a_suspended_history_owner_fences_successors_until_real_death() {
    let root = fresh_root("suspend");
    let mut child = spawn_child("hold-local-history", &root);
    await_marker(&mut child, "READY");

    // Refused while live, refused while merely SIGSTOPped: no lease expiry
    // mints ownership from wall-clock time.
    assert!(
        Db::open(&root.join("db"), theory(), work()).is_err(),
        "live owner excludes"
    );
    signal(&child, "STOP");
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        Db::open(&root.join("db"), theory(), work()).is_err(),
        "a paused owner remains owner; time mints nothing"
    );

    // Real death releases; the successor sees exactly the durable history.
    signal(&child, "CONT");
    child.kill().expect("kill");
    let _ = child.wait().expect("reap");
    let start = Instant::now();
    let db = loop {
        match Db::open(&root.join("db"), theory(), work()) {
            Ok(db) => break Arc::new(db),
            Err(error) => {
                assert!(start.elapsed() < WAIT, "death releases ownership: {error}");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let history = LocalHistory::open(Arc::clone(&db), LIMITS).expect("successor opens");
    assert_eq!(
        scan_user_ids(&db),
        vec![7],
        "the committed fact survived the kill"
    );
    let command = insert_user(&db, history.identity(), 0x01, 7);
    match history.resolve(command.command_ref(), &work()) {
        Ok(ResolveOutcome::Found(receipt)) => {
            assert!(
                matches!(receipt.outcome, TerminalOutcome::Committed { .. }),
                "the retained receipt keeps its exact terminal outcome"
            );
        }
        other => panic!("retained receipt resolves after real death: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// PROTO-05/PROTO-12 shape with a REAL kill: the head CAS landed, the owner
/// died before observing the response or committing locally. The lost
/// publication is not lost history — reopening catches up from the durable
/// decision object and the retained ref resolves to the exact recorded
/// outcome (REP-010, SDK-001 root cause, OPS-005 missing-history-is-never-
/// empty).
#[test]
fn a_kill_at_the_publication_boundary_leaves_a_resolvable_decision() {
    let root = fresh_root("pubkill");
    let mut child = spawn_child("park-after-publish", &root);
    await_marker(&mut child, "PUBLISHED");
    child.kill().expect("kill published child");
    let _ = child.wait().expect("reap");

    // Reopen the SAME local materialization (its LMDB never saw the commit)
    // over the SAME durable backend directory.
    let start = Instant::now();
    let db = loop {
        match Db::open(&root.join("localdb"), theory(), work()) {
            Ok(db) => break Arc::new(db),
            Err(error) => {
                assert!(
                    start.elapsed() < WAIT,
                    "death releases the local store: {error}"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let store = FsStore::new(root.join("store"));
    let history = HostedHistory::open(db, store, "t".to_string(), LIMITS, &work())
        .expect("reopen catches up to the published tip");
    // The identical sealed command re-derives the identical ref.
    let command = insert_user(history.db(), history.identity(), 0x01, 7);
    match history.resolve(command.command_ref(), &work()) {
        Ok(ResolveOutcome::Found(receipt)) => {
            assert!(
                matches!(receipt.outcome, TerminalOutcome::Committed { .. }),
                "the published decision is Committed, never downgraded"
            );
        }
        other => panic!("the lost publication resolves after reopen: {other:?}"),
    }
    assert_eq!(
        scan_user_ids(history.db()),
        vec![7],
        "catch-up materialized the published facts into the reopened cache"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// GC-04/GC-07/GC-12 crash arm with a REAL kill at the sweep's durable
/// progress CAS: the resumed collection converges under the same operation,
/// the protected closure (checkpoint + tail decisions) survives byte-exact,
/// the orphan is gone, and the head returns to Idle (REP-007/REP-013/
/// REP-019).
#[test]
fn a_kill_mid_sweep_resumes_to_a_converged_collection() {
    let root = fresh_root("gckill");
    let mut child = spawn_child("park-mid-sweep", &root);
    await_marker(&mut child, "SWEEPCAS");
    child.kill().expect("kill sweeping child");
    let _ = child.wait().expect("reap");

    let store = FsStore::new(root.join("store"));
    // Resume under the SAME operation id the child's barrier recorded.
    let start = Instant::now();
    let report = loop {
        match run_collection(&store, "t", op(0xee), LIMITS, &gc_policy(), &work()) {
            Ok(report) => break report,
            Err(error) => {
                // The killed child's mutation lock releases with its death.
                assert!(start.elapsed() < WAIT, "collection resumes: {error:?}");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    assert!(report.finished, "the resumed sweep converges");
    let (head, _) = read_live_head(&store, "t", HEAD_CAP).expect("head reads");
    assert!(
        matches!(head.gc, GcPhase::Idle),
        "collection state returns to Idle"
    );
    let recovery = head.recovery.expect("the recovery root survived the kill");
    let checkpoint = recovery.checkpoint.expect("checkpoint object reference");
    drop(
        try_verified(&store, &checkpoint)
            .expect("the protected checkpoint manifest survives byte-exact")
            .into_owner(),
    );
    // The orphan was staged under the closed epoch and never referenced:
    // no listing of the objects namespace may still contain its bytes.
    let orphan_ref =
        bumbledb_log::store::ObjectRef::of(recovery.epoch_floor, ObjectKind::Chunk, b"p12-orphan");
    assert!(
        try_verified(&store, &orphan_ref).is_err(),
        "the unreferenced old-epoch orphan was collected"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// REC-04 process arm with a REAL kill: hydration into a fresh tenant cache
/// is killed while its owned staging directory exists (or immediately after
/// completion — the safety assertions hold under either race outcome). The
/// killed staging is never adopted; the resumed open hydrates/verifies a
/// complete materialization with the exact published facts; leftover staging
/// is owned scratch the successor cleans under the kernel lock
/// (REP-009/REP-017, STORE-08 neighbor, G10).
#[test]
fn a_kill_during_hydration_is_never_adopted_and_the_resume_completes() {
    let root = fresh_root("hydratekill");
    let mut child = spawn_child("hydrate-and-park", &root);
    await_marker(&mut child, "HYDRATING");
    // Kill as soon as an owned staging directory appears (mid-hydration), or
    // after a bounded delay if the small fixture already finished.
    let tenant = root.join("tenant");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        let staging_exists = std::fs::read_dir(&tenant).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with("staging-"))
        });
        if staging_exists {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    child.kill().expect("kill hydrating child");
    let _ = child.wait().expect("reap");

    let store = FsStore::new(root.join("store"));
    let start = Instant::now();
    let recovered = loop {
        match recovery::open_hosted(
            &tenant,
            theory(),
            &store,
            "test-origin",
            "t",
            LIMITS,
            StreamLimits::DEFAULT,
            HEAD_CAP,
            &work(),
        ) {
            Ok(recovered) => break recovered,
            Err(error) => {
                // The killed child's directory lock releases with its death.
                assert!(start.elapsed() < WAIT, "hydration resumes: {error:?}");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    // The complete published state, exactly: 500 wide rows + two singles.
    let ids = scan_user_ids(&recovered.db);
    assert_eq!(
        ids.len(),
        502,
        "the resumed hydration is complete, never partial"
    );
    assert!(ids.contains(&10) && ids.contains(&20) && ids.contains(&1_000) && ids.contains(&1_499));
    // The ready materialization is the one adopted path; no killed staging
    // directory was renamed into it.
    assert!(materialization_path(&tenant).exists());
    drop(recovered);
    let _ = std::fs::remove_dir_all(&root);
}

/// Successor open over a tenant directory whose previous owner just exited
/// or was killed: retry until the kernel lock releases with death, then
/// return the hydrated facts of the adopted COMPLETE materialization.
fn open_converged(tenant: &Path, store: &FsStore) -> Vec<u64> {
    let start = Instant::now();
    let recovered = loop {
        match recovery::open_hosted(
            tenant,
            theory(),
            store,
            "test-origin",
            "t",
            LIMITS,
            StreamLimits::DEFAULT,
            HEAD_CAP,
            &work(),
        ) {
            Ok(recovered) => break recovered,
            Err(error) => {
                assert!(
                    start.elapsed() < WAIT,
                    "the successor open succeeds: {error:?}"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    assert!(
        materialization_path(tenant).exists(),
        "the successor adopted a complete materialization"
    );
    let ids = scan_user_ids(&recovered.db);
    drop(recovered);
    ids
}

/// REC-05 process arm (verification-manifest §8 debt 2, closed here): a
/// hydration hold — the named root pinning the closure a hydrate reads — is
/// revoked while that hydrate is mid-flight in another process, frozen at a
/// deterministic first-chunk-GET boundary and SIGSTOP-suspended. The
/// revocation contract holds in both arms: while the hold is REGISTERED, a
/// full collection cannot touch the mid-flight hydrate's closure; once the
/// hold is released and a later collection reclaims the closure, the resumed
/// hydrate refuses WHOLE and typed — no incomplete or wrong snapshot is ever
/// returned or adopted. The killed variant adopts nothing either, and both
/// tenants' successor opens converge on the identical complete current state
/// (REP-003 retained-roots neighbor, GC-03 pin/release shape, REC-04's
/// staging-invisibility carried across the revocation).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one adversarial scenario told end to end; splitting would hide \
              the revocation ordering under test"
)]
fn a_hold_revoked_mid_hydrate_refuses_whole_and_variants_converge() {
    let root = fresh_root("holdrevoke");
    let store = FsStore::new(root.join("store"));
    let mut mirror = Mirror::create("p12-holdrevoke", &store, "t");
    let identity = mirror.identity;
    // 500 wide rows plus two singles, then the first checkpoint: enough
    // chunk traffic under the 4 KiB chunk target that "first chunk GET" is a
    // real mid-hydrate boundary with unread bytes behind it.
    mirror.submit(&lane_support::command(
        mirror.db(),
        identity,
        0x01,
        Condition::Unconditional,
        |draft| {
            for id in 1_000u64..1_500 {
                draft
                    .insert(RelationId(0), &[Value::U64(id)])
                    .expect("insert");
            }
        },
    ));
    mirror.submit(&insert_user(mirror.db(), identity, 0x02, 10));
    mirror.submit(&insert_user(mirror.db(), identity, 0x03, 20));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("first checkpoint publishes");
    // Register the hydration hold against the exact current closure.
    let held = admin::add_named_root_hosted(
        &store,
        "t",
        op(0x71),
        RootKind::HydrationHold,
        "mid-hydrate-hold",
        op(0x72),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("hydration hold registers");
    assert_eq!(held.kind, RootKind::HydrationHold);
    assert_eq!(
        Some(held.recovery),
        mirror.head().recovery,
        "the hold pins the exact current recovery closure"
    );
    let pinned_manifest = held
        .recovery
        .checkpoint
        .expect("pinned checkpoint reference");
    let charged = try_verified(&store, &pinned_manifest).expect("pinned manifest reads");
    let pinned = bumbledb_log::codec::decode_manifest(charged.as_bytes(), ckpt_policy().stream)
        .expect("pinned manifest decodes");
    drop(charged.into_owner());
    assert!(
        !pinned.chunks.is_empty(),
        "the pinned closure has chunk objects to revoke"
    );

    // Two children hydrate fresh tenant caches from that closure and freeze
    // at their first chunk GET; SIGSTOP makes each a genuinely suspended
    // process, exactly the suspended-owner arm's shape.
    let mut resumed = spawn_child_in("hydrate-hold-revoked", &root, Some("tenant-a"));
    await_marker(&mut resumed, "CHUNKPARK");
    let mut killed = spawn_child_in("hydrate-hold-revoked", &root, Some("tenant-b"));
    await_marker(&mut killed, "CHUNKPARK");
    signal(&resumed, "STOP");
    signal(&killed, "STOP");

    // The head moves past the pinned closure while both hydrates are frozen:
    // after the second checkpoint, ONLY the hold protects what they read.
    mirror.submit(&insert_user(mirror.db(), identity, 0x04, 30));
    mirror.submit(&insert_user(mirror.db(), identity, 0x05, 40));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("second checkpoint publishes");

    // The refusing arm of the revocation contract: with the hold registered,
    // a full collection converges WITHOUT touching the mid-flight hydrate's
    // closure.
    let protected = run_collection(&store, "t", op(0x73), LIMITS, &gc_policy(), &work())
        .expect("collection with the hold registered");
    assert!(protected.finished);
    drop(
        try_verified(&store, &pinned_manifest)
            .expect("the held manifest survives a full collection")
            .into_owner(),
    );
    for chunk in &pinned.chunks {
        drop(
            try_verified(&store, chunk)
                .expect("every held chunk survives a full collection")
                .into_owner(),
        );
    }

    // The revocation: release the hold (the report names the exact lost
    // recovery capability), then a later collection reclaims the closure out
    // from under both frozen hydrates.
    let released =
        admin::release_named_root_hosted(&store, "t", op(0x71), false, HEAD_CAP, &work())
            .expect("release runs")
            .expect("the hold existed");
    assert_eq!(
        released.recovery.checkpoint,
        Some(pinned_manifest),
        "the release reports the exact revoked closure"
    );
    let reclaimed = run_collection(&store, "t", op(0x74), LIMITS, &gc_policy(), &work())
        .expect("collection after the release");
    assert!(reclaimed.finished);
    assert!(
        try_verified(&store, &pinned_manifest).is_err(),
        "the revoked closure's manifest is collected"
    );
    for chunk in &pinned.chunks {
        assert!(
            try_verified(&store, chunk).is_err(),
            "the revoked closure's chunks are collected"
        );
    }
    // Revocation never touches the live root: the CURRENT closure verifies.
    let current = mirror
        .head()
        .recovery
        .expect("current recovery root")
        .checkpoint
        .expect("current checkpoint");
    drop(
        try_verified(&store, &current)
            .expect("the current checkpoint is retained")
            .into_owner(),
    );

    // Killed variant: real death while frozen mid-hydrate. Nothing was
    // adopted; the successor hydrates the CURRENT complete closure.
    killed.kill().expect("kill the frozen hydrating child");
    let _ = killed.wait().expect("reap killed child");
    let tenant_killed = root.join("tenant-b");
    assert!(
        !materialization_path(&tenant_killed).exists(),
        "a killed mid-hydrate never adopted a materialization"
    );
    let ids_killed = open_converged(&tenant_killed, &store);

    // Resumed variant: the frozen hydrate resumes into the collected
    // closure. The child prints HYDRATED-COMPLETE only for a byte-complete
    // closure at ITS identified head (the one other lawful outcome) and
    // HYDRATED-WRONG for any partial/mixed snapshot; with the closure
    // deterministically collected before the gate opens, the outcome here is
    // the whole typed refusal.
    resumed
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(b"GO\n")
        .expect("gate opens");
    let _ = resumed.stdin.as_mut().expect("child stdin").flush();
    signal(&resumed, "CONT");
    let outcome = next_line(&mut resumed);
    assert_eq!(
        outcome, "REFUSED",
        "a hydrate whose closure was revoked refuses whole"
    );
    let _ = resumed.wait().expect("reap resumed child");
    let tenant_resumed = root.join("tenant-a");
    assert!(
        !materialization_path(&tenant_resumed).exists(),
        "the refusal adopted nothing"
    );
    let ids_resumed = open_converged(&tenant_resumed, &store);

    // Convergence: both variants land on the identical complete current
    // state — the mirror's own committed facts, tail included.
    assert_eq!(
        ids_resumed, ids_killed,
        "resumed and killed variants converge"
    );
    assert_eq!(
        ids_resumed,
        scan_user_ids(mirror.db()),
        "both successors converge on the complete current state"
    );
    assert_eq!(
        ids_resumed.len(),
        504,
        "500 wide rows plus ids 10/20/30/40, nothing partial"
    );
    assert!(ids_resumed.contains(&30) && ids_resumed.contains(&40));
    let _ = std::fs::remove_dir_all(&root);
}
