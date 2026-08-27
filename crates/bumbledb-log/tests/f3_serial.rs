//! Lane 3 — the serial-verdict lane. For each statement family a hand
//! fixture races a pair on a shared base, and the loser's re-judgment
//! must produce exactly the serial verdict: the double-booking FD
//! rejection, the dangling-reference verdict per order, the capacity
//! ceiling and floor rejections, the weighted-hold spend and reclaim
//! races, and the byte-equal absorption of an ambiguous PUT. Every
//! fixture cross-checks the racing outcome against a plain serial
//! execution of the same two batches on a fresh store — the verdict IS
//! a serial execution, performed.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Admission, Db, Value, Violation, Violations};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::manifest::{Head, log_key};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap,
};
use bumbledb_log::writer::{Options, Slotted, Writer, WriterOpened};

const SLOT: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const ENTRY: RelationId = RelationId(2);
const POOL: RelationId = RelationId(3);
const RES: RelationId = RelationId(4);
const UNIT_CHILD: RelationId = RelationId(5);
const VAULT: RelationId = RelationId(6);
const COIN: RelationId = RelationId(7);

const UNIT_CEILING: u64 = 6;
const RES_CEILING: u64 = 10;
const COIN_FLOOR: u64 = 2;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-f3-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// One relation per family: `slot` carries the key statement; `entry`
/// in `account` carries the containment; `pool` parents a unit-weight
/// child and the weighted `res` relation; the `vault`/`coin`
/// capacity has a floor above zero so the lower bound is reachable by
/// the engine, not only imagined.
#[allow(clippy::too_many_lines)]
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
                name: "slot".into(),
                fields: vec![
                    field("slot", ValueType::U64),
                    field("holder", ValueType::U64),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "account".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("name", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "entry".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("memo", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "pool".into(),
                fields: vec![field("id", ValueType::U64), field("label", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "res".into(),
                fields: vec![
                    field("pool", ValueType::U64),
                    field("units", ValueType::U64),
                    field("expiry", ValueType::U64),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "unit_child".into(),
                fields: vec![field("pool", ValueType::U64), field("tag", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "vault".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "coin".into(),
                fields: vec![field("vault", ValueType::U64), field("tag", ValueType::U64)],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: SLOT,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: ACCOUNT,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(ENTRY, &[0]),
                target: side(ACCOUNT, &[0]),
            },
            StatementDescriptor::Functionality {
                relation: POOL,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(POOL, &[0]),
                weight: Weight::Unit,
                lo: 0,
                hi: Some(Bound::Lit(UNIT_CEILING)),
                source: side(UNIT_CHILD, &[0]),
            },
            StatementDescriptor::Capacity {
                target: side(POOL, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::Lit(RES_CEILING)),
                source: side(RES, &[0]),
            },
            StatementDescriptor::Functionality {
                relation: VAULT,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(VAULT, &[0]),
                weight: Weight::Unit,
                lo: COIN_FLOOR,
                hi: Some(Bound::Lit(100)),
                source: side(COIN, &[0]),
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

fn braid(relation: RelationId) -> BraidId {
    codec().braids().braid_of(relation).expect("ordinary braid")
}

fn slot_row(slot: u64, holder: u64) -> Box<[Value]> {
    Box::from([Value::U64(slot), Value::U64(holder)])
}

fn account_row(id: u64, name: &str) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String(name.into())])
}

fn entry_row(account: u64, memo: &str) -> Box<[Value]> {
    Box::from([Value::U64(account), Value::String(memo.into())])
}

fn pool_row(id: u64, label: u64) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::U64(label)])
}

fn unit_row(pool: u64, tag: u64) -> Box<[Value]> {
    Box::from([Value::U64(pool), Value::U64(tag)])
}

fn res_row(pool: u64, units: u64, expiry: u64) -> Box<[Value]> {
    Box::from([Value::U64(pool), Value::U64(units), Value::U64(expiry)])
}

fn vault_row(id: u64) -> Box<[Value]> {
    Box::from([Value::U64(id)])
}

fn coin_row(vault: u64, tag: u64) -> Box<[Value]> {
    Box::from([Value::U64(vault), Value::U64(tag)])
}

fn ins(relation: RelationId, rows: Vec<Box<[Value]>>) -> Op {
    Op {
        kind: OpKind::Insert,
        relation,
        rows,
    }
}

fn del(relation: RelationId, rows: Vec<Box<[Value]>>) -> Op {
    Op {
        kind: OpKind::Delete,
        relation,
        rows,
    }
}

/// The law a rejection cites, read from its first violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Law {
    Functionality,
    Containment,
    Capacity,
}

fn cited(violations: &Violations) -> Law {
    match violations
        .get(0)
        .expect("a rejection cites at least one violation")
    {
        Violation::Functionality { .. } => Law::Functionality,
        Violation::Containment { .. } => Law::Containment,
        Violation::Capacity { .. } => Law::Capacity,
    }
}

fn fresh_db(tag: &str) -> Db<SchemaDescriptor> {
    let dir = temp_dir(tag).join("db");
    Db::create(&dir, theory())
        .expect("create store")
        .expect("theory admits the empty store")
}

fn apply_ops(db: &Db<SchemaDescriptor>, ops: &[Op]) -> Admission<u64> {
    db.write(|tx| {
        for op in ops {
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
    .expect("engine write")
    .map(|committed| committed.generation.value())
}

fn seeded(tag: &str, base: &[Op]) -> Db<SchemaDescriptor> {
    let db = fresh_db(tag);
    if !base.is_empty() {
        assert!(
            matches!(apply_ops(&db, base), Admission::Accepted(_)),
            "the shared base admits"
        );
    }
    db
}

/// Applies base then `first` (both must accept) and asserts the second
/// batch's serial verdict cites `law` — the oracle every racing fixture
/// is held to.
fn assert_second_rejects(tag: &str, base: &[Op], first: &[Op], second: &[Op], law: Law) {
    let db = seeded(tag, base);
    assert!(
        matches!(apply_ops(&db, first), Admission::Accepted(_)),
        "the winner is individually valid on the shared base"
    );
    match apply_ops(&db, second) {
        Admission::Rejected(violations) => {
            assert_eq!(cited(&violations), law, "the serial verdict cites its law");
        }
        Admission::Accepted(_) => panic!("{tag}: the serial verdict is a rejection"),
    }
}

type FsWriter = Writer<SchemaDescriptor, FsStore>;

fn open_at(root: PathBuf, dir: &Path, writer_id: u64) -> FsWriter {
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

/// A test-side slot publisher: walks the braid's log to its tip so a
/// planted competitor chains onto whatever the writer under test has
/// already published, then claims the next slot.
struct Planter {
    store: FsStore,
    codec: Codec,
}

impl Planter {
    fn new(root: PathBuf) -> Self {
        Self {
            store: FsStore::new(root),
            codec: codec(),
        }
    }

    fn plant(&self, braid: BraidId, ops: &[Op]) -> u64 {
        let mut head = Head {
            g: 0,
            hash: [0u8; 32],
            ts: 0,
        };
        loop {
            let key = log_key("", braid, head.g + 1);
            let Some(fetched) = self.store.get(&key).expect("walk the log") else {
                break;
            };
            let batch = self
                .codec
                .decode(&fetched.bytes)
                .expect("published slots decode");
            head.g += 1;
            head.hash = *blake3::hash(&fetched.bytes).as_bytes();
            head.ts = batch.header.timestamp;
        }
        let slot = head.g + 1;
        let header = BatchHeader {
            fingerprint: *self.codec.fingerprint(),
            braid,
            braid_gen: slot,
            prev: head.hash,
            writer: 7002,
            timestamp: head.ts.max(1),
        };
        let bytes = self.codec.encode(&header, ops).expect("encode competitor");
        assert!(
            matches!(
                self.store
                    .put_create(&log_key("", braid, slot), &bytes)
                    .expect("plant competitor"),
                Create::Created(_)
            ),
            "the planted slot is free"
        );
        slot
    }

    fn slot_absent(&self, braid: BraidId, slot: u64) -> bool {
        self.store
            .get(&log_key("", braid, slot))
            .expect("probe slot")
            .is_none()
    }
}

struct Race {
    writer: FsWriter,
    planter: Planter,
}

fn race(tag: &str) -> Race {
    let root = temp_dir(tag);
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 31);
    let planter = Planter::new(root);
    Race { writer, planter }
}

fn accepted_slot<R>(outcome: &Admission<Slotted<R>>) -> u64 {
    match outcome {
        Admission::Accepted(slotted) => slotted.slot,
        Admission::Rejected(violations) => panic!("accepted expected, rejected: {violations:?}"),
    }
}

fn rejected_law<R>(outcome: &Admission<Slotted<R>>) -> Law {
    match outcome {
        Admission::Rejected(violations) => cited(violations),
        Admission::Accepted(_) => panic!("rejected expected"),
    }
}

// --- the key family: the double booking ---

#[test]
fn double_booking_rejudges_to_the_serial_fd_rejection() {
    let winner = [ins(SLOT, vec![slot_row(5, 2)])];
    let loser = [ins(SLOT, vec![slot_row(5, 1)])];
    assert_second_rejects("fd_serial", &[], &winner, &loser, Law::Functionality);

    let fixture = race("fd_race");
    let slot_braid = braid(SLOT);
    fixture.planter.plant(slot_braid, &winner);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(SLOT, [slot_row(5, 1)]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(
        rejected_law(&outcome),
        Law::Functionality,
        "the double-booking refused with a proof"
    );
    assert_eq!(fixture.writer.losses(), 1, "one loss, one re-judgment");
    assert!(
        fixture.planter.slot_absent(slot_braid, 2),
        "a rejected loser publishes nothing"
    );
    fixture
        .writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(SLOT, &slot_row(5, 2))?);
                assert!(!instance.contains_dyn(SLOT, &slot_row(5, 1))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
}

// --- the containment family: the dangling reference, per order ---

#[test]
fn dangling_reference_source_loser_gets_the_containment_rejection() {
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let remove = [del(ACCOUNT, vec![account_row(7, "base")])];
    let need = [ins(ENTRY, vec![entry_row(7, "x")])];
    assert_second_rejects("dangle_rn", &base, &remove, &need, Law::Containment);

    let fixture = race("dangle_src");
    let account_braid = braid(ACCOUNT);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(ACCOUNT, [account_row(7, "base")]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_slot(&outcome), 1);
    fixture.planter.plant(account_braid, &remove);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(ENTRY, [entry_row(7, "x")]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(rejected_law(&outcome), Law::Containment);
    assert_eq!(fixture.writer.losses(), 1);
    assert!(fixture.planter.slot_absent(account_braid, 3));
}

#[test]
fn dangling_reference_target_loser_gets_the_containment_rejection() {
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let need = [ins(ENTRY, vec![entry_row(7, "x")])];
    let remove = [del(ACCOUNT, vec![account_row(7, "base")])];
    assert_second_rejects("dangle_nr", &base, &need, &remove, Law::Containment);

    let fixture = race("dangle_tgt");
    let account_braid = braid(ACCOUNT);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(ACCOUNT, [account_row(7, "base")]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_slot(&outcome), 1);
    fixture.planter.plant(account_braid, &need);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(ACCOUNT, [account_row(7, "base")]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(rejected_law(&outcome), Law::Containment);
    assert_eq!(fixture.writer.losses(), 1);
    assert!(fixture.planter.slot_absent(account_braid, 3));
}

// --- the capacity family: ceiling and floor ---

#[test]
fn capacity_ceiling_loser_rejudges_to_the_serial_rejection() {
    let base = vec![
        ins(POOL, vec![pool_row(1, 0)]),
        ins(UNIT_CHILD, vec![unit_row(1, 0), unit_row(1, 1)]),
    ];
    let winner = [ins(
        UNIT_CHILD,
        vec![
            unit_row(1, 10),
            unit_row(1, 11),
            unit_row(1, 12),
            unit_row(1, 13),
        ],
    )];
    let loser = [ins(UNIT_CHILD, vec![unit_row(1, 20)])];
    assert_second_rejects("ceiling_serial", &base, &winner, &loser, Law::Capacity);

    let fixture = race("ceiling_race");
    let pool_braid = braid(POOL);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(POOL, [pool_row(1, 0)]);
            batch.insert(UNIT_CHILD, [unit_row(1, 0), unit_row(1, 1)]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_slot(&outcome), 1);
    fixture.planter.plant(pool_braid, &winner);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(UNIT_CHILD, [unit_row(1, 20)]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(
        rejected_law(&outcome),
        Law::Capacity,
        "the loser re-judges to the ceiling rejection"
    );
    assert!(fixture.planter.slot_absent(pool_braid, 3));
}

#[test]
fn capacity_floor_loser_rejudges_to_the_serial_rejection() {
    let base = vec![
        ins(VAULT, vec![vault_row(1)]),
        ins(COIN, vec![coin_row(1, 0), coin_row(1, 1), coin_row(1, 2)]),
    ];
    let winner = [del(COIN, vec![coin_row(1, 0)])];
    let loser = [del(COIN, vec![coin_row(1, 1)])];
    assert_second_rejects("floor_serial", &base, &winner, &loser, Law::Capacity);

    let fixture = race("floor_race");
    let vault_braid = braid(VAULT);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(VAULT, [vault_row(1)]);
            batch.insert(COIN, [coin_row(1, 0), coin_row(1, 1), coin_row(1, 2)]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_slot(&outcome), 1);
    fixture.planter.plant(vault_braid, &winner);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(COIN, [coin_row(1, 1)]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(
        rejected_law(&outcome),
        Law::Capacity,
        "the loser re-judges to the floor rejection"
    );
    assert!(fixture.planter.slot_absent(vault_braid, 3));
}

// --- the weighted-hold family: spend and reclaim races ---

#[test]
fn weighted_hold_spend_outraced_by_a_fill_publishes_at_the_tip() {
    // The winner books unrelated units with headroom to spare; the
    // spend re-judges clean at the moved base and publishes.
    let fixture = race("spend_publish");
    let pool_braid = braid(POOL);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(POOL, [pool_row(9, 0)]);
            Ok(())
        })
        .expect("pool commit");
    assert_eq!(accepted_slot(&outcome), 1);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(RES, [res_row(9, 3, 99)]);
            Ok(())
        })
        .expect("hold commit");
    assert_eq!(accepted_slot(&outcome), 2);
    fixture
        .planter
        .plant(pool_braid, &[ins(RES, vec![res_row(9, 2, 50)])]);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(RES, [res_row(9, 3, 99)]);
            batch.insert(RES, [res_row(9, 3, 0)]);
            Ok(())
        })
        .expect("spend commit");
    assert_eq!(
        accepted_slot(&outcome),
        4,
        "the spend re-judges clean and publishes behind the winner"
    );
    assert_eq!(fixture.writer.losses(), 1);
    fixture
        .writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(!instance.contains_dyn(RES, &res_row(9, 3, 99))?);
                assert!(instance.contains_dyn(RES, &res_row(9, 3, 0))?);
                assert!(instance.contains_dyn(RES, &res_row(9, 2, 50))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
}

#[test]
fn weighted_hold_spend_outraced_by_a_reclaim_rejudges_to_the_serial_rejection() {
    // The winner reclaims the hold and re-books it elsewhere; the
    // loser's spend re-judges with its delete evaporated and its
    // children priced against the real slack — the serial rejection.
    let fixture = race("spend_reclaim");
    let pool_braid = braid(POOL);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(POOL, [pool_row(9, 0)]);
            Ok(())
        })
        .expect("pool commit");
    assert_eq!(accepted_slot(&outcome), 1);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(RES, [res_row(9, 3, 99)]);
            Ok(())
        })
        .expect("hold commit");
    assert_eq!(accepted_slot(&outcome), 2);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(RES, [res_row(9, 7, 50)]);
            Ok(())
        })
        .expect("fill commit");
    assert_eq!(accepted_slot(&outcome), 3);
    fixture.planter.plant(
        pool_braid,
        &[
            del(RES, vec![res_row(9, 3, 99)]),
            ins(RES, vec![res_row(9, 3, 77)]),
        ],
    );
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(RES, [res_row(9, 3, 99)]);
            batch.insert(RES, [res_row(9, 3, 0)]);
            Ok(())
        })
        .expect("spend commit");
    assert_eq!(
        rejected_law(&outcome),
        Law::Capacity,
        "the spend prices its children against the real slack"
    );
    assert!(fixture.planter.slot_absent(pool_braid, 5));
}

// --- the byte-equal absorption ---

/// Performs the first log-slot create for real but reports `Exists` —
/// the ambiguous-PUT shape whose one sound resolution is
/// fetch-and-compare.
struct AmbiguousOnce {
    inner: FsStore,
    tripped: AtomicU64,
}

impl ObjectStore for AmbiguousOnce {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        let created = self.inner.put_create(key, body)?;
        if key.as_str().starts_with("log/")
            && let Create::Created(_) = created
            && self.tripped.swap(1, Ordering::SeqCst) == 0
        {
            return Ok(Create::Exists);
        }
        Ok(created)
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> StoreResult<Swap> {
        self.inner.put_swap(key, body, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

#[test]
fn byte_equal_exists_is_ours_and_absorbs_without_a_loss() {
    let root = temp_dir("absorb");
    let store = AmbiguousOnce {
        inner: FsStore::new(root.clone()),
        tripped: AtomicU64::new(0),
    };
    let opened =
        Writer::open(store, "", &root.join("w"), theory(), Options::new(31)).expect("open");
    let WriterOpened::Ready(writer) = opened else {
        panic!("ready expected");
    };
    let outcome = writer
        .commit(|batch| {
            batch.insert(SLOT, [slot_row(1, 1)]);
            Ok(())
        })
        .expect("commit");
    assert!(
        matches!(outcome, Admission::Accepted(Slotted { slot: 1, .. })),
        "byte-equal Exists is our own earlier PUT, absorbed"
    );
    assert_eq!(writer.losses(), 0, "absorption is not a loss");
    let plain = Planter::new(root);
    assert!(
        !plain.slot_absent(braid(SLOT), 1),
        "the ambiguous create landed exactly once"
    );
    assert!(plain.slot_absent(braid(SLOT), 2));
}
