//! Lane 3: the conflict matrix, adversarially, cell by cell. Every
//! cell of the four commutativity matrices (F, K, C, W) is a hand
//! fixture pair built on a shared base, each test named by its matrix
//! coordinates. Commute cells run both apply orders to equal raw-value
//! state digests (L8's executable shadow) and resolve in the loser algebra
//! without re-judgment where the algebra's strict disjointness allows
//! it: the subsumed arm where the pair shares its effects, the
//! republish arm where no key of any class is shared. A shared
//! commute-cell key re-judges by design — L6's hypothesis is full key
//! disjointness, strictly stronger than "no CONFLICT cell" — so those
//! cells pin the commute with the order test and the conservative
//! re-judgment with the intersect verdict. CONFLICT cells re-judge to
//! exactly the serial verdict. The W-class quantitative boundary runs
//! worst-case interval endpoints at slack and slack + 1, unit and
//! weighted children, ceiling and floor, widened and unwidened; the
//! evaporation fixtures defeat a naive point-delta oracle; the
//! reservation spend runs both arms. The fixture table is an
//! exhaustive match over the footprint classes, so a new class fails
//! to compile before it ships unmatrixed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, StatementId, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Admission, Db, Value, Violation, Violations};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::footprint::{
    CapacityKey, CapacityMode, CapacityProfile, ContainmentMode, Entry, Vocabulary,
    capacity_profiles, footprint,
};
use bumbledb_log::intersect::{
    BaseMeasure, CapacityCell, ConflictCause, LoserDecision, capacity_cell, intersect,
};
use bumbledb_log::manifest::{Head, log_key};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{Create, ObjectStore};
use bumbledb_log::writer::{Commit, Options, Writer, WriterOpened};

const TAG: RelationId = RelationId(0);
const SLOT: RelationId = RelationId(1);
const ACCOUNT: RelationId = RelationId(2);
const ENTRY: RelationId = RelationId(3);
const POOL: RelationId = RelationId(4);
const UNIT_CHILD: RelationId = RelationId(5);
const RES: RelationId = RelationId(6);
const VAULT: RelationId = RelationId(7);
const COIN: RelationId = RelationId(8);

const SLOT_KEY: StatementId = StatementId(0);
const ACCOUNT_KEY: StatementId = StatementId(1);
const ENTRY_IN_ACCOUNT: StatementId = StatementId(2);
const POOL_KEY: StatementId = StatementId(3);
const UNIT_CAPACITY: StatementId = StatementId(4);
const RES_CAPACITY: StatementId = StatementId(5);
const COIN_CAPACITY: StatementId = StatementId(7);

const UNIT_CEILING: u64 = 6;
const RES_CEILING: u64 = 10;
const COIN_FLOOR: u64 = 2;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("f3_{tag}_{}_{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// One relation per matrix need: `tag` carries only fact identity (the
/// F rows in isolation); `slot` carries the key statement; `entry` in
/// `account` carries the containment; `pool` parents a unit-weight and
/// a weighted child (the weighted one in the reservation shape); the
/// `vault`/`coin` capacity has a floor above zero so the lower bound is
/// reachable by the engine, not only by a supplied measure. Every
/// containment and capacity target is keyed on its target projection —
/// the star-guarded discipline the validator mandates — so a target
/// group holds at most one row and distinct-row support races co-fire
/// the K matrix.
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
                name: "tag".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("note", ValueType::String),
                ],
                extension: None,
            },
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
                name: "unit_child".into(),
                fields: vec![field("pool", ValueType::U64), field("tag", ValueType::U64)],
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
    Codec::new(&descriptor, fingerprint).expect("fixture vocabulary")
}

fn vocabulary() -> Vocabulary {
    Vocabulary::new(&theory()).expect("fixture vocabulary")
}

fn braid(relation: RelationId) -> BraidId {
    codec().braids().braid_of(relation).expect("ordinary braid")
}

fn tag_row(id: u64) -> Box<[Value]> {
    Box::from([Value::U64(id), Value::String("note".into())])
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

/// A representation-independent digest of the judged content: every
/// relation's rows raw-value-hashed the footprint's way, sorted, then
/// folded. L8 states set-level equality across apply orders, and this
/// is its executable form. The engine's `catalog_digest` cannot serve
/// here: it folds `F | relation | row_id` catalog keys
/// (`crates/bumbledb/src/storage/keys.rs`), and row ids are allocated
/// in commit order, so byte-level catalog equality does not survive an
/// apply-order swap inside one relation even when the fact sets agree.
fn hash_value(hasher: &mut blake3::Hasher, value: &Value) {
    match value {
        Value::Bool(b) => {
            hasher.update(&[0, u8::from(*b)]);
        }
        Value::U64(v) => {
            hasher.update(&[1]);
            hasher.update(&v.to_le_bytes());
        }
        Value::I64(v) => {
            hasher.update(&[2]);
            hasher.update(&v.to_le_bytes());
        }
        Value::String(s) => {
            hasher.update(&[3]);
            hasher.update(&len_word(s.len()));
            hasher.update(s.as_bytes());
        }
        Value::FixedBytes(raw) => {
            hasher.update(&[4]);
            hasher.update(&len_word(raw.len()));
            hasher.update(raw);
        }
        Value::IntervalU64(interval) => {
            hasher.update(&[5]);
            hasher.update(&interval.start().to_le_bytes());
            hasher.update(&interval.end().to_le_bytes());
        }
        Value::IntervalI64(interval) => {
            hasher.update(&[6]);
            hasher.update(&interval.start().to_le_bytes());
            hasher.update(&interval.end().to_le_bytes());
        }
    }
}

fn len_word(len: usize) -> [u8; 8] {
    u64::try_from(len).expect("length fits u64").to_le_bytes()
}

fn state_digest(db: &Db<SchemaDescriptor>) -> [u8; 32] {
    let descriptor = theory();
    let mut fids: Vec<[u8; 32]> = Vec::new();
    db.read(|instance| {
        for index in 0..descriptor.relations.len() {
            let id = RelationId(u32::try_from(index).expect("relation count fits u32"));
            for row in instance.scan(id)? {
                let row = row?;
                let mut hasher = blake3::Hasher::new();
                hasher.update(&id.0.to_le_bytes());
                for value in &row {
                    hash_value(&mut hasher, value);
                }
                fids.push(*hasher.finalize().as_bytes());
            }
        }
        Ok(())
    })
    .expect("scan the store");
    fids.sort_unstable();
    let mut digest = blake3::Hasher::new();
    for fid in &fids {
        digest.update(fid);
    }
    *digest.finalize().as_bytes()
}

/// Both orders accepted on the shared base and the state digests
/// equal — the executable form of L8's state equality.
fn assert_commutes(tag: &str, base: &[Op], a: &[Op], b: &[Op]) {
    let ab = {
        let db = seeded(&format!("{tag}_ab"), base);
        assert!(matches!(apply_ops(&db, a), Admission::Accepted(_)));
        assert!(matches!(apply_ops(&db, b), Admission::Accepted(_)));
        state_digest(&db)
    };
    let ba = {
        let db = seeded(&format!("{tag}_ba"), base);
        assert!(matches!(apply_ops(&db, b), Admission::Accepted(_)));
        assert!(matches!(apply_ops(&db, a), Admission::Accepted(_)));
        state_digest(&db)
    };
    assert_eq!(ab, ba, "either apply order yields the identical state (L8)");
}

/// Applies base then `first` (both must accept) and returns the second
/// batch's admission — the serial verdict the loser must re-judge to.
fn second_verdict(tag: &str, base: &[Op], first: &[Op], second: &[Op]) -> Admission<u64> {
    let db = seeded(tag, base);
    assert!(
        matches!(apply_ops(&db, first), Admission::Accepted(_)),
        "the winner is individually valid on the shared base"
    );
    apply_ops(&db, second)
}

fn assert_second_rejects(tag: &str, base: &[Op], first: &[Op], second: &[Op], law: Law) {
    match second_verdict(tag, base, first, second) {
        Admission::Rejected(violations) => {
            assert_eq!(cited(&violations), law, "the serial verdict cites its law");
        }
        Admission::Accepted(_) => panic!("{tag}: the serial verdict is a rejection"),
    }
}

fn decide(loser: &[Op], winner: &[Op], base: &BTreeMap<CapacityKey, BaseMeasure>) -> LoserDecision {
    let vocab = vocabulary();
    let loser_fp = footprint(&vocab, loser).expect("loser footprint");
    intersect(&vocab, &loser_fp, loser, winner, base).expect("intersect")
}

/// One `BaseMeasure` for every capacity key either batch touches — the
/// caller-supplied base-state prices the interval test consumes.
fn shared_measures(
    a: &[Op],
    b: &[Op],
    measure: u64,
    floor: u64,
    ceiling: Option<u64>,
) -> BTreeMap<CapacityKey, BaseMeasure> {
    let vocab = vocabulary();
    let mut map = BTreeMap::new();
    for ops in [a, b] {
        for key in capacity_profiles(&vocab, ops)
            .expect("profiles")
            .into_keys()
        {
            map.insert(
                key,
                BaseMeasure {
                    measure,
                    floor,
                    ceiling,
                },
            );
        }
    }
    map
}

fn profile_at(ops: &[Op], statement: StatementId) -> CapacityProfile {
    capacity_profiles(&vocabulary(), ops)
        .expect("profiles")
        .into_iter()
        .find_map(|(key, profile)| (key.statement == statement).then_some(profile))
        .expect("a capacity profile at the statement")
}

/// The oracle the interval law exists to refute: sum the published
/// point deltas and check them against the bounds, ignoring that set
/// semantics can evaporate ops against the final base.
fn naive_point_delta_commutes(
    loser: CapacityProfile,
    winner: CapacityProfile,
    base: BaseMeasure,
) -> bool {
    let joint = i128::from(base.measure) + loser.delta + winner.delta;
    let under = base
        .ceiling
        .is_none_or(|ceiling| joint <= i128::from(ceiling));
    under && joint >= i128::from(base.floor)
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

fn accepted_generation<R>(outcome: &Commit<R>) -> u64 {
    match outcome {
        Commit::Accepted { generation, .. } => *generation,
        Commit::Rejected(violations) => panic!("accepted expected, rejected: {violations:?}"),
    }
}

fn rejected_law<R>(outcome: &Commit<R>) -> Law {
    match outcome {
        Commit::Rejected(violations) => cited(violations),
        Commit::Accepted { .. } => panic!("rejected expected"),
    }
}

/// One coordinate of the F matrix (shared fact id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FCell {
    InsertXInsert,
    InsertXDelete,
    DeleteXInsert,
    DeleteXDelete,
}

/// One coordinate of the K matrix (shared determinant under one key
/// statement), plus its two recorded boundaries: the byte-identical
/// exception the F table owns and the distinct-determinant workhorse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum KCell {
    InsertXInsert,
    InsertXDelete,
    DeleteXDelete,
    ByteIdenticalFException,
    DistinctDeterminants,
}

/// One coordinate of the C matrix (shared target group under one
/// containment), row = loser's mode, column = winner's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CCell {
    NeedXNeed,
    NeedXSupportAdd,
    NeedXSupportRemove,
    SupportAddXNeed,
    SupportAddXSupportAdd,
    SupportAddXSupportRemove,
    SupportRemoveXNeed,
    SupportRemoveXSupportAdd,
    SupportRemoveXSupportRemove,
}

/// The W matrix's coordinates: the childΔ×childΔ quantitative boundary
/// (both weights, both bounds, at slack and one past it), the widened
/// evaporation cells with their point-delta refutation, the parent
/// rows, and the reservation idiom's two arms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum WCell {
    UnitCeilingAtSlack,
    UnitCeilingPastSlack,
    WeightedCeilingAtSlack,
    WeightedCeilingPastSlack,
    UnitFloorAtSlack,
    UnitFloorPastSlack,
    WeightedFloorAtSlack,
    WeightedFloorPastSlack,
    EvaporationWithHeadroom,
    EvaporationAtTheBound,
    NaivePointDeltaRefuted,
    ChildAddXParentAdd,
    ChildRemoveXParentAdd,
    ChildDeltaXParentRemove,
    ParentAddXParentAdd,
    ParentAddXParentRemove,
    ParentRemoveXParentRemove,
    ParentRemoveXInert,
    SpendCommuteArm,
    SpendConflictArm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Cell {
    F(FCell),
    K(KCell),
    C(CCell),
    W(WCell),
}

/// The roster, keyed by footprint class: a fifth `Entry` arm fails
/// this match before it ships without matrix cells.
fn cells_of(class: &Entry) -> Vec<Cell> {
    match class {
        Entry::Fact { .. } => vec![
            Cell::F(FCell::InsertXInsert),
            Cell::F(FCell::InsertXDelete),
            Cell::F(FCell::DeleteXInsert),
            Cell::F(FCell::DeleteXDelete),
        ],
        Entry::Key { .. } => vec![
            Cell::K(KCell::InsertXInsert),
            Cell::K(KCell::InsertXDelete),
            Cell::K(KCell::DeleteXDelete),
            Cell::K(KCell::ByteIdenticalFException),
            Cell::K(KCell::DistinctDeterminants),
        ],
        Entry::Containment { .. } => vec![
            Cell::C(CCell::NeedXNeed),
            Cell::C(CCell::NeedXSupportAdd),
            Cell::C(CCell::NeedXSupportRemove),
            Cell::C(CCell::SupportAddXNeed),
            Cell::C(CCell::SupportAddXSupportAdd),
            Cell::C(CCell::SupportAddXSupportRemove),
            Cell::C(CCell::SupportRemoveXNeed),
            Cell::C(CCell::SupportRemoveXSupportAdd),
            Cell::C(CCell::SupportRemoveXSupportRemove),
        ],
        Entry::Capacity { .. } => vec![
            Cell::W(WCell::UnitCeilingAtSlack),
            Cell::W(WCell::UnitCeilingPastSlack),
            Cell::W(WCell::WeightedCeilingAtSlack),
            Cell::W(WCell::WeightedCeilingPastSlack),
            Cell::W(WCell::UnitFloorAtSlack),
            Cell::W(WCell::UnitFloorPastSlack),
            Cell::W(WCell::WeightedFloorAtSlack),
            Cell::W(WCell::WeightedFloorPastSlack),
            Cell::W(WCell::EvaporationWithHeadroom),
            Cell::W(WCell::EvaporationAtTheBound),
            Cell::W(WCell::NaivePointDeltaRefuted),
            Cell::W(WCell::ChildAddXParentAdd),
            Cell::W(WCell::ChildRemoveXParentAdd),
            Cell::W(WCell::ChildDeltaXParentRemove),
            Cell::W(WCell::ParentAddXParentAdd),
            Cell::W(WCell::ParentAddXParentRemove),
            Cell::W(WCell::ParentRemoveXParentRemove),
            Cell::W(WCell::ParentRemoveXInert),
            Cell::W(WCell::SpendCommuteArm),
            Cell::W(WCell::SpendConflictArm),
        ],
    }
}

/// The one dispatch every named test and the roster share: a cell
/// without a fixture is a missing match arm.
fn check(cell: Cell) {
    match cell {
        Cell::F(FCell::InsertXInsert) => f_insert_x_insert(),
        Cell::F(FCell::InsertXDelete) => f_insert_x_delete(),
        Cell::F(FCell::DeleteXInsert) => f_delete_x_insert(),
        Cell::F(FCell::DeleteXDelete) => f_delete_x_delete(),
        Cell::K(KCell::InsertXInsert) => k_insert_x_insert(),
        Cell::K(KCell::InsertXDelete) => k_insert_x_delete(),
        Cell::K(KCell::DeleteXDelete) => k_delete_x_delete(),
        Cell::K(KCell::ByteIdenticalFException) => k_byte_identical_exception(),
        Cell::K(KCell::DistinctDeterminants) => k_distinct_determinants(),
        Cell::C(CCell::NeedXNeed) => c_need_x_need(),
        Cell::C(CCell::NeedXSupportAdd) => c_need_x_support_add(),
        Cell::C(CCell::NeedXSupportRemove) => c_need_x_support_remove(),
        Cell::C(CCell::SupportAddXNeed) => c_support_add_x_need(),
        Cell::C(CCell::SupportAddXSupportAdd) => c_support_add_x_support_add(),
        Cell::C(CCell::SupportAddXSupportRemove) => c_support_add_x_support_remove(),
        Cell::C(CCell::SupportRemoveXNeed) => c_support_remove_x_need(),
        Cell::C(CCell::SupportRemoveXSupportAdd) => c_support_remove_x_support_add(),
        Cell::C(CCell::SupportRemoveXSupportRemove) => c_support_remove_x_support_remove(),
        Cell::W(WCell::UnitCeilingAtSlack) => w_unit_ceiling_at_slack(),
        Cell::W(WCell::UnitCeilingPastSlack) => w_unit_ceiling_past_slack(),
        Cell::W(WCell::WeightedCeilingAtSlack) => w_weighted_ceiling_at_slack(),
        Cell::W(WCell::WeightedCeilingPastSlack) => w_weighted_ceiling_past_slack(),
        Cell::W(WCell::UnitFloorAtSlack) => w_unit_floor_at_slack(),
        Cell::W(WCell::UnitFloorPastSlack) => w_unit_floor_past_slack(),
        Cell::W(WCell::WeightedFloorAtSlack) => w_weighted_floor_at_slack(),
        Cell::W(WCell::WeightedFloorPastSlack) => w_weighted_floor_past_slack(),
        Cell::W(WCell::EvaporationWithHeadroom) => w_evaporation_with_headroom(),
        Cell::W(WCell::EvaporationAtTheBound) => w_evaporation_at_the_bound(),
        Cell::W(WCell::NaivePointDeltaRefuted) => w_naive_point_delta_refuted(),
        Cell::W(WCell::ChildAddXParentAdd) => w_child_add_x_parent_add(),
        Cell::W(WCell::ChildRemoveXParentAdd) => w_child_remove_x_parent_add(),
        Cell::W(WCell::ChildDeltaXParentRemove) => w_child_delta_x_parent_remove(),
        Cell::W(WCell::ParentAddXParentAdd) => w_parent_add_x_parent_add(),
        Cell::W(WCell::ParentAddXParentRemove) => w_parent_add_x_parent_remove(),
        Cell::W(WCell::ParentRemoveXParentRemove) => w_parent_remove_x_parent_remove(),
        Cell::W(WCell::ParentRemoveXInert) => w_parent_remove_x_inert(),
        Cell::W(WCell::SpendCommuteArm) => w_spend_commute_arm(),
        Cell::W(WCell::SpendConflictArm) => w_spend_conflict_arm(),
    }
}

// --- F: same fid ---

fn f_insert_x_insert() {
    let a = [ins(TAG, vec![tag_row(1)])];
    assert_eq!(
        decide(&a, &a, &BTreeMap::new()),
        LoserDecision::Subsumed,
        "the second insert no-ops: the winner already performed the effect"
    );
    assert_commutes("f_ii", &[], &a, &a);

    let fixture = race("f_ii_w");
    let tag_braid = braid(TAG);
    fixture.planter.plant(tag_braid, &a);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(TAG, [tag_row(1)]);
            Ok(())
        })
        .expect("commit");
    assert_eq!(
        accepted_generation(&outcome),
        1,
        "the loser reports the winner's generation"
    );
    let counters = fixture.writer.counters();
    assert_eq!(counters.subsumptions, 1, "the subsumed arm resolved it");
    assert_eq!(
        counters.re_judgments, 0,
        "commute resolves without re-judgment"
    );
    assert_eq!(counters.republishes, 0);
    assert!(
        fixture.planter.slot_absent(tag_braid, 2),
        "a subsumed loser publishes nothing"
    );
}

fn f_insert_x_delete() {
    let loser = [ins(TAG, vec![tag_row(1)])];
    let winner = [del(TAG, vec![tag_row(1)])];
    assert!(
        matches!(
            decide(&loser, &winner, &BTreeMap::new()),
            LoserDecision::Conflict(ConflictCause::Fact { .. })
        ),
        "a shared fid with opposite modes is the F CONFLICT cell"
    );

    // Final presence is order-dependent: the two orders land on
    // different states, which is exactly why the cell coordinates.
    let base = [ins(TAG, vec![tag_row(1)])];
    let present = {
        let db = seeded("f_id_di", &base);
        assert!(matches!(apply_ops(&db, &winner), Admission::Accepted(_)));
        assert!(matches!(apply_ops(&db, &loser), Admission::Accepted(_)));
        state_digest(&db)
    };
    let absent = {
        let db = seeded("f_id_id", &base);
        assert!(matches!(apply_ops(&db, &loser), Admission::Accepted(_)));
        assert!(matches!(apply_ops(&db, &winner), Admission::Accepted(_)));
        state_digest(&db)
    };
    assert_ne!(present, absent, "final presence is order-dependent");

    // The loser re-judges to exactly the serial verdict: its recorded
    // ops against the winner-current state.
    let fixture = race("f_id_w");
    let tag_braid = braid(TAG);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(TAG, [tag_row(1)]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_generation(&outcome), 1);
    fixture.planter.plant(tag_braid, &winner);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(TAG, [tag_row(1), tag_row(2)]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(
        accepted_generation(&outcome),
        3,
        "re-judged and republished behind the winner"
    );
    assert_eq!(fixture.writer.counters().re_judgments, 1);
    fixture.writer.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(TAG, &tag_row(1))?);
            assert!(instance.contains_dyn(TAG, &tag_row(2))?);
            Ok(())
        })
        .expect("read");
    });
}

fn f_delete_x_insert() {
    let loser = [del(TAG, vec![tag_row(1)])];
    let winner = [ins(TAG, vec![tag_row(1)])];
    assert!(
        matches!(
            decide(&loser, &winner, &BTreeMap::new()),
            LoserDecision::Conflict(ConflictCause::Fact { .. })
        ),
        "the mirrored coordinate is the same CONFLICT cell"
    );
    // The serial verdict for the delete loser: the winner's insert
    // lands first, the delete then removes it — accepted, row absent.
    let db = seeded("f_di", &[]);
    assert!(matches!(apply_ops(&db, &winner), Admission::Accepted(_)));
    assert!(matches!(apply_ops(&db, &loser), Admission::Accepted(_)));
    db.read(|instance| {
        assert!(!instance.contains_dyn(TAG, &tag_row(1))?);
        Ok(())
    })
    .expect("read");
}

fn f_delete_x_delete() {
    let base = [ins(TAG, vec![tag_row(1)])];
    let a = [del(TAG, vec![tag_row(1)])];
    assert_eq!(
        decide(&a, &a, &BTreeMap::new()),
        LoserDecision::Subsumed,
        "same-mode shared F entries subsume"
    );
    assert_commutes("f_dd", &base, &a, &a);

    let fixture = race("f_dd_w");
    let tag_braid = braid(TAG);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(TAG, [tag_row(1)]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_generation(&outcome), 1);
    fixture.planter.plant(tag_braid, &a);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(TAG, [tag_row(1)]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(
        accepted_generation(&outcome),
        2,
        "the loser reports the winner's generation"
    );
    let counters = fixture.writer.counters();
    assert_eq!(counters.subsumptions, 1);
    assert_eq!(
        counters.re_judgments, 0,
        "commute resolves without re-judgment"
    );
    assert!(
        fixture.planter.slot_absent(tag_braid, 3),
        "a subsumed loser publishes nothing"
    );
}

// --- K: same fkey(det) under one key statement ---

fn k_insert_x_insert() {
    let loser = [ins(SLOT, vec![slot_row(5, 1)])];
    let winner = [ins(SLOT, vec![slot_row(5, 2)])];
    assert!(
        matches!(
            decide(&loser, &winner, &BTreeMap::new()),
            LoserDecision::Conflict(ConflictCause::Key {
                statement: SLOT_KEY,
                ..
            })
        ),
        "two writers of one determinant CONFLICT"
    );
    assert_second_rejects("k_ii", &[], &winner, &loser, Law::Functionality);

    // The double-booking: the loser re-judges to the FD rejection a
    // serial execution would have produced.
    let fixture = race("k_ii_w");
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
    assert_eq!(fixture.writer.counters().re_judgments, 1);
    assert!(
        fixture.planter.slot_absent(slot_braid, 2),
        "a rejected loser publishes nothing"
    );
    fixture.writer.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(SLOT, &slot_row(5, 2))?);
            assert!(!instance.contains_dyn(SLOT, &slot_row(5, 1))?);
            Ok(())
        })
        .expect("read");
    });
}

fn k_insert_x_delete() {
    let loser = [ins(SLOT, vec![slot_row(5, 1)])];
    let winner = [del(SLOT, vec![slot_row(5, 2)])];
    assert!(
        matches!(
            decide(&loser, &winner, &BTreeMap::new()),
            LoserDecision::Conflict(ConflictCause::Key {
                statement: SLOT_KEY,
                ..
            })
        ),
        "insert-vs-delete of one determinant CONFLICTS"
    );
    // The reordered visibility: after the delete the insert is legal;
    // before it, the incumbent convicts it.
    let base = [ins(SLOT, vec![slot_row(5, 2)])];
    let db = seeded("k_id_da", &base);
    assert!(matches!(apply_ops(&db, &winner), Admission::Accepted(_)));
    assert!(matches!(apply_ops(&db, &loser), Admission::Accepted(_)));
    let db = seeded("k_id_ad", &base);
    match apply_ops(&db, &loser) {
        Admission::Rejected(violations) => assert_eq!(cited(&violations), Law::Functionality),
        Admission::Accepted(_) => panic!("the incumbent convicts the early insert"),
    }
}

fn k_delete_x_delete() {
    // Op-derived, base-blind: the algebra never consults the store, so
    // two deletes of distinct rows under one determinant still meet at
    // the K key even though the FD keeps such bases unreachable.
    let loser = [del(SLOT, vec![slot_row(5, 1)])];
    let winner = [del(SLOT, vec![slot_row(5, 2)])];
    assert!(matches!(
        decide(&loser, &winner, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Key {
            statement: SLOT_KEY,
            ..
        })
    ));
}

fn k_byte_identical_exception() {
    let a = [ins(SLOT, vec![slot_row(5, 1)])];
    assert_eq!(
        decide(&a, &a, &BTreeMap::new()),
        LoserDecision::Subsumed,
        "byte-identical rows are the F table's commute case; K fires only on distinct fids"
    );
    assert_commutes("k_eq", &[], &a, &a);
}

fn k_distinct_determinants() {
    let loser = [ins(SLOT, vec![slot_row(5, 1)])];
    let winner = [ins(SLOT, vec![slot_row(6, 2)])];
    assert_eq!(
        decide(&loser, &winner, &BTreeMap::new()),
        LoserDecision::Disjoint,
        "distinct determinants never interact — the workhorse"
    );
    assert_commutes("k_dd", &[], &loser, &winner);

    // The republish arm: the loser lands its own slot behind the
    // winner with ops, footprint, and verdict untouched — L7's
    // acceptance form licenses carrying the verdict to the moved base,
    // and the re-judgment counter stays at zero.
    let fixture = race("k_dd_w");
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
        accepted_generation(&outcome),
        2,
        "republished into its own slot"
    );
    let counters = fixture.writer.counters();
    assert_eq!(
        counters.disjoint_verdicts, 1,
        "the strict verdict is computed"
    );
    assert_eq!(counters.republishes, 1);
    assert_eq!(
        counters.re_judgments, 0,
        "the commute cell resolves without re-judgment"
    );
    fixture.writer.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(SLOT, &slot_row(5, 1))?);
            assert!(instance.contains_dyn(SLOT, &slot_row(6, 2))?);
            Ok(())
        })
        .expect("read");
    });
}

// --- C: same fkey(target det) under one containment ---

/// A shared containment key of ANY mode pair re-judges under strict
/// disjointness — L6's hypothesis excludes shared commute-cell keys —
/// so every C cell pins the intersect verdict as Conflict and the
/// commute cells pin the commute itself with the order test.
fn assert_c_conflict(loser: &[Op], winner: &[Op]) {
    assert!(matches!(
        decide(loser, winner, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Containment {
            statement: ENTRY_IN_ACCOUNT,
            ..
        })
    ));
}

fn c_need_x_need() {
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let a = [ins(ENTRY, vec![entry_row(7, "x")])];
    let b = [ins(ENTRY, vec![entry_row(7, "y")])];
    assert_c_conflict(&a, &b);
    assert_commutes("c_nn", &base, &a, &b);
}

fn c_need_x_support_add() {
    // The keyed target makes a live support add base-redundant: the
    // reinsert still carries the support+ entry (footprints are
    // op-derived), and the pair commutes.
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let a = [ins(ENTRY, vec![entry_row(7, "x")])];
    let b = [ins(ACCOUNT, vec![account_row(7, "base")])];
    assert_c_conflict(&a, &b);
    assert_commutes("c_ns", &base, &a, &b);
}

fn c_need_x_support_remove() {
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let need = [ins(ENTRY, vec![entry_row(7, "x")])];
    let remove = [del(ACCOUNT, vec![account_row(7, "base")])];
    assert_c_conflict(&need, &remove);
    // The dangling-reference race, per order: whoever runs second is
    // the one the serial verdict rejects.
    assert_second_rejects("c_nsr_rn", &base, &remove, &need, Law::Containment);
    assert_second_rejects("c_nsr_nr", &base, &need, &remove, Law::Containment);

    // The need loser re-judges to the dangling-reference rejection.
    let fixture = race("c_nsr_w");
    let account_braid = braid(ACCOUNT);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(ACCOUNT, [account_row(7, "base")]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_generation(&outcome), 1);
    fixture.planter.plant(account_braid, &remove);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(ENTRY, [entry_row(7, "x")]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(rejected_law(&outcome), Law::Containment);
    assert_eq!(fixture.writer.counters().re_judgments, 1);
    assert!(fixture.planter.slot_absent(account_braid, 3));
}

fn c_support_add_x_need() {
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let a = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let b = [ins(ENTRY, vec![entry_row(7, "x")])];
    assert_c_conflict(&a, &b);
    assert_commutes("c_sn", &base, &a, &b);
}

fn c_support_add_x_support_add() {
    // Byte-identical establishing rows are the F table's commute case,
    // lifted through the loser algebra as subsumption.
    let same = [ins(ACCOUNT, vec![account_row(7, "base")])];
    assert_eq!(
        decide(&same, &same, &BTreeMap::new()),
        LoserDecision::Subsumed
    );
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    assert_commutes("c_ss_eq", &base, &same, &same);

    // Distinct establishing rows of one keyed group are the K matrix's
    // double-mint, and the K coordinate fires first in the scan.
    let a = [ins(ACCOUNT, vec![account_row(7, "x")])];
    let b = [ins(ACCOUNT, vec![account_row(7, "y")])];
    assert!(matches!(
        decide(&a, &b, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Key {
            statement: ACCOUNT_KEY,
            ..
        })
    ));
    assert_second_rejects("c_ss_ne", &[], &a, &b, Law::Functionality);
}

fn c_support_add_x_support_remove() {
    // The add only strengthens the remover's premise — and under a
    // keyed target a distinct concurrent add is a replacement race the
    // K matrix already owns, so the C cell's freedom never reaches the
    // strict loser algebra alone.
    let add = [ins(ACCOUNT, vec![account_row(7, "y")])];
    let remove = [del(ACCOUNT, vec![account_row(7, "base")])];
    assert!(matches!(
        decide(&add, &remove, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Key {
            statement: ACCOUNT_KEY,
            ..
        })
    ));
    // Serially the remove clears the incumbent and the add lands; the
    // early add is convicted by the incumbent — reordered visibility.
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let db = seeded("c_ssr_ra", &base);
    assert!(matches!(apply_ops(&db, &remove), Admission::Accepted(_)));
    assert!(matches!(apply_ops(&db, &add), Admission::Accepted(_)));
    let db = seeded("c_ssr_ar", &base);
    match apply_ops(&db, &add) {
        Admission::Rejected(violations) => assert_eq!(cited(&violations), Law::Functionality),
        Admission::Accepted(_) => panic!("the incumbent convicts the early add"),
    }
}

fn c_support_remove_x_need() {
    let remove = [del(ACCOUNT, vec![account_row(7, "base")])];
    let need = [ins(ENTRY, vec![entry_row(7, "x")])];
    assert_c_conflict(&remove, &need);

    // The support− loser re-judges to the dangling-reference
    // rejection: the winner's source now pins the target it removes.
    let fixture = race("c_srn_w");
    let account_braid = braid(ACCOUNT);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(ACCOUNT, [account_row(7, "base")]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_generation(&outcome), 1);
    fixture.planter.plant(account_braid, &need);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(ACCOUNT, [account_row(7, "base")]);
            Ok(())
        })
        .expect("loser commit");
    assert_eq!(rejected_law(&outcome), Law::Containment);
    assert_eq!(fixture.writer.counters().re_judgments, 1);
    assert!(fixture.planter.slot_absent(account_braid, 3));
}

fn c_support_remove_x_support_add() {
    // The mirrored replacement race: the remover as loser against a
    // distinct concurrent add meets the K coordinate first.
    let remove = [del(ACCOUNT, vec![account_row(7, "base")])];
    let add = [ins(ACCOUNT, vec![account_row(7, "y")])];
    assert!(matches!(
        decide(&remove, &add, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Key {
            statement: ACCOUNT_KEY,
            ..
        })
    ));
}

fn c_support_remove_x_support_remove() {
    // Each remover would count the other's row as the survivor. The
    // keyed target holds one row per group, so the reachable engine
    // instance is two deletes of one fid — the F table's commute case,
    // subsumed — while the op-space coordinate with distinct fids
    // still refuses at the shared determinant.
    let base = [ins(ACCOUNT, vec![account_row(7, "base")])];
    let same = [del(ACCOUNT, vec![account_row(7, "base")])];
    assert_eq!(
        decide(&same, &same, &BTreeMap::new()),
        LoserDecision::Subsumed
    );
    assert_commutes("c_srsr_eq", &base, &same, &same);

    let a = [del(ACCOUNT, vec![account_row(7, "x")])];
    let b = [del(ACCOUNT, vec![account_row(7, "y")])];
    assert!(matches!(
        decide(&a, &b, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Key {
            statement: ACCOUNT_KEY,
            ..
        })
    ));
}

// --- W: same fkey(parent det) under one capacity ---

fn unit_base(pool: u64, count: u64) -> Vec<Op> {
    let children = (0..count).map(|tag| unit_row(pool, tag)).collect();
    vec![
        ins(POOL, vec![pool_row(pool, 0)]),
        ins(UNIT_CHILD, children),
    ]
}

fn w_unit_ceiling_at_slack() {
    // measure 2, ceiling 6: slack is 4 and the joint worst case is 4.
    let base = unit_base(1, 2);
    let a = [ins(UNIT_CHILD, vec![unit_row(1, 10), unit_row(1, 11)])];
    let b = [ins(UNIT_CHILD, vec![unit_row(1, 12), unit_row(1, 13)])];
    let measures = shared_measures(&a, &b, 2, 0, Some(UNIT_CEILING));
    assert_eq!(
        decide(&a, &b, &measures),
        LoserDecision::Disjoint,
        "endpoints exactly at slack commute"
    );
    assert_commutes("w_ucs", &base, &a, &b);
}

fn w_unit_ceiling_past_slack() {
    // measure 2, ceiling 6: slack 4, joint worst case 5 = slack + 1.
    let base = unit_base(1, 2);
    let a = [ins(UNIT_CHILD, vec![unit_row(1, 10), unit_row(1, 11)])];
    let b = [ins(
        UNIT_CHILD,
        vec![unit_row(1, 12), unit_row(1, 13), unit_row(1, 14)],
    )];
    let measures = shared_measures(&a, &b, 2, 0, Some(UNIT_CEILING));
    assert!(matches!(
        decide(&b, &a, &measures),
        LoserDecision::Conflict(ConflictCause::CapacityInterval {
            statement: UNIT_CAPACITY,
            ..
        })
    ));
    assert_second_rejects("w_ucp_ab", &base, &a, &b, Law::Capacity);
    assert_second_rejects("w_ucp_ba", &base, &b, &a, Law::Capacity);

    // The capacity loser re-judges to the serial rejection against the
    // ceiling.
    let fixture = race("w_ucp_w");
    let pool_braid = braid(POOL);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(POOL, [pool_row(1, 0)]);
            batch.insert(UNIT_CHILD, [unit_row(1, 0), unit_row(1, 1)]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_generation(&outcome), 1);
    fixture.planter.plant(pool_braid, &a);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(
                UNIT_CHILD,
                [unit_row(1, 12), unit_row(1, 13), unit_row(1, 14)],
            );
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

fn w_weighted_ceiling_at_slack() {
    // measure 4, ceiling 10: slack 6, two weighted inserts of 3.
    let base = vec![
        ins(POOL, vec![pool_row(2, 0)]),
        ins(RES, vec![res_row(2, 4, 11)]),
    ];
    let a = [ins(RES, vec![res_row(2, 3, 12)])];
    let b = [ins(RES, vec![res_row(2, 3, 13)])];
    let measures = shared_measures(&a, &b, 4, 0, Some(RES_CEILING));
    assert_eq!(decide(&a, &b, &measures), LoserDecision::Disjoint);
    assert_commutes("w_wcs", &base, &a, &b);
}

fn w_weighted_ceiling_past_slack() {
    // measure 5, ceiling 10: slack 5, joint worst case 6 = slack + 1.
    let base = vec![
        ins(POOL, vec![pool_row(2, 0)]),
        ins(RES, vec![res_row(2, 5, 11)]),
    ];
    let a = [ins(RES, vec![res_row(2, 3, 12)])];
    let b = [ins(RES, vec![res_row(2, 3, 13)])];
    let measures = shared_measures(&a, &b, 5, 0, Some(RES_CEILING));
    assert!(matches!(
        decide(&a, &b, &measures),
        LoserDecision::Conflict(ConflictCause::CapacityInterval {
            statement: RES_CAPACITY,
            ..
        })
    ));
    assert_second_rejects("w_wcp_ab", &base, &a, &b, Law::Capacity);
    assert_second_rejects("w_wcp_ba", &base, &b, &a, Law::Capacity);
}

fn coin_base(vault: u64, count: u64) -> Vec<Op> {
    let coins = (0..count).map(|tag| coin_row(vault, tag)).collect();
    vec![ins(VAULT, vec![vault_row(vault)]), ins(COIN, coins)]
}

fn w_unit_floor_at_slack() {
    // measure 4, floor 2: downward slack 2, two unit deletes.
    let base = coin_base(1, 4);
    let a = [del(COIN, vec![coin_row(1, 0)])];
    let b = [del(COIN, vec![coin_row(1, 1)])];
    let measures = shared_measures(&a, &b, 4, COIN_FLOOR, Some(100));
    assert_eq!(decide(&a, &b, &measures), LoserDecision::Disjoint);
    assert_commutes("w_ufs", &base, &a, &b);
}

fn w_unit_floor_past_slack() {
    // measure 3, floor 2: downward slack 1, joint worst case 2.
    let base = coin_base(1, 3);
    let a = [del(COIN, vec![coin_row(1, 0)])];
    let b = [del(COIN, vec![coin_row(1, 1)])];
    let measures = shared_measures(&a, &b, 3, COIN_FLOOR, Some(100));
    assert!(matches!(
        decide(&a, &b, &measures),
        LoserDecision::Conflict(ConflictCause::CapacityInterval {
            statement: COIN_CAPACITY,
            ..
        })
    ));
    assert_second_rejects("w_ufp_ab", &base, &a, &b, Law::Capacity);
    assert_second_rejects("w_ufp_ba", &base, &b, &a, Law::Capacity);

    // The floor is a real engine bound, not only a supplied measure:
    // the loser re-judges to the serial floor rejection.
    let fixture = race("w_ufp_w");
    let vault_braid = braid(VAULT);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(VAULT, [vault_row(1)]);
            batch.insert(COIN, [coin_row(1, 0), coin_row(1, 1), coin_row(1, 2)]);
            Ok(())
        })
        .expect("base commit");
    assert_eq!(accepted_generation(&outcome), 1);
    fixture.planter.plant(vault_braid, &a);
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

fn w_weighted_floor_at_slack() {
    // Weighted deletes against a supplied floor: measure 5, floor 0,
    // joint minimum exactly at the downward slack.
    let base = vec![
        ins(POOL, vec![pool_row(3, 0)]),
        ins(RES, vec![res_row(3, 3, 21), res_row(3, 2, 22)]),
    ];
    let a = [del(RES, vec![res_row(3, 3, 21)])];
    let b = [del(RES, vec![res_row(3, 2, 22)])];
    let measures = shared_measures(&a, &b, 5, 0, Some(RES_CEILING));
    assert_eq!(decide(&a, &b, &measures), LoserDecision::Disjoint);
    assert_commutes("w_wfs", &base, &a, &b);
}

fn w_weighted_floor_past_slack() {
    // The same pair one past the bound: floor 1 leaves downward slack
    // 4 against a joint minimum of 5.
    let a = [del(RES, vec![res_row(3, 3, 21)])];
    let b = [del(RES, vec![res_row(3, 2, 22)])];
    let measures = shared_measures(&a, &b, 5, 1, Some(RES_CEILING));
    assert!(matches!(
        decide(&a, &b, &measures),
        LoserDecision::Conflict(ConflictCause::CapacityInterval {
            statement: RES_CAPACITY,
            ..
        })
    ));
}

fn w_evaporation_with_headroom() {
    // Both batches delete one shared row and insert their own: the
    // intervals widen on both sides, and with headroom on both bounds
    // the W cell commutes — while the pair-level decision still
    // re-judges on the shared fid, the strictness L6 demands.
    let base = vec![
        ins(POOL, vec![pool_row(8, 0)]),
        ins(RES, vec![res_row(8, 2, 0), res_row(8, 2, 9)]),
    ];
    let loser = [
        del(RES, vec![res_row(8, 2, 0)]),
        ins(RES, vec![res_row(8, 2, 1)]),
    ];
    let winner = [
        del(RES, vec![res_row(8, 2, 0)]),
        ins(RES, vec![res_row(8, 1, 2)]),
    ];
    let base_measure = BaseMeasure {
        measure: 4,
        floor: 0,
        ceiling: Some(RES_CEILING),
    };
    assert_eq!(
        capacity_cell(
            &profile_at(&loser, RES_CAPACITY),
            &profile_at(&winner, RES_CAPACITY),
            Some(&base_measure),
        ),
        CapacityCell::Commutes,
        "widened endpoints inside both bounds commute"
    );
    assert!(matches!(
        decide(
            &loser,
            &winner,
            &shared_measures(&loser, &winner, 4, 0, Some(RES_CEILING))
        ),
        LoserDecision::Conflict(ConflictCause::Fact { .. })
    ));
    assert_commutes("w_evh", &base, &loser, &winner);
}

fn evaporation_bound_fixture() -> (Vec<Op>, [Op; 2], [Op; 2], BaseMeasure) {
    // measure 9, ceiling 10. The loser publishes delta +1 but its
    // delete can evaporate against the winner, lifting the effective
    // delta to +3; the winner nets −1.
    let base = vec![
        ins(POOL, vec![pool_row(8, 0)]),
        ins(RES, vec![res_row(8, 2, 0), res_row(8, 7, 9)]),
    ];
    let loser = [
        del(RES, vec![res_row(8, 2, 0)]),
        ins(RES, vec![res_row(8, 3, 1)]),
    ];
    let winner = [
        del(RES, vec![res_row(8, 2, 0)]),
        ins(RES, vec![res_row(8, 1, 2)]),
    ];
    let base_measure = BaseMeasure {
        measure: 9,
        floor: 0,
        ceiling: Some(RES_CEILING),
    };
    (base, loser, winner, base_measure)
}

fn w_evaporation_at_the_bound() {
    let (base, loser, winner, base_measure) = evaporation_bound_fixture();
    assert_eq!(
        capacity_cell(
            &profile_at(&loser, RES_CAPACITY),
            &profile_at(&winner, RES_CAPACITY),
            Some(&base_measure),
        ),
        CapacityCell::IntervalExceeded,
        "the evaporation-widened maximum breaks the ceiling slack"
    );
    // The serial truth the interval law protects: after the winner,
    // the loser's delete evaporates and its insert overshoots.
    let db = seeded("w_evb", &base);
    assert!(matches!(apply_ops(&db, &winner), Admission::Accepted(_)));
    match apply_ops(&db, &loser) {
        Admission::Rejected(violations) => assert_eq!(cited(&violations), Law::Capacity),
        Admission::Accepted(_) => panic!("the evaporated delete overshoots the ceiling"),
    }
}

fn w_naive_point_delta_refuted() {
    // The negative fixture that keeps the interval law honest: a
    // point-delta oracle reads the published sums (+1 and −1), finds
    // them inside the slack, and would republish a batch whose replay
    // rejects on every store. The interval test refuses exactly here.
    let (base, loser, winner, base_measure) = evaporation_bound_fixture();
    let loser_profile = profile_at(&loser, RES_CAPACITY);
    let winner_profile = profile_at(&winner, RES_CAPACITY);
    assert!(
        naive_point_delta_commutes(loser_profile, winner_profile, base_measure),
        "the naive oracle waves the pair through"
    );
    assert_eq!(
        capacity_cell(&loser_profile, &winner_profile, Some(&base_measure)),
        CapacityCell::IntervalExceeded,
        "the interval law refuses where the point oracle passes"
    );
    let db = seeded("w_npd", &base);
    assert!(matches!(apply_ops(&db, &winner), Admission::Accepted(_)));
    assert!(
        matches!(apply_ops(&db, &loser), Admission::Rejected(_)),
        "the serial verdict convicts the naive oracle"
    );
}

fn w_child_add_x_parent_add() {
    // The keyed parent makes a live parent add base-redundant; the
    // reinsert still carries parent+ in the op-derived footprint, and
    // a parent add commutes with child adds.
    let base = [ins(POOL, vec![pool_row(3, 0)])];
    let child = [ins(UNIT_CHILD, vec![unit_row(3, 9)])];
    let parent = [ins(POOL, vec![pool_row(3, 0)])];
    let measures = shared_measures(&child, &parent, 0, 0, Some(UNIT_CEILING));
    assert_eq!(
        decide(&child, &parent, &measures),
        LoserDecision::Disjoint,
        "a parent add commutes with child adds"
    );
    assert_commutes("w_cap", &base, &child, &parent);
}

fn w_child_remove_x_parent_add() {
    let base = vec![
        ins(POOL, vec![pool_row(3, 0)]),
        ins(UNIT_CHILD, vec![unit_row(3, 9)]),
    ];
    let child_remove = [del(UNIT_CHILD, vec![unit_row(3, 9)])];
    let parent_add = [ins(POOL, vec![pool_row(3, 0)])];
    let measures = shared_measures(&child_remove, &parent_add, 1, 0, Some(UNIT_CEILING));
    assert!(matches!(
        decide(&child_remove, &parent_add, &measures),
        LoserDecision::Conflict(ConflictCause::CapacityParent {
            statement: UNIT_CAPACITY,
            ..
        })
    ));
    // A deliberately conservative cell: this instance's serial
    // verdicts agree, and the cheap re-judgment settles it.
    assert_commutes("w_crp", &base, &child_remove, &parent_add);
}

fn w_child_delta_x_parent_remove() {
    let base = [ins(POOL, vec![pool_row(2, 0)])];
    let child = [ins(UNIT_CHILD, vec![unit_row(2, 9)])];
    let parent_remove = [del(POOL, vec![pool_row(2, 0)])];
    let measures = shared_measures(&child, &parent_remove, 0, 0, Some(UNIT_CEILING));
    for (loser, winner) in [
        (&child[..], &parent_remove[..]),
        (&parent_remove[..], &child[..]),
    ] {
        assert!(matches!(
            decide(loser, winner, &measures),
            LoserDecision::Conflict(ConflictCause::CapacityParent {
                statement: UNIT_CAPACITY,
                ..
            })
        ));
    }
    // The verdict flip that makes this cell necessary: the group's
    // bound holds only while its parent row exists, so an over-ceiling
    // child batch is refused with the parent present and admitted
    // vacuously once the removal lands first.
    let over: Vec<Box<[Value]>> = (10..17).map(|tag| unit_row(2, tag)).collect();
    let over_ceiling = [ins(UNIT_CHILD, over)];
    let db = seeded("w_cpr_cp", &base);
    match apply_ops(&db, &over_ceiling) {
        Admission::Rejected(violations) => assert_eq!(cited(&violations), Law::Capacity),
        Admission::Accepted(_) => panic!("the ceiling refuses while the parent stands"),
    }
    assert!(matches!(
        apply_ops(&db, &parent_remove),
        Admission::Accepted(_)
    ));
    assert!(
        matches!(apply_ops(&db, &over_ceiling), Admission::Accepted(_)),
        "the removal first makes the same batch admissible — order flips the verdict"
    );
}

fn w_parent_add_x_parent_add() {
    // The matrix cell itself: two parent adds race, whatever their
    // rows. The keyed parent routes distinct rows through K first (the
    // double-mint) and byte-identical adds through subsumption, so the
    // ParentRace posture is pinned on the cell function directly.
    let adding = CapacityProfile {
        parent_add: true,
        ..CapacityProfile::default()
    };
    assert_eq!(
        capacity_cell(&adding, &adding, None),
        CapacityCell::ParentRace
    );
    let a = [ins(POOL, vec![pool_row(3, 1)])];
    let b = [ins(POOL, vec![pool_row(3, 2)])];
    assert!(matches!(
        decide(&a, &b, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Key {
            statement: POOL_KEY,
            ..
        })
    ));
    assert_second_rejects("w_pp", &[], &a, &b, Law::Functionality);
}

fn w_parent_add_x_parent_remove() {
    let adding = CapacityProfile {
        parent_add: true,
        ..CapacityProfile::default()
    };
    let removing = CapacityProfile {
        parent_remove: true,
        ..CapacityProfile::default()
    };
    assert_eq!(
        capacity_cell(&adding, &removing, None),
        CapacityCell::ParentRace
    );
    // The reachable instance under a keyed parent is one row added
    // and removed — the F table's insert-x-delete order dependence.
    let base = [ins(POOL, vec![pool_row(3, 0)])];
    let add = [ins(POOL, vec![pool_row(3, 0)])];
    let remove = [del(POOL, vec![pool_row(3, 0)])];
    assert!(matches!(
        decide(&add, &remove, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::Fact { .. })
    ));
    let present = {
        let db = seeded("w_ppr_ra", &base);
        assert!(matches!(apply_ops(&db, &remove), Admission::Accepted(_)));
        assert!(matches!(apply_ops(&db, &add), Admission::Accepted(_)));
        state_digest(&db)
    };
    let absent = {
        let db = seeded("w_ppr_ar", &base);
        assert!(matches!(apply_ops(&db, &add), Admission::Accepted(_)));
        assert!(matches!(apply_ops(&db, &remove), Admission::Accepted(_)));
        state_digest(&db)
    };
    assert_ne!(present, absent, "parent presence is order-dependent");
}

fn w_parent_remove_x_parent_remove() {
    let removing = CapacityProfile {
        parent_remove: true,
        ..CapacityProfile::default()
    };
    assert_eq!(
        capacity_cell(&removing, &removing, None),
        CapacityCell::ParentRace
    );
    // The keyed parent holds one row per group: two removers share
    // its fid and the F table's delete-x-delete subsumption answers.
    let base = [ins(POOL, vec![pool_row(4, 0)])];
    let same = [del(POOL, vec![pool_row(4, 0)])];
    assert_eq!(
        decide(&same, &same, &BTreeMap::new()),
        LoserDecision::Subsumed
    );
    assert_commutes("w_prpr", &base, &same, &same);
}

fn w_parent_remove_x_inert() {
    // The one posture a parent removal commutes with: the exact point
    // interval [0, 0] and no parent moves on the other side.
    let removal = CapacityProfile {
        parent_remove: true,
        ..CapacityProfile::default()
    };
    let inert = CapacityProfile::default();
    assert!(inert.is_inert());
    assert_eq!(
        capacity_cell(&removal, &inert, None),
        CapacityCell::Commutes
    );
    assert_eq!(
        capacity_cell(&inert, &removal, None),
        CapacityCell::Commutes
    );
}

fn w_spend_commute_arm() {
    // Spend = delete-reservation + insert children in one commit: net
    // delta 0 with the evaporation interval around it. With headroom
    // beyond the spend's own units the fast path is the matrix's own
    // arithmetic — the W interval test passes and the loss republishes.
    let spend = [
        del(RES, vec![res_row(9, 3, 99)]),
        ins(RES, vec![res_row(9, 3, 0)]),
    ];
    let winner = [ins(RES, vec![res_row(9, 2, 50)])];
    let measures = shared_measures(&spend, &winner, 3, 0, Some(RES_CEILING));
    assert_eq!(
        decide(&spend, &winner, &measures),
        LoserDecision::Disjoint,
        "the spend commutes by the W interval test"
    );

    let fixture = race("w_spend_c");
    let pool_braid = braid(POOL);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(POOL, [pool_row(9, 0)]);
            Ok(())
        })
        .expect("pool commit");
    assert_eq!(accepted_generation(&outcome), 1);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.reserve_capacity(RES_CAPACITY, &[Value::U64(9)], 3, 99)?;
            Ok(())
        })
        .expect("mint commit");
    assert_eq!(accepted_generation(&outcome), 2);
    fixture.planter.plant(pool_braid, &winner);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.delete(RES, [res_row(9, 3, 99)]);
            batch.insert(RES, [res_row(9, 3, 0)]);
            Ok(())
        })
        .expect("spend commit");
    assert_eq!(
        accepted_generation(&outcome),
        4,
        "the spend republishes behind the winner"
    );
    fixture.writer.with_db(|db| {
        db.read(|instance| {
            assert!(!instance.contains_dyn(RES, &res_row(9, 3, 99))?);
            assert!(instance.contains_dyn(RES, &res_row(9, 3, 0))?);
            assert!(instance.contains_dyn(RES, &res_row(9, 2, 50))?);
            Ok(())
        })
        .expect("read");
    });
}

fn w_spend_conflict_arm() {
    // Reclaim-vs-spend: two deletes of one reservation row carry
    // correlated intervals the test reads uncorrelated, so at the
    // bound it goes CONFLICT and forces the honest re-judgment.
    let spend = [
        del(RES, vec![res_row(9, 3, 99)]),
        ins(RES, vec![res_row(9, 3, 0)]),
    ];
    let reclaim = [del(RES, vec![res_row(9, 3, 99)])];
    let at_bound = BaseMeasure {
        measure: RES_CEILING,
        floor: 0,
        ceiling: Some(RES_CEILING),
    };
    assert_eq!(
        capacity_cell(
            &profile_at(&spend, RES_CAPACITY),
            &profile_at(&reclaim, RES_CAPACITY),
            Some(&at_bound),
        ),
        CapacityCell::IntervalExceeded
    );
    assert!(matches!(
        decide(
            &spend,
            &reclaim,
            &shared_measures(&spend, &reclaim, RES_CEILING, 0, Some(RES_CEILING)),
        ),
        LoserDecision::Conflict(ConflictCause::Fact { .. })
    ));

    // The winner reclaims the hold and re-mints it elsewhere; the
    // loser's spend re-judges with its delete evaporated and its
    // children priced against the real slack — the serial rejection.
    let fixture = race("w_spend_r");
    let pool_braid = braid(POOL);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(POOL, [pool_row(9, 0)]);
            Ok(())
        })
        .expect("pool commit");
    assert_eq!(accepted_generation(&outcome), 1);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.reserve_capacity(RES_CAPACITY, &[Value::U64(9)], 3, 99)?;
            Ok(())
        })
        .expect("mint commit");
    assert_eq!(accepted_generation(&outcome), 2);
    let outcome = fixture
        .writer
        .commit(|batch| {
            batch.insert(RES, [res_row(9, 7, 50)]);
            Ok(())
        })
        .expect("fill commit");
    assert_eq!(accepted_generation(&outcome), 3);
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

// --- the roster ---

#[test]
fn the_fixture_table_is_an_exhaustive_match_over_the_footprint_classes() {
    let samples = [
        Entry::Fact {
            fid: [0u8; 32],
            mode: OpKind::Insert,
        },
        Entry::Key {
            statement: SLOT_KEY,
            key: [0u8; 32],
        },
        Entry::Containment {
            statement: ENTRY_IN_ACCOUNT,
            key: [0u8; 32],
            mode: ContainmentMode::Need,
        },
        Entry::Capacity {
            statement: UNIT_CAPACITY,
            key: [0u8; 32],
            mode: CapacityMode::ParentAdd,
        },
    ];
    let mut all: Vec<Cell> = Vec::new();
    for sample in &samples {
        all.extend(cells_of(sample));
    }
    let mut sorted = all.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), all.len(), "each cell appears exactly once");
    for cell in all {
        check(cell);
    }
}

// --- F: same fid ---

#[test]
fn f_matrix_insert_x_insert_commutes_second_no_ops_subsumed_without_re_judgment() {
    check(Cell::F(FCell::InsertXInsert));
}

#[test]
fn f_matrix_insert_x_delete_conflicts_final_presence_is_order_dependent() {
    check(Cell::F(FCell::InsertXDelete));
}

#[test]
fn f_matrix_delete_x_insert_conflicts_and_re_judges_to_the_serial_state() {
    check(Cell::F(FCell::DeleteXInsert));
}

#[test]
fn f_matrix_delete_x_delete_commutes_subsumed_without_re_judgment() {
    check(Cell::F(FCell::DeleteXDelete));
}

// --- K: same fkey(det) ---

#[test]
fn k_matrix_insert_x_insert_one_determinant_conflicts_with_the_fd_rejection() {
    check(Cell::K(KCell::InsertXInsert));
}

#[test]
fn k_matrix_insert_x_delete_one_determinant_conflicts_reordering_visibility() {
    check(Cell::K(KCell::InsertXDelete));
}

#[test]
fn k_matrix_delete_x_delete_one_determinant_conflicts_by_op_derivation() {
    check(Cell::K(KCell::DeleteXDelete));
}

#[test]
fn k_matrix_byte_identical_rows_land_in_the_f_commute_cell() {
    check(Cell::K(KCell::ByteIdenticalFException));
}

#[test]
fn k_matrix_distinct_determinants_never_interact_republish_arm() {
    check(Cell::K(KCell::DistinctDeterminants));
}

// --- C: same fkey(target det) ---

#[test]
fn c_matrix_need_x_need_commutes() {
    check(Cell::C(CCell::NeedXNeed));
}

#[test]
fn c_matrix_need_x_support_add_commutes() {
    check(Cell::C(CCell::NeedXSupportAdd));
}

#[test]
fn c_matrix_need_x_support_remove_conflicts_dangling_reference() {
    check(Cell::C(CCell::NeedXSupportRemove));
}

#[test]
fn c_matrix_support_add_x_need_commutes() {
    check(Cell::C(CCell::SupportAddXNeed));
}

#[test]
fn c_matrix_support_add_x_support_add_commutes() {
    check(Cell::C(CCell::SupportAddXSupportAdd));
}

#[test]
fn c_matrix_support_add_x_support_remove_commutes_add_strengthens_the_premise() {
    check(Cell::C(CCell::SupportAddXSupportRemove));
}

#[test]
fn c_matrix_support_remove_x_need_conflicts_dangling_reference() {
    check(Cell::C(CCell::SupportRemoveXNeed));
}

#[test]
fn c_matrix_support_remove_x_support_add_commutes() {
    check(Cell::C(CCell::SupportRemoveXSupportAdd));
}

#[test]
fn c_matrix_support_remove_x_support_remove_conflicts_each_counted_the_other_survivor() {
    check(Cell::C(CCell::SupportRemoveXSupportRemove));
}

// --- W: same fkey(parent det) ---

#[test]
fn w_matrix_child_delta_x_child_delta_unit_ceiling_at_slack_commutes() {
    check(Cell::W(WCell::UnitCeilingAtSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_unit_ceiling_slack_plus_one_conflicts() {
    check(Cell::W(WCell::UnitCeilingPastSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_weighted_ceiling_at_slack_commutes() {
    check(Cell::W(WCell::WeightedCeilingAtSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_weighted_ceiling_slack_plus_one_conflicts() {
    check(Cell::W(WCell::WeightedCeilingPastSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_unit_floor_at_slack_commutes() {
    check(Cell::W(WCell::UnitFloorAtSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_unit_floor_slack_plus_one_conflicts() {
    check(Cell::W(WCell::UnitFloorPastSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_weighted_floor_at_slack_commutes() {
    check(Cell::W(WCell::WeightedFloorAtSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_weighted_floor_slack_plus_one_conflicts() {
    check(Cell::W(WCell::WeightedFloorPastSlack));
}

#[test]
fn w_matrix_child_delta_x_child_delta_evaporation_commutes_with_headroom() {
    check(Cell::W(WCell::EvaporationWithHeadroom));
}

#[test]
fn w_matrix_child_delta_x_child_delta_evaporation_conflicts_at_the_bound() {
    check(Cell::W(WCell::EvaporationAtTheBound));
}

#[test]
fn w_matrix_child_delta_x_child_delta_a_naive_point_delta_oracle_fails_the_evaporation_cell() {
    check(Cell::W(WCell::NaivePointDeltaRefuted));
}

#[test]
fn w_matrix_child_add_x_parent_add_commutes() {
    check(Cell::W(WCell::ChildAddXParentAdd));
}

#[test]
fn w_matrix_child_remove_x_parent_add_conflicts() {
    check(Cell::W(WCell::ChildRemoveXParentAdd));
}

#[test]
fn w_matrix_child_delta_x_parent_remove_conflicts_both_orders() {
    check(Cell::W(WCell::ChildDeltaXParentRemove));
}

#[test]
fn w_matrix_parent_add_x_parent_add_conflicts() {
    check(Cell::W(WCell::ParentAddXParentAdd));
}

#[test]
fn w_matrix_parent_add_x_parent_remove_conflicts() {
    check(Cell::W(WCell::ParentAddXParentRemove));
}

#[test]
fn w_matrix_parent_remove_x_parent_remove_conflicts() {
    check(Cell::W(WCell::ParentRemoveXParentRemove));
}

#[test]
fn w_matrix_parent_remove_x_inert_other_side_commutes() {
    check(Cell::W(WCell::ParentRemoveXInert));
}

#[test]
fn w_matrix_reservation_spend_commute_arm_the_interval_test_is_the_fast_path() {
    check(Cell::W(WCell::SpendCommuteArm));
}

#[test]
fn w_matrix_reservation_spend_conflict_arm_re_judgment_prices_the_children() {
    check(Cell::W(WCell::SpendConflictArm));
}
