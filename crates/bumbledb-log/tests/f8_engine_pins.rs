//! Conformance lane 8: the engine guarantees the protocol stands on,
//! pinned from the protocol's seat — through the writer's commit
//! discipline and `apply` over `FsStore`, never through engine
//! internals. Intern-mint determinism lands as byte-identical catalog
//! digests across independent replays; a fresh-in-command collision
//! arrives as an ordinary functionality rejection, serially and through
//! the loss path's re-judgment alike; op order inside a batch rides
//! the wire but cannot reach stored bytes; and the publish law's engine
//! half — `commit_noop` in the trace means no generation advance means
//! nothing to publish — is observed on the live trace names for the
//! net no-op commit, the rejected commit, and the crash-window
//! re-application whose no-op arm the recovery design stands on (L10).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValidateDescriptor as _, ValueType,
};
use bumbledb::{Db, Value, Violation, obs};
use bumbledb_log::apply::{Applied, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, OpKind};
use bumbledb_log::manifest::log_key;
use bumbledb_log::replica::{Opened, Replica};
use bumbledb_log::sidecar::Chain;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Durability, Options, Writer, WriterOpened};

const RECIPE: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const NOTE: RelationId = RelationId(2);
const TICKET: RelationId = RelationId(3);

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-f8-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Three braids: recipe+step under a key and a containment (string
/// fields so every commit minted intern ids), note alone, and ticket —
/// a `Fresh`-generation id whose auto-materialized key statement is the
/// fresh-row key form whose collisions this lane pins.
fn theory() -> SchemaDescriptor {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let side = |relation: RelationId, fields: &[u16]| Side {
        relation,
        projection: fields.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::from([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "recipe".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("title", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "step".into(),
                fields: vec![
                    field("recipe", ValueType::U64),
                    field("name", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "note".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("body", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "ticket".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    field("label", ValueType::String),
                ],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RECIPE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(STEP, &[0]),
                target: side(RECIPE, &[0]),
            },
        ],
    }
}

fn codec() -> Codec {
    let descriptor = theory();
    let schema = descriptor.clone().validate().expect("fixture validates");
    let fingerprint = schema_fingerprint(&schema).0;
    Codec::new(&descriptor, fingerprint)
}

fn kitchen_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(RECIPE).expect("recipe braid")
}

fn ticket_braid(codec: &Codec) -> BraidId {
    codec.braids().braid_of(TICKET).expect("ticket braid")
}

fn recipe_row(id: u64, title: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(title.into())])
}

fn step_row(recipe: u64, name: &str) -> Box<[Value]> {
    Box::from([Value::U64(recipe), Value::String(name.into())])
}

fn note_row(id: u64, body: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(body.into())])
}

fn ticket_row(id: u64, label: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(label.into())])
}

type FsWriter = Writer<SchemaDescriptor, FsStore>;
type FsReplica = Replica<SchemaDescriptor, FsStore>;

fn open_writer(root: PathBuf, dir: &Path, writer_id: u64) -> FsWriter {
    match Writer::open(
        FsStore::new(root),
        "",
        dir,
        theory(),
        Options::new(writer_id),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn open_replica(root: PathBuf, dir: &Path) -> FsReplica {
    match Replica::open(FsStore::new(root), "", dir, theory()).expect("open replica") {
        Opened::Ready(replica) => *replica,
        Opened::Refused(refusal) => panic!("replica refused: {refusal:?}"),
    }
}

fn writer_digest(writer: &FsWriter) -> [u8; 32] {
    writer
        .with_db(|db| db.catalog_digest().expect("catalog digest"))
        .expect("db")
}

fn replica_digest(replica: &FsReplica) -> [u8; 32] {
    replica
        .db()
        .expect("db")
        .catalog_digest()
        .expect("catalog digest")
}

fn accepted_generation<R>(outcome: &Commit<R>) -> u64 {
    match outcome {
        Commit::Accepted { generation, .. } => *generation,
        Commit::Rejected(violations) => panic!("accepted expected, got {violations:?}"),
    }
}

fn has_point(events: &[obs::TraceEvent], point: obs::TracePoint) -> bool {
    events.iter().any(|event| event.point() == point)
}

#[test]
fn intern_mint_determinism_lands_byte_identical_catalog_digests_across_replicas() {
    let root = temp_dir("intern");
    let writer = open_writer(root.clone(), &root.join("w"), 81);

    let first = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "sourdough")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&first), 1);

    // Listed order runs step ops before the recipe op: first-use mint
    // order inside the batch is the host's op order, and the log
    // carries exactly that order to every replay.
    let second = writer
        .commit(|batch| {
            batch.insert(
                STEP,
                [
                    step_row(1, "proof the levain"),
                    step_row(1, "shape the boule"),
                ],
            );
            batch.insert(RECIPE, [recipe_row(2, "rye")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&second), 2);

    // The note body reuses the string kitchen slot 1 minted — the one
    // slot every arrival order applies first — so this history's
    // fresh-mint sequence is the same under every cross-braid
    // interleaving: intern ids are store-local (the wire carries raw
    // values only), and the digest oracle demands equality exactly when
    // the fresh-mint sequences agree.
    let third = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "sourdough")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&third), 1);

    let minted = writer_digest(&writer);
    let replica_one = open_replica(root.clone(), &root.join("r1"));
    let replica_two = open_replica(root.clone(), &root.join("r2"));
    assert_eq!(
        replica_digest(&replica_one),
        minted,
        "an independent replay mints the writer's own intern ids"
    );
    assert_eq!(
        replica_digest(&replica_two),
        minted,
        "every replay of one log is one catalog content"
    );

    // Now the mints interleave across braids: each braid gains a slot
    // carrying a string the store has never seen. Every fresh replay
    // walks one catch-up order, so replays stay byte-identical to each
    // other — first-use mint order is deterministic per arrival, and
    // the replica's arrival is a deterministic function of the log.
    let fourth = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(3, "poolish overnight")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&fourth), 2);
    let fifth = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(3, "barm")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&fifth), 3);

    let replay_one = open_replica(root.clone(), &root.join("r3"));
    let replay_two = open_replica(root.clone(), &root.join("r4"));
    let replayed = replica_digest(&replay_one);
    assert_eq!(
        replica_digest(&replay_two),
        replayed,
        "identical batches against identical stores mint identical ids"
    );
    assert!(
        replay_one
            .db()
            .expect("db")
            .read(|instance| instance.contains_dyn(NOTE, &note_row(3, "poolish overnight")))
            .expect("read")
    );
    assert!(
        replay_one
            .db()
            .expect("db")
            .read(|instance| instance.contains_dyn(RECIPE, &recipe_row(3, "barm")))
            .expect("read")
    );
}

#[test]
fn fresh_in_command_ids_replay_and_a_collision_rejects_as_ordinary_functionality() {
    let root = temp_dir("fresh");
    let writer = open_writer(root.clone(), &root.join("w"), 82);
    let codec = codec();
    let braid = ticket_braid(&codec);

    let minted = writer
        .commit(|batch| {
            batch.insert(TICKET, [ticket_row(7, "first")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&minted), 1);

    let collision = writer
        .commit(|batch| {
            batch.insert(TICKET, [ticket_row(7, "second")]);
            Ok(())
        })
        .expect("commit");
    let Commit::Rejected(violations) = collision else {
        panic!("a fresh-id collision is a rejection, got an acceptance");
    };
    assert!(
        violations
            .iter()
            .any(|violation| matches!(violation, Violation::Functionality { .. })),
        "the collision is the key statement's ordinary verdict: {violations:?}"
    );

    let store = FsStore::new(root.clone());
    assert!(
        store.get(&log_key("", braid, 2)).expect("get").is_none(),
        "a rejected collision publishes nothing"
    );
    assert_eq!(
        writer.vector().at(braid),
        1,
        "the rejection advanced nothing"
    );

    let replica = open_replica(root.clone(), &root.join("r"));
    assert!(
        replica
            .db()
            .expect("db")
            .read(|instance| instance.contains_dyn(TICKET, &ticket_row(7, "first")))
            .expect("read"),
        "the fresh-keyed row replays with the id carried in the command"
    );
    assert_eq!(replica_digest(&replica), writer_digest(&writer));
}

#[test]
fn concurrent_fresh_double_mint_re_judges_to_the_serial_functionality_rejection() {
    let root = temp_dir("double_mint");
    let writer_a = open_writer(root.clone(), &root.join("wa"), 83);
    let writer_b = open_writer(root.clone(), &root.join("wb"), 84);
    let codec = codec();
    let braid = ticket_braid(&codec);

    let won = writer_a
        .commit(|batch| {
            batch.insert(TICKET, [ticket_row(7, "alpha")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&won), 1);

    // B still stands at the zero vector: its local apply accepts, its
    // publish loses slot 1, and the one path re-judges the recorded
    // ops against the winner's state — the serial verdict, unchanged.
    let lost = writer_b
        .commit(|batch| {
            batch.insert(TICKET, [ticket_row(7, "beta")]);
            Ok(())
        })
        .expect("commit");
    let Commit::Rejected(violations) = lost else {
        panic!("the double mint must re-judge to a rejection");
    };
    assert!(
        violations
            .iter()
            .any(|violation| matches!(violation, Violation::Functionality { .. })),
        "the concurrent collision carries the same ordinary verdict: {violations:?}"
    );
    assert_eq!(writer_b.losses(), 1, "one loss, one re-judgment");

    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 2)).expect("get").is_none(),
        "the loser's rejection publishes nothing"
    );
    assert_eq!(
        writer_b.vector().at(braid),
        1,
        "the loser holds the winner's slot"
    );
    assert_eq!(
        writer_digest(&writer_b),
        writer_digest(&writer_a),
        "both stores converge on the winner's mint"
    );
}

#[test]
fn host_order_inside_a_batch_rides_the_wire_but_cannot_reach_stored_bytes() {
    let root_fwd = temp_dir("order_fwd");
    let root_rev = temp_dir("order_rev");
    let forward = open_writer(root_fwd.clone(), &root_fwd.join("w"), 85);
    let reverse = open_writer(root_rev.clone(), &root_rev.join("w"), 86);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    // The intern first-use sequence is held fixed by an identical seed
    // on both stores, so the batch under test differs in op order and
    // nothing else — the scope of the canonical-plan-sort guarantee.
    let seed = |writer: &FsWriter| {
        let seeded = writer
            .commit(|batch| {
                batch.insert(
                    RECIPE,
                    [
                        recipe_row(100, "focaccia"),
                        recipe_row(101, "dimple the dough"),
                        recipe_row(102, "ciabatta"),
                    ],
                );
                Ok(())
            })
            .expect("seed");
        assert_eq!(accepted_generation(&seeded), 1);
    };
    seed(&forward);
    seed(&reverse);

    let fwd = forward
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(5, "focaccia")]);
            batch.insert(STEP, [step_row(5, "dimple the dough")]);
            batch.delete(RECIPE, [recipe_row(102, "ciabatta")]);
            Ok(())
        })
        .expect("forward order");
    assert_eq!(accepted_generation(&fwd), 2);
    let rev = reverse
        .commit(|batch| {
            batch.delete(RECIPE, [recipe_row(102, "ciabatta")]);
            batch.insert(STEP, [step_row(5, "dimple the dough")]);
            batch.insert(RECIPE, [recipe_row(5, "focaccia")]);
            Ok(())
        })
        .expect("reverse order");
    assert_eq!(accepted_generation(&rev), 2);

    let slot_fwd = FsStore::new(root_fwd)
        .get(&log_key("", braid, 2))
        .expect("get")
        .expect("published");
    let slot_rev = FsStore::new(root_rev.clone())
        .get(&log_key("", braid, 2))
        .expect("get")
        .expect("published");
    let batch_fwd = codec.decode(&slot_fwd.bytes).expect("decode");
    let batch_rev = codec.decode(&slot_rev.bytes).expect("decode");
    assert_eq!(batch_fwd.ops[0].kind, OpKind::Insert);
    assert_eq!(batch_fwd.ops[0].relation, RECIPE);
    assert_eq!(batch_rev.ops[0].kind, OpKind::Delete);
    assert_eq!(
        batch_rev.ops[2].relation, RECIPE,
        "the wire carries the host's own op order"
    );

    let digest = writer_digest(&forward);
    assert_eq!(
        writer_digest(&reverse),
        digest,
        "op order inside a batch cannot influence stored bytes"
    );
    let replica = open_replica(root_rev.clone(), &root_rev.join("r"));
    assert_eq!(
        replica_digest(&replica),
        digest,
        "a replay of the reversed wire lands the same catalog content"
    );
}

#[test]
fn a_net_noop_commit_takes_the_engine_noop_arm_and_publishes_nothing() {
    let root = temp_dir("noop");
    let writer = open_writer(root.clone(), &root.join("w"), 87);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let first = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(4, "soup")]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(accepted_generation(&first), 1);

    obs::start_capture();
    let again = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(4, "soup")]);
            Ok(())
        })
        .expect("commit");
    let events = obs::finish_capture();

    let Commit::Accepted {
        generation,
        durability,
        braid: got,
        ..
    } = again
    else {
        panic!("the net no-op reports acceptance at the current generation");
    };
    assert_eq!(got, braid);
    assert_eq!(
        generation, 1,
        "no advance: the current generation is the answer"
    );
    assert_eq!(durability, Durability::Published);

    assert!(
        has_point(&events, obs::names::COMMIT_NOOP),
        "the engine took its no-op arm: {events:?}"
    );
    assert!(
        !has_point(&events, obs::names::LMDB_COMMIT),
        "commit_noop means nothing durable moved"
    );

    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 2)).expect("get").is_none(),
        "no generation advance means nothing to publish"
    );
    assert_eq!(writer.vector().at(braid), 1);
    assert_eq!(
        writer
            .with_db(|db| db.generation().expect("generation").value())
            .expect("db"),
        1,
        "the wholeness identity holds with no pending term"
    );
    assert_eq!(writer.backlog(), None);
}

#[test]
fn a_rejected_commit_leaves_no_lmdb_commit_no_object_and_no_advance() {
    let root = temp_dir("rejected");
    let writer = open_writer(root.clone(), &root.join("w"), 88);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    obs::start_capture();
    let outcome = writer
        .commit(|batch| {
            batch.insert(STEP, [step_row(9, "knead")]);
            Ok(())
        })
        .expect("commit");
    let events = obs::finish_capture();

    assert!(
        matches!(outcome, Commit::Rejected(_)),
        "a step without its recipe violates the containment"
    );
    assert!(
        !has_point(&events, obs::names::LMDB_COMMIT),
        "a rejected commit leaves nothing durable: {events:?}"
    );
    assert!(
        !has_point(&events, obs::names::COMMIT_NOOP),
        "a rejection is a judged verdict, never the no-op arm"
    );

    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 1)).expect("get").is_none(),
        "rejected commits create no objects"
    );
    assert_eq!(writer.vector().at(braid), 0);
    assert_eq!(
        writer
            .with_db(|db| db.generation().expect("generation").value())
            .expect("db"),
        0,
        "rejected commits advance nothing"
    );
    assert_eq!(writer.backlog(), None);
}

#[test]
fn crash_window_reapply_lands_in_the_engine_noop_arm_on_the_trace() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let dir = temp_dir("reapply").join("db");
    let db: Db<SchemaDescriptor> = Db::create(&dir, theory())
        .expect("create")
        .expect("theory admits empty store");
    let mut chain = Chain::genesis(codec.braids());

    let header = BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid,
        braid_gen: 1,
        prev: [0u8; 32],
        writer: 89,
        timestamp: 500,
    };
    let bytes = codec
        .encode(
            &header,
            &[bumbledb_log::codec::Op {
                kind: OpKind::Insert,
                relation: RECIPE,
                rows: vec![recipe_row(9, "boule")],
            }],
        )
        .expect("encode");
    assert_eq!(
        apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply"),
        Applied::Advanced { generation: 1 }
    );

    // The crash window: the engine committed, the sidecar bump was
    // lost. The re-application net-disposes every op (L10), so the
    // engine's own no-op arm is what makes recovery need no detection
    // state.
    let mut rewound = Chain::genesis(codec.braids());
    obs::start_capture();
    let reapplied = apply(&db, &mut rewound, &codec, braid, 1, &bytes).expect("reapply");
    let events = obs::finish_capture();

    assert_eq!(reapplied, Applied::Absorbed { generation: 1 });
    assert!(
        has_point(&events, obs::names::COMMIT_NOOP),
        "the re-application observed commit_noop: {events:?}"
    );
    assert!(
        !has_point(&events, obs::names::LMDB_COMMIT),
        "no generation advance, nothing durable moved"
    );
    assert_eq!(db.generation().expect("generation").value(), 1);
    assert_eq!(rewound.position(braid).g, 1, "the identity lands exact");
}
