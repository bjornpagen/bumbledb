//! Conformance F1 — replay determinism, the base oracle. Random command
//! sequences over a three-braid fixture theory, three arrivals compared
//! at every probed vector: direct apply through the writer, replay
//! through the log, and a checkpoint hop (compact + upload mid-sequence,
//! restore, replay the tail). The gate is `catalog_digest`
//! triple-equality across one hundred generated worlds; a disagreement
//! is a trophy and the panic carries the seed.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Admission, Db, Value};
use bumbledb_log::apply::{Applied, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{Codec, Op, OpKind};
use bumbledb_log::gc::{Restore, restore_to_vector};
use bumbledb_log::manifest::{
    Checkpoint, Head, Manifest, Published, ckpt_mdb_key, log_key, manifest_key, publish_checkpoint,
};
use bumbledb_log::replica::{Opened, Provenance, Replica, Vector};
use bumbledb_log::sidecar::Chain;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{Create, ObjectStore};
use bumbledb_log::writer::{Batch, Commit, Options, Writer, WriterOpened};

const RECIPE: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const NOTE: RelationId = RelationId(2);
const VENUE: RelationId = RelationId(3);
const BOOKING: RelationId = RelationId(4);

const TITLES: [&str; 3] = ["alpha", "beta", "gamma"];
const NAMES: [&str; 4] = ["mix", "bake", "rest", "chop"];
const BODIES: [&str; 4] = ["draft", "final", "memo", "scrap"];

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-f1-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Three braids: recipe+step (key + containment), note alone, and
/// venue+booking under a tight capacity ceiling so the generator hits
/// accepts, functionality rejections, containment rejections, capacity
/// rejections, and net no-ops in one world.
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
                name: "venue".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "booking".into(),
                fields: vec![
                    field("venue", ValueType::U64),
                    field("units", ValueType::U64),
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
            StatementDescriptor::Functionality {
                relation: VENUE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(VENUE, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::Lit(8)),
                source: side(BOOKING, &[0]),
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

/// splitmix64 — the world generator's one source of randomness, seeded
/// per world so every trophy names its reproduction.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick(&mut self, len: usize) -> usize {
        let bound = u64::try_from(len).expect("list length fits u64");
        usize::try_from(self.next() % bound).expect("residue fits usize")
    }
}

/// Rows the generator has proposed so far — deletes draw from here so
/// some genuinely remove state while others evaporate against absence.
#[derive(Default)]
struct Shadow {
    recipes: Vec<Box<[Value]>>,
    steps: Vec<Box<[Value]>>,
    notes: Vec<Box<[Value]>>,
    bookings: Vec<Box<[Value]>>,
}

fn recipe_row(rng: &mut Rng) -> Box<[Value]> {
    Box::from([
        Value::U64(1 + rng.below(6)),
        Value::String(TITLES[rng.pick(TITLES.len())].into()),
    ])
}

fn step_row(rng: &mut Rng) -> Box<[Value]> {
    Box::from([
        Value::U64(1 + rng.below(6)),
        Value::String(NAMES[rng.pick(NAMES.len())].into()),
    ])
}

fn note_row(rng: &mut Rng) -> Box<[Value]> {
    Box::from([
        Value::U64(1 + rng.below(40)),
        Value::String(BODIES[rng.pick(BODIES.len())].into()),
    ])
}

fn venue_row(rng: &mut Rng) -> Box<[Value]> {
    Box::from([Value::U64(1 + rng.below(3))])
}

fn booking_row(rng: &mut Rng) -> Box<[Value]> {
    Box::from([Value::U64(1 + rng.below(4)), Value::U64(1 + rng.below(4))])
}

fn insert_op(
    rng: &mut Rng,
    relation: RelationId,
    shadow: &mut Vec<Box<[Value]>>,
    make: fn(&mut Rng) -> Box<[Value]>,
) -> Op {
    let rows: Vec<Box<[Value]>> = (0..=rng.below(2)).map(|_| make(rng)).collect();
    shadow.extend(rows.iter().cloned());
    Op {
        kind: OpKind::Insert,
        relation,
        rows,
    }
}

fn delete_op(rng: &mut Rng, relation: RelationId, shadow: &[Box<[Value]>]) -> Option<Op> {
    if shadow.is_empty() {
        return None;
    }
    let rows: Vec<Box<[Value]>> = (0..=rng.below(2))
        .map(|_| shadow[rng.pick(shadow.len())].clone())
        .collect();
    Some(Op {
        kind: OpKind::Delete,
        relation,
        rows,
    })
}

/// One command: one to two ops confined to a single braid, so the
/// writer's one-braid law holds by construction.
fn gen_command(rng: &mut Rng, shadow: &mut Shadow) -> Vec<Op> {
    let braid_pick = rng.below(3);
    let mut ops = Vec::new();
    for _ in 0..=rng.below(2) {
        let op = match braid_pick {
            0 => match rng.below(6) {
                0 | 1 => insert_op(rng, RECIPE, &mut shadow.recipes, recipe_row),
                2 | 3 => insert_op(rng, STEP, &mut shadow.steps, step_row),
                4 => delete_op(rng, RECIPE, &shadow.recipes)
                    .unwrap_or_else(|| insert_op(rng, RECIPE, &mut shadow.recipes, recipe_row)),
                _ => delete_op(rng, STEP, &shadow.steps)
                    .unwrap_or_else(|| insert_op(rng, STEP, &mut shadow.steps, step_row)),
            },
            1 => match rng.below(3) {
                0 | 1 => insert_op(rng, NOTE, &mut shadow.notes, note_row),
                _ => delete_op(rng, NOTE, &shadow.notes)
                    .unwrap_or_else(|| insert_op(rng, NOTE, &mut shadow.notes, note_row)),
            },
            _ => match rng.below(5) {
                0 => {
                    let rows = vec![venue_row(rng)];
                    Op {
                        kind: OpKind::Insert,
                        relation: VENUE,
                        rows,
                    }
                }
                1 | 2 => insert_op(rng, BOOKING, &mut shadow.bookings, booking_row),
                _ => delete_op(rng, BOOKING, &shadow.bookings)
                    .unwrap_or_else(|| insert_op(rng, BOOKING, &mut shadow.bookings, booking_row)),
            },
        };
        ops.push(op);
    }
    ops
}

fn stage<S: ObjectStore>(batch: &mut Batch<'_, S>, ops: &[Op]) {
    for op in ops {
        match op.kind {
            OpKind::Insert => batch.insert(op.relation, op.rows.iter().cloned()),
            OpKind::Delete => batch.delete(op.relation, op.rows.iter().cloned()),
        }
    }
}

/// The direct arrival: every command through one writer, recording each
/// published slot in commit order with the catalog digest the live
/// store held at that vector — the probe schedule the other two
/// arrivals are held to.
fn run_direct(
    root: &Path,
    seed: u64,
    commands: u64,
) -> (Vec<(BraidId, u64)>, Vec<[u8; 32]>, Vector) {
    let writer = match Writer::open(
        FsStore::new(root.to_path_buf()),
        "",
        &root.join("w"),
        theory(),
        Options::new(4200 + seed),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("seed {seed}: open refused: {refusal:?}"),
    };
    let mut rng = Rng(seed);
    let mut shadow = Shadow::default();
    let mut published: Vec<(BraidId, u64)> = Vec::new();
    let mut digests: Vec<[u8; 32]> = Vec::new();
    for index in 0..commands {
        let ops = gen_command(&mut rng, &mut shadow);
        let before = writer.vector();
        let outcome = writer
            .commit(|batch| {
                stage(batch, &ops);
                Ok(())
            })
            .unwrap_or_else(|error| panic!("seed {seed}: command {index} failed: {error}"));
        let after = writer.vector();
        match outcome {
            Commit::Accepted {
                braid, generation, ..
            } => {
                if after == before {
                    continue;
                }
                assert_eq!(
                    after.at(braid),
                    before.at(braid) + 1,
                    "seed {seed}: one batch advances its braid by exactly one"
                );
                assert_eq!(
                    generation,
                    after.at(braid),
                    "seed {seed}: the reported generation is the slot number"
                );
                published.push((braid, generation));
                digests.push(
                    writer
                        .with_db(Db::catalog_digest)
                        .expect("db")
                        .expect("direct digest"),
                );
            }
            Commit::Rejected(_) => {
                assert_eq!(
                    after, before,
                    "seed {seed}: a rejected commit advances nothing"
                );
            }
        }
    }
    let vector = writer.vector();
    let generation = writer
        .with_db(Db::generation)
        .expect("db")
        .expect("direct generation")
        .value();
    assert_eq!(
        generation,
        vector.sum().expect("sum"),
        "seed {seed}: the direct store ends whole"
    );
    writer.quiesce();
    (published, digests, vector)
}

/// Compacts the replay store at its current vector and publishes the
/// pair of checkpoint objects plus the manifest CAS — the direct
/// compact-and-upload arm of the checkpoint hop.
fn publish_hop_checkpoint(
    store: &FsStore,
    codec: &Codec,
    db: &Db<SchemaDescriptor>,
    chain: &Chain,
    scratch: &Path,
    seed: u64,
) {
    let _ = std::fs::remove_dir_all(scratch);
    db.compact(scratch).expect("compact at the hop");
    let bytes = std::fs::read(scratch.join("data.mdb")).expect("read compacted store");
    let _ = std::fs::remove_dir_all(scratch);
    let catalog = db.catalog_digest().expect("hop catalog digest");
    let heads: std::collections::BTreeMap<_, Head> = chain
        .entries()
        .iter()
        .map(|(braid, entry)| {
            (
                *braid,
                Head {
                    g: entry.g,
                    hash: entry.prev,
                    ts: entry.ts,
                },
            )
        })
        .collect();
    let prev = store
        .get(&manifest_key(""))
        .expect("manifest get")
        .and_then(|fetched| Manifest::parse(&fetched.bytes).ok()?.checkpoint);
    let doc = Checkpoint {
        braids: heads,
        catalog,
        writer: 4200 + seed,
        prev,
    };
    let digest = doc.digest();
    assert!(matches!(
        store
            .put_create(&ckpt_mdb_key("", &digest), &bytes)
            .expect("upload checkpoint object"),
        Create::Created(_)
    ));
    match publish_checkpoint(store, "", codec.braids(), &doc).expect("manifest CAS") {
        Published::Replaced | Published::Kept { .. } => {}
        Published::Refused(refusal) => panic!("seed {seed}: checkpoint refused: {refusal:?}"),
    }
}

fn fetch_slot(store: &FsStore, braid: BraidId, slot: u64, seed: u64) -> Vec<u8> {
    store
        .get(&log_key("", braid, slot))
        .expect("get log slot")
        .unwrap_or_else(|| panic!("seed {seed}: published slot {braid}/{slot} exists"))
        .bytes
}

/// One world, three arrivals. Replay applies every published slot in
/// commit order and compares the digest at every probed vector; at the
/// midpoint it publishes a checkpoint, and the hop arrival restores
/// from that checkpoint and replays the tail under the same probes.
fn run_world(seed: u64) {
    let root = temp_dir(&format!("world_{seed}"));
    let mut count_rng = Rng(seed ^ 0x5EED);
    let commands = 16 + count_rng.below(17);
    let (published, digests, direct_vector) = run_direct(&root, seed, commands);

    let store = FsStore::new(root.clone());
    let codec = codec();
    let replay_db = match Db::create(&root.join("replay"), theory()).expect("create replay store") {
        Admission::Accepted(db) => db,
        Admission::Rejected(violations) => {
            panic!("seed {seed}: the fixture theory admits an empty store: {violations:?}")
        }
    };
    let mut chain = Chain::genesis(codec.braids());
    let hop_pos = published.len() / 2;
    let mut hop: Option<(Vector, Chain)> = None;
    for (index, (braid, slot)) in published.iter().enumerate() {
        let bytes = fetch_slot(&store, *braid, *slot, seed);
        match apply(&replay_db, &mut chain, &codec, *braid, *slot, &bytes).expect("engine") {
            Applied::Advanced { .. } => {}
            other => panic!(
                "TROPHY seed {seed}: replay of probe {index} ({braid}/{slot}) \
                 was not a state-changing accept: {other:?}"
            ),
        }
        let got = replay_db.catalog_digest().expect("replay digest");
        assert_eq!(
            got, digests[index],
            "TROPHY seed {seed}: log replay disagrees with direct apply at probe {index}"
        );
        if index + 1 == hop_pos {
            publish_hop_checkpoint(
                &store,
                &codec,
                &replay_db,
                &chain,
                &root.join("scratch"),
                seed,
            );
            hop = Some((chain.vector(), chain.clone()));
        }
    }
    assert_eq!(
        chain.vector(),
        direct_vector,
        "seed {seed}: replay reaches the direct vector"
    );
    assert_eq!(
        replay_db.generation().expect("replay generation").value(),
        chain.sum(),
        "seed {seed}: the replay store ends whole"
    );

    let (hop_vector, mut hop_chain) = hop.expect("the checkpoint hop ran: every world crosses it");
    {
        let restored = restore_to_vector(&store, "", &root.join("hop"), &theory(), &hop_vector)
            .expect("restore infrastructure");
        let Restore::Restored { db: hop_db, vector } = restored else {
            let Restore::Refused(refusal) = restored else {
                unreachable!("restore is Restored or Refused");
            };
            panic!("TROPHY seed {seed}: restore to the hop vector refused: {refusal:?}");
        };
        assert_eq!(
            vector, hop_vector,
            "seed {seed}: restore reports its vector"
        );
        assert_eq!(
            hop_db.catalog_digest().expect("hop digest"),
            digests[hop_pos - 1],
            "TROPHY seed {seed}: checkpoint restore disagrees with direct apply at the hop vector"
        );
        for (index, (braid, slot)) in published.iter().enumerate().skip(hop_pos) {
            let bytes = fetch_slot(&store, *braid, *slot, seed);
            match apply(&hop_db, &mut hop_chain, &codec, *braid, *slot, &bytes).expect("engine") {
                Applied::Advanced { .. } => {}
                other => panic!(
                    "TROPHY seed {seed}: checkpoint-hop tail replay of probe {index} \
                     ({braid}/{slot}) was not a state-changing accept: {other:?}"
                ),
            }
            assert_eq!(
                hop_db.catalog_digest().expect("hop digest"),
                digests[index],
                "TROPHY seed {seed}: checkpoint hop disagrees with direct apply at probe {index}"
            );
        }
        assert_eq!(
            hop_db.generation().expect("hop generation").value(),
            hop_chain.sum(),
            "seed {seed}: the hopped store ends whole"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn one_hundred_worlds_agree_across_all_three_arrivals() {
    for seed in 1..=100 {
        run_world(seed);
    }
}

/// The writer's-duty arm of the hop: the cadence crossing publishes the
/// checkpoint mid-sequence, and a fresh replica seeds from it and
/// replays the tail to the tip — the production checkpoint-hop path
/// landing on the direct store's digest.
#[test]
fn the_writers_own_duty_checkpoint_hops_to_the_direct_digest() {
    let seed = 7u64;
    let root = temp_dir("duty_hop");
    let writer = match Writer::open(
        FsStore::new(root.clone()),
        "",
        &root.join("w"),
        theory(),
        Options::new(4300),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    writer.set_checkpoint_cadence(4, u64::MAX);
    let mut rng = Rng(seed);
    let mut shadow = Shadow::default();
    let mut publishes = 0u64;
    for index in 0..24 {
        let ops = gen_command(&mut rng, &mut shadow);
        let before = writer.vector();
        writer
            .commit(|batch| {
                stage(batch, &ops);
                Ok(())
            })
            .unwrap_or_else(|error| panic!("command {index} failed: {error}"));
        if writer.vector() != before {
            publishes += 1;
        }
    }
    assert!(
        publishes > 5,
        "the scripted world publishes enough slots to cross the cadence mid-sequence"
    );
    writer.quiesce();
    let direct_vector = writer.vector();
    let direct_digest = writer
        .with_db(Db::catalog_digest)
        .expect("db")
        .expect("direct digest");
    drop(writer);

    let opened = Replica::open(FsStore::new(root.clone()), "", &root.join("r"), theory())
        .expect("replica open");
    let Opened::Ready(replica) = opened else {
        let Opened::Refused(refusal) = opened else {
            unreachable!("open is Ready or Refused");
        };
        panic!("the replica opens against the duty's checkpoint: {refusal:?}");
    };
    assert_eq!(
        replica.provenance(),
        Provenance::Checkpoint,
        "the hop starts from the duty's checkpoint, not a full replay"
    );
    assert_eq!(replica.vector(), direct_vector);
    assert_eq!(
        replica
            .db()
            .expect("db")
            .catalog_digest()
            .expect("replica digest"),
        direct_digest,
        "TROPHY: the duty-checkpoint hop disagrees with direct apply at the tip"
    );
    let _ = std::fs::remove_dir_all(&root);
}
