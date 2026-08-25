//! Conformance lane 4: the crash matrices, protocol steps as data.
//! Every proper prefix of the writer's step enum is executed, killed,
//! and recovered under both ack modes; the replica's two-step apply
//! discipline gets the same treatment, executed mechanically. Recovery
//! is nothing but pending resolution, the ordinary catch-up loop, and
//! the wholeness identity — the expectations below are exhaustive
//! matches over the step enums, so a forgotten crash case is a missing
//! enum arm. The double-apply suite runs L10 as an executable oracle:
//! every batch of a generated history applied twice at every prefix,
//! with digest, generation, and vector unmoved.

mod lane_e_support;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bumbledb::{Admission, Db, SchemaDescriptor, Value, obs};
use bumbledb_log::apply::{Applied, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Op, OpKind};
use bumbledb_log::manifest::{Manifest, create_manifest, log_key};
use bumbledb_log::replica::{Opened, Provenance, Replica};
use bumbledb_log::sidecar::{Chain, ChainEntry, SidecarRead};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{
    AckMode, Commit, Durability, Error, Options, StepControl, StepHook, Writer, WriterOpened,
    WriterStep,
};
use lane_e_support::{
    BOOKING, CrashOnce, NOTE, RECIPE, STEP, TestLog, VENUE, codec, insert, kitchen_braid,
    note_braid, note_row, recipe_row, step_row, temp_dir, theory, venue_braid,
};

type FsWriter<H> = Writer<SchemaDescriptor, FsStore, H>;

const WRITER_STEPS: [WriterStep; 7] = [
    WriterStep::Encode,
    WriterStep::PendingWrite,
    WriterStep::ApplyLocal,
    WriterStep::AckLocal,
    WriterStep::PutLog,
    WriterStep::ChainAdvance,
    WriterStep::PendingClear,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Published,
    Local,
}

fn options(mode: Mode, writer_id: u64) -> Options {
    let mut options = Options::new(writer_id);
    if mode == Mode::Local {
        options.ack = AckMode::Local;
    }
    options
}

/// Records every observed step and never crashes — the instrument that
/// pins one-pass convergence: a reopen after recovery observes nothing.
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<WriterStep>>>);

impl Recorder {
    fn steps(&self) -> Vec<WriterStep> {
        self.0.lock().expect("recorder lock").clone()
    }
}

impl StepHook for Recorder {
    fn observe(&self, step: WriterStep) -> StepControl {
        self.0.lock().expect("recorder lock").push(step);
        StepControl::Continue
    }
}

fn open_with<H: StepHook + 'static>(
    root: PathBuf,
    dir: &Path,
    opts: Options,
    hook: H,
) -> FsWriter<H> {
    match Writer::open_hooked(FsStore::new(root), "", dir, theory(), opts, hook)
        .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn open_replica(root: &Path, dir: &Path) -> Replica<SchemaDescriptor, FsStore> {
    match Replica::open(FsStore::new(root.to_path_buf()), "", dir, theory()).expect("open replica")
    {
        Opened::Ready(replica) => *replica,
        Opened::Refused(refusal) => panic!("replica refused: {refusal:?}"),
    }
}

fn create_db(dir: &Path) -> Db<SchemaDescriptor> {
    match Db::create(dir, theory()).expect("create store") {
        Admission::Accepted(db) => db,
        Admission::Rejected(violations) => panic!("theory rejected: {violations:?}"),
    }
}

fn noop_committed(events: &[obs::TraceEvent]) -> bool {
    events
        .iter()
        .any(|event| event.point() == obs::names::COMMIT_NOOP)
}

/// Whether the pending slot survives a crash after `step` — exactly the
/// cases whose recovery re-application must land in the engine's no-op
/// arm.
fn pending_survives(mode: Mode, step: WriterStep) -> bool {
    match step {
        WriterStep::Encode | WriterStep::PendingClear => false,
        WriterStep::PendingWrite
        | WriterStep::ApplyLocal
        | WriterStep::PutLog
        | WriterStep::ChainAdvance => true,
        WriterStep::AckLocal => mode == Mode::Local,
    }
}

/// Whether the accepted batch reaches the log by the end of recovery:
/// everything durable past the pending fsync is judged accepted at
/// recovery and published; only the pre-pending crash leaves nothing.
fn lands(step: WriterStep) -> bool {
    match step {
        WriterStep::Encode => false,
        WriterStep::PendingWrite
        | WriterStep::ApplyLocal
        | WriterStep::AckLocal
        | WriterStep::PutLog
        | WriterStep::ChainAdvance
        | WriterStep::PendingClear => true,
    }
}

/// Whether the host's commit call resolves `Accepted` despite the
/// crash. Under `ack = published` the `AckLocal` step never fires, so
/// arming it proves the parenthetical; under `ack = local` every step
/// past the ack crashes on the detached publisher after the host holds
/// its answer.
fn acked(mode: Mode, step: WriterStep) -> bool {
    match step {
        WriterStep::Encode | WriterStep::PendingWrite | WriterStep::ApplyLocal => false,
        WriterStep::AckLocal => true,
        WriterStep::PutLog | WriterStep::ChainAdvance | WriterStep::PendingClear => {
            mode == Mode::Local
        }
    }
}

#[allow(clippy::too_many_lines)]
fn crash_case(mode: Mode, step: WriterStep) {
    let root = temp_dir(&format!("f4_{mode:?}_{step:?}"));
    let dir = root.join("w");
    let writer = open_with(
        root.clone(),
        &dir,
        options(mode, 41),
        CrashOnce::new(step, 0),
    );
    let result = writer.commit(|batch| {
        batch.insert(NOTE, [note_row(7, "matrix")]);
        Ok(())
    });
    if acked(mode, step) {
        match result.expect("acked commit resolves") {
            Commit::Accepted {
                generation,
                durability,
                ..
            } => {
                assert_eq!(generation, 1, "{mode:?}/{step:?}");
                let expected = match mode {
                    Mode::Published => Durability::Published,
                    Mode::Local => Durability::LocalPending,
                };
                assert_eq!(durability, expected, "{mode:?}/{step:?}");
            }
            Commit::Rejected(violations) => panic!("{mode:?}/{step:?} rejected: {violations:?}"),
        }
    } else {
        let err = result.expect_err("the crash surfaces to the caller");
        assert!(
            matches!(err, Error::InjectedCrash { step: got } if got == step),
            "{mode:?}/{step:?}: {err:?}"
        );
    }
    writer.quiesce();
    drop(writer);

    // Recovery is an ordinary open: pending resolution + catch-up + the
    // wholeness identity, nothing else — in place, never a discard. For
    // the mid-publish cells the marker also proves the byte-equal
    // absorption arm ran: the occupant compared equal to the pending
    // bytes and the store survived where a loss would have deleted it.
    std::fs::write(dir.join("marker"), b"in-place").expect("plant marker");
    obs::start_capture();
    let recovered = open_with(root.clone(), &dir, options(mode, 41), Recorder::default());
    let events = obs::finish_capture();
    assert_eq!(
        recovered.backlog(),
        None,
        "{mode:?}/{step:?}: recovery leaves no backlog"
    );
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(
        recovered.vector()[&braid],
        u64::from(lands(step)),
        "{mode:?}/{step:?}"
    );
    let store = FsStore::new(root.clone());
    let slot = store.get(&log_key("", braid, 1)).expect("get slot 1");
    assert_eq!(
        slot.is_some(),
        lands(step),
        "{mode:?}/{step:?}: the batch reaches the log iff its pending survived"
    );
    if let Some(fetched) = &slot {
        let batch = codec.decode(&fetched.bytes).expect("decode slot 1");
        assert_eq!(batch.header.writer, 41, "{mode:?}/{step:?}: our own slot");
    }
    assert!(
        store
            .get(&log_key("", braid, 2))
            .expect("get slot 2")
            .is_none(),
        "{mode:?}/{step:?}: recovery never double-publishes"
    );
    let row_present = recovered.with_db(|db| {
        db.read(|instance| instance.contains_dyn(NOTE, &note_row(7, "matrix")))
            .expect("read")
    });
    assert_eq!(row_present, lands(step), "{mode:?}/{step:?}");
    if acked(mode, step) {
        assert!(
            row_present && slot.is_some(),
            "{mode:?}/{step:?}: no acked commit is ever lost"
        );
    }
    let sum: u64 = recovered.vector().values().sum();
    let generation = recovered.with_db(|db| db.generation().expect("generation").value());
    assert_eq!(
        generation, sum,
        "{mode:?}/{step:?}: the wholeness identity is exact after recovery"
    );
    if pending_survives(mode, step) {
        assert!(
            noop_committed(&events),
            "{mode:?}/{step:?}: the crash-window re-application lands in the engine no-op arm"
        );
    }
    assert!(
        dir.join("marker").exists(),
        "{mode:?}/{step:?}: recovery resolved in place — the absorption arm, never a discard"
    );
    let digest = recovered.with_db(|db| db.catalog_digest().expect("digest"));
    drop(recovered);

    // One pass: a further reopen observes zero writer steps and moves
    // nothing.
    let second = Recorder::default();
    let settled = open_with(root.clone(), &dir, options(mode, 41), second.clone());
    assert!(
        second.steps().is_empty(),
        "{mode:?}/{step:?}: recovery converged in one pass"
    );
    assert_eq!(
        settled.with_db(|db| db.catalog_digest().expect("digest")),
        digest,
        "{mode:?}/{step:?}"
    );
    drop(settled);

    // The whole log replays clean on a fresh replica: apply's own
    // battery is the oracle that no rejected batch, no net-no-op slot,
    // and no phantom reached the bucket.
    let replica = open_replica(&root, &root.join("probe"));
    assert!(replica.wedged().is_empty(), "{mode:?}/{step:?}");
    assert_eq!(
        replica.db().catalog_digest().expect("digest"),
        digest,
        "{mode:?}/{step:?}: the log's replay agrees with the recovered writer"
    );
}

#[test]
fn writer_crash_matrix_published_mode() {
    for step in WRITER_STEPS {
        crash_case(Mode::Published, step);
    }
}

#[test]
fn writer_crash_matrix_local_ack_mode() {
    for step in WRITER_STEPS {
        crash_case(Mode::Local, step);
    }
}

/// The loss path's re-persist window: the crash lands exactly at the
/// carried pending's re-persist into the fresh sidecar — the second
/// `PendingWrite` of one commit, after the loss's discard and re-open
/// and before the re-judgment. Recovery is the ordinary pending
/// resolution: the carried batch re-judges at the tip with one race,
/// and the commit lands exactly once.
#[test]
fn crash_at_the_loss_re_persist_recovers_through_pending_resolution() {
    let root = temp_dir("f4_repersist");
    let dir = root.join("w");
    let writer = open_with(
        root.clone(),
        &dir,
        options(Mode::Published, 41),
        CrashOnce::new(WriterStep::PendingWrite, 1),
    );
    let codec = codec();
    let braid = note_braid(&codec);
    let mut log = TestLog::attach(root.clone(), "");
    log.publish(braid, &[insert(NOTE, note_row(50, "competitor"))], 5);

    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(51, "carried")]);
            Ok(())
        })
        .expect_err("the crash lands at the re-persist");
    assert!(matches!(
        err,
        Error::InjectedCrash {
            step: WriterStep::PendingWrite
        }
    ));
    writer.quiesce();
    drop(writer);

    let recovered = open_with(
        root.clone(),
        &dir,
        options(Mode::Published, 41),
        Recorder::default(),
    );
    assert_eq!(recovered.backlog(), None, "resolved at open");
    assert_eq!(recovered.vector()[&braid], 2, "the tip plus our slot");
    let store = FsStore::new(root);
    let slot2 = store
        .get(&log_key("", braid, 2))
        .expect("get")
        .expect("the carried commit landed at tip+1");
    let batch = codec.decode(&slot2.bytes).expect("decode");
    assert_eq!(batch.header.writer, 41, "our own slot, exactly once");
    assert!(
        store.get(&log_key("", braid, 3)).expect("get").is_none(),
        "never twice"
    );
    let sum: u64 = recovered.vector().values().sum();
    let generation = recovered.with_db(|db| db.generation().expect("generation").value());
    assert_eq!(generation, sum, "whole after the windowed crash");
    recovered.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(50, "competitor"))?);
            assert!(instance.contains_dyn(NOTE, &note_row(51, "carried"))?);
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn resurrected_unjudged_batch_rejects_at_recovery_and_never_reaches_the_log() {
    for mode in [Mode::Published, Mode::Local] {
        let root = temp_dir(&format!("f4_resurrect_{mode:?}"));
        let dir = root.join("w");
        let writer = open_with(
            root.clone(),
            &dir,
            options(mode, 41),
            CrashOnce::new(WriterStep::PendingWrite, 0),
        );
        // A step without its recipe: containment rejects it — but the
        // crash lands before the first judgment, so the batch is
        // resurrected unjudged and sentenced at recovery.
        let err = writer
            .commit(|batch| {
                batch.insert(STEP, [step_row(9, "orphan")]);
                Ok(())
            })
            .expect_err("crash injected");
        assert!(matches!(
            err,
            Error::InjectedCrash {
                step: WriterStep::PendingWrite
            }
        ));
        writer.quiesce();
        drop(writer);

        let recovered = open_with(root.clone(), &dir, options(mode, 41), Recorder::default());
        assert_eq!(recovered.backlog(), None, "{mode:?}: rejection cleared");
        let codec = codec();
        let braid = kitchen_braid(&codec);
        assert_eq!(recovered.vector()[&braid], 0, "{mode:?}");
        let store = FsStore::new(root);
        assert!(
            store.get(&log_key("", braid, 1)).expect("get").is_none(),
            "{mode:?}: a born-rejected batch never reaches the log"
        );
        let present = recovered.with_db(|db| {
            db.read(|instance| instance.contains_dyn(STEP, &step_row(9, "orphan")))
                .expect("read")
        });
        assert!(!present, "{mode:?}");
        let generation = recovered.with_db(|db| db.generation().expect("generation").value());
        assert_eq!(generation, 0, "{mode:?}: nothing was owed, nothing moved");
    }
}

#[test]
fn born_noop_batch_clears_at_the_exact_vector_sum_and_never_reaches_the_log() {
    // Both crash shapes of the born-no-op arm: before the duplicate's
    // judgment (resurrected, judged no-op at recovery) and after its
    // no-op verdict with the pending clear lost. The allowance skips
    // the base commit's own observations of the step — under local
    // acks the detached publisher re-applies, so the base fires
    // `ApplyLocal` twice.
    for (mode, step, allow) in [
        (Mode::Published, WriterStep::PendingWrite, 1),
        (Mode::Published, WriterStep::ApplyLocal, 1),
        (Mode::Local, WriterStep::PendingWrite, 1),
        (Mode::Local, WriterStep::ApplyLocal, 2),
    ] {
        let root = temp_dir(&format!("f4_noop_{mode:?}_{step:?}"));
        let dir = root.join("w");
        let writer = open_with(
            root.clone(),
            &dir,
            options(mode, 41),
            CrashOnce::new(step, allow),
        );
        match writer
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "first")]);
                Ok(())
            })
            .expect("base commit")
        {
            Commit::Accepted { generation: 1, .. } => {}
            other => panic!("{mode:?}/{step:?}: base commit went {other:?}"),
        }
        writer.quiesce();
        let err = writer
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "first")]);
                Ok(())
            })
            .expect_err("crash injected on the duplicate");
        assert!(matches!(err, Error::InjectedCrash { step: got } if got == step));
        writer.quiesce();
        drop(writer);

        obs::start_capture();
        let recovered = open_with(root.clone(), &dir, options(mode, 41), Recorder::default());
        let events = obs::finish_capture();
        assert_eq!(recovered.backlog(), None, "{mode:?}/{step:?}");
        let codec = codec();
        let braid = note_braid(&codec);
        assert_eq!(recovered.vector()[&braid], 1, "{mode:?}/{step:?}");
        let store = FsStore::new(root);
        assert!(
            store.get(&log_key("", braid, 2)).expect("get").is_none(),
            "{mode:?}/{step:?}: a born no-op publishes nothing"
        );
        let generation = recovered.with_db(|db| db.generation().expect("generation").value());
        assert_eq!(
            generation, 1,
            "{mode:?}/{step:?}: cleared at the exact vector sum"
        );
        assert!(
            noop_committed(&events),
            "{mode:?}/{step:?}: the recovery re-application is the engine no-op arm"
        );
    }
}

#[test]
fn a_phantom_writer_commit_discards_with_the_directory_never_silently_divergent() {
    let root = temp_dir("f4_phantom");
    let dir = root.join("w");
    let writer = open_with(
        root.clone(),
        &dir,
        options(Mode::Published, 41),
        Recorder::default(),
    );
    match writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "acked")]);
            Ok(())
        })
        .expect("commit")
    {
        Commit::Accepted { generation: 1, .. } => {}
        other => panic!("commit went {other:?}"),
    }
    writer.quiesce();
    drop(writer);

    // A local commit the log never assigned and no pending accounts
    // for: torn by definition, and the disposable law answers.
    let db: Db<SchemaDescriptor> = Db::open(&dir, theory()).expect("raw open");
    db.write(|tx| {
        tx.insert_dyn(NOTE, [[Value::U64(99), Value::String("phantom".into())]])?;
        Ok(())
    })
    .expect("write")
    .expect("accepted");
    drop(db);
    std::fs::write(dir.join("marker"), b"pre-discard").expect("plant marker");

    let recovered = open_with(
        root,
        &dir,
        options(Mode::Published, 41),
        Recorder::default(),
    );
    assert!(
        !dir.join("marker").exists(),
        "the torn directory was discarded and rebuilt from the log"
    );
    assert_eq!(recovered.backlog(), None);
    let phantom = recovered.with_db(|db| {
        db.read(|instance| {
            instance.contains_dyn(NOTE, &[Value::U64(99), Value::String("phantom".into())])
        })
        .expect("read")
    });
    assert!(!phantom, "the phantom died with the directory");
    let acked_row = recovered.with_db(|db| {
        db.read(|instance| instance.contains_dyn(NOTE, &note_row(1, "acked")))
            .expect("read")
    });
    assert!(acked_row, "the acked commit survived through its slot");
    let sum: u64 = recovered.vector().values().sum();
    let generation = recovered.with_db(|db| db.generation().expect("generation").value());
    assert_eq!(generation, sum);
}

/// The replica's apply discipline, reified for the matrix: engine
/// commit, then sidecar advance (50's two steps, in order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplicaStep {
    ApplyLocal,
    ChainAdvance,
}

const REPLICA_STEPS: [ReplicaStep; 2] = [ReplicaStep::ApplyLocal, ReplicaStep::ChainAdvance];

#[test]
#[allow(clippy::too_many_lines)]
fn replica_crash_matrix_every_prefix_recovers_through_catch_up_alone() {
    for len in 0..=REPLICA_STEPS.len() {
        let root = temp_dir(&format!("f4_replica_{len}"));
        let store = FsStore::new(root.clone());
        let codec = codec();
        create_manifest(
            &store,
            "",
            &Manifest {
                fingerprint: *codec.fingerprint(),
                checkpoint: None,
            },
        )
        .expect("create manifest");
        let mut log = TestLog::attach(root.clone(), "");
        let notes = note_braid(&codec);
        let kitchen = kitchen_braid(&codec);
        log.publish(kitchen, &[insert(RECIPE, recipe_row(1, "base"))], 50);
        log.publish(notes, &[insert(NOTE, note_row(1, "one"))], 100);
        let dir = root.join("replica");
        let replica = open_replica(&root, &dir);
        assert_eq!(replica.vector()[&notes], 1);
        drop(replica);
        let slot = log.publish(notes, &[insert(NOTE, note_row(2, "two"))], 200);
        assert_eq!(slot, 2);

        // Execute the prefix mechanically, then kill: the state on disk
        // is exactly what a crash at that step leaves.
        if len > 0 {
            let bytes = store
                .get(&log_key("", notes, 2))
                .expect("get slot 2")
                .expect("slot 2 exists")
                .bytes;
            let batch = codec.decode(&bytes).expect("decode slot 2");
            let db: Db<SchemaDescriptor> = Db::open(&dir, theory()).expect("raw open");
            let mut chain = match Chain::read(&dir, codec.braids()) {
                SidecarRead::Read(chain) => chain,
                other => panic!("expected Read, got {}", other.identity()),
            };
            for step in &REPLICA_STEPS[..len] {
                match step {
                    ReplicaStep::ApplyLocal => {
                        let admission = db
                            .write(|tx| {
                                for op in &batch.ops {
                                    match op.kind {
                                        OpKind::Insert => {
                                            tx.insert_dyn(op.relation, op.rows.iter())?;
                                        }
                                        OpKind::Delete => {
                                            tx.delete_dyn(op.relation, op.rows.iter())?;
                                        }
                                    }
                                }
                                Ok(())
                            })
                            .expect("engine commit");
                        assert!(matches!(admission, Admission::Accepted(_)));
                    }
                    ReplicaStep::ChainAdvance => {
                        chain.entries_mut().insert(
                            notes,
                            ChainEntry {
                                g: 2,
                                prev: *blake3::hash(&bytes).as_bytes(),
                                ts: batch.header.timestamp,
                            },
                        );
                        chain.write_atomic(&dir).expect("write sidecar");
                    }
                }
            }
            drop(db);
        }

        obs::start_capture();
        let replica = open_replica(&root, &dir);
        let events = obs::finish_capture();
        assert_eq!(
            replica.provenance(),
            Provenance::LocalDir,
            "prefix {len}: recovery is the catch-up loop, never a discard"
        );
        assert!(replica.wedged().is_empty(), "prefix {len}");
        assert_eq!(replica.vector()[&notes], 2, "prefix {len}");
        let sum: u64 = replica.vector().values().sum();
        assert_eq!(
            replica.db().generation().expect("generation").value(),
            sum,
            "prefix {len}: the wholeness identity is exact"
        );
        let present = replica
            .db()
            .read(|instance| instance.contains_dyn(NOTE, &note_row(2, "two")))
            .expect("read");
        assert!(present, "prefix {len}");
        if len == 1 {
            assert!(
                noop_committed(&events),
                "prefix {len}: the crash-window re-application lands in the engine no-op arm"
            );
        }
        let probe = open_replica(&root, &root.join("probe"));
        assert_eq!(
            replica.db().catalog_digest().expect("digest"),
            probe.db().catalog_digest().expect("digest"),
            "prefix {len}: the recovered directory agrees with a fresh replay"
        );
    }
}

struct Rng(u64);

impl Rng {
    fn roll(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn double_apply_every_batch_at_every_prefix_leaves_digest_generation_vector_unmoved() {
    let codec = codec();
    let notes = note_braid(&codec);
    let kitchen = kitchen_braid(&codec);
    let venues = venue_braid(&codec);

    // A generated multi-braid history: every batch accepted and
    // state-changing by construction (fresh ids, deletes of live rows,
    // bookings well under the ceiling).
    let mut rng = Rng(0x5eed_0cf4_0000_0001);
    let mut history: Vec<(BraidId, Vec<Op>)> = vec![
        (venues, vec![insert(VENUE, Box::from([Value::U64(1)]))]),
        (kitchen, vec![insert(RECIPE, recipe_row(1, "base"))]),
    ];
    let mut next_note = 1u64;
    let mut live_notes: Vec<u64> = Vec::new();
    let mut next_recipe = 2u64;
    let mut recipes = vec![1u64];
    let mut next_unit = 1u64;
    for index in 0..40u64 {
        let entry = match rng.roll() % 6 {
            0 | 1 => {
                let id = next_note;
                next_note += 1;
                live_notes.push(id);
                (notes, vec![insert(NOTE, note_row(id, "gen"))])
            }
            2 => {
                if live_notes.len() > 1 {
                    let id = live_notes.remove(0);
                    (
                        notes,
                        vec![Op {
                            kind: OpKind::Delete,
                            relation: NOTE,
                            rows: vec![note_row(id, "gen")],
                        }],
                    )
                } else {
                    let id = next_note;
                    next_note += 1;
                    live_notes.push(id);
                    (notes, vec![insert(NOTE, note_row(id, "gen"))])
                }
            }
            3 => {
                let id = next_recipe;
                next_recipe += 1;
                recipes.push(id);
                (kitchen, vec![insert(RECIPE, recipe_row(id, "gen"))])
            }
            4 => {
                let count = u64::try_from(recipes.len()).expect("recipe count fits");
                let pick = usize::try_from(rng.roll() % count).expect("index fits");
                (
                    kitchen,
                    vec![insert(STEP, step_row(recipes[pick], &format!("s{index}")))],
                )
            }
            _ => {
                let unit = next_unit;
                next_unit += 1;
                (
                    venues,
                    vec![insert(
                        BOOKING,
                        Box::from([Value::U64(1), Value::U64(unit)]),
                    )],
                )
            }
        };
        history.push(entry);
    }

    // Encode against a builder store.
    let build_root = temp_dir("f4_l10_build");
    let db_build = create_db(&build_root.join("db"));
    let mut chain_build = Chain::genesis(codec.braids());
    let mut encoded: Vec<(BraidId, u64, Vec<u8>)> = Vec::new();
    for (index, (braid, ops)) in history.iter().enumerate() {
        let position = chain_build.position(*braid);
        let slot = position.g + 1;
        let header = BatchHeader {
            fingerprint: *codec.fingerprint(),
            braid: *braid,
            braid_gen: slot,
            prev: position.prev,
            writer: 77,
            timestamp: 1_000 + u64::try_from(index).expect("index fits"),
        };
        let bytes = codec.encode(&header, ops).expect("encode");
        let applied =
            apply(&db_build, &mut chain_build, &codec, *braid, slot, &bytes).expect("apply");
        assert!(
            matches!(applied, Applied::Advanced { .. }),
            "batch {index} must be state-changing: {applied:?}"
        );
        encoded.push((*braid, slot, bytes));
    }
    let digest_build = db_build.catalog_digest().expect("digest");

    // The oracle: replay on a second store, every batch applied twice
    // at every prefix — the second application through a chain that
    // missed the advance, the crash-window shape L10 heals.
    let replay_root = temp_dir("f4_l10_replay");
    let db_replay = create_db(&replay_root.join("db"));
    let mut chain_replay = Chain::genesis(codec.braids());
    for (index, (braid, slot, bytes)) in encoded.iter().enumerate() {
        let mut stale = chain_replay.clone();
        let first =
            apply(&db_replay, &mut chain_replay, &codec, *braid, *slot, bytes).expect("apply");
        assert!(matches!(first, Applied::Advanced { .. }), "prefix {index}");
        let generation = db_replay.generation().expect("generation").value();
        let digest = db_replay.catalog_digest().expect("digest");
        let vector = chain_replay.vector();
        let second = apply(&db_replay, &mut stale, &codec, *braid, *slot, bytes).expect("re-apply");
        match second {
            Applied::Absorbed {
                generation: absorbed,
            } => assert_eq!(absorbed, generation, "prefix {index}"),
            other => panic!("prefix {index}: the double apply must absorb, got {other:?}"),
        }
        assert_eq!(
            db_replay.generation().expect("generation").value(),
            generation,
            "prefix {index}: generation unmoved"
        );
        assert_eq!(
            db_replay.catalog_digest().expect("digest"),
            digest,
            "prefix {index}: digest unmoved"
        );
        assert_eq!(
            stale.vector(),
            vector,
            "prefix {index}: the vector caught up to the store"
        );
        assert_eq!(stale, chain_replay, "prefix {index}: chains agree whole");
    }
    assert_eq!(
        db_replay.catalog_digest().expect("digest"),
        digest_build,
        "the doubled replay converges to the builder's digest"
    );
}
