//! The staleness pin's closed-relation leg (finding 005): build pins a
//! kept closed occurrence at its sealed extension's length
//! (`plan/selectivity`'s `relation_rows`), and `staleness` reads the
//! live side through the same route — the raw `S` counter never exists
//! for a storage-virtual relation and reads 0, which would render as a
//! permanent phantom drift no reprepare could clear.

use super::*;
use crate::api::db::Db;
use bumbledb_theory::schema::{Row, Side, StatementDescriptor};

const READING: RelationId = RelationId(0);
const KIND: RelationId = RelationId(1);

/// Reading(kind u64, value i64) referencing the closed Kind(rank u64;
/// four rows) through Reading(kind) <= Kind(id) — `folded.rs`'s shape,
/// minus the fresh key (the dyn write surface seeds rows directly).
fn closed_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Reading".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "value".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "A".into(),
                        values: Box::new([Value::U64(10)]),
                    },
                    Row {
                        handle: "B".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "C".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "D".into(),
                        values: Box::new([Value::U64(30)]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![FieldDescriptor {
                    name: "rank".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: READING,
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: KIND,
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
}

/// `Q(v, r) :- Reading(kind = x, value = v), Kind(id = x, rank = r)` —
/// the rank escapes to the head, one of the four kept shapes
/// (`plan/ground/evaluate.rs`), so the closed occurrence survives the
/// grounding fold, enters DP, and earns a pin.
fn kept_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(READING),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(KIND),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// The kept closed occurrence pins ratio 1.0, fresh and after ordinary
/// growth: pinned and live both read the sealed extension. Reading the
/// raw counter instead would report pinned = 4 against live = 0 — a
/// phantom max ratio of 4 on a plan with zero actual drift.
#[test]
fn a_kept_closed_occurrence_never_reads_as_drift() {
    let dir = TempDir::new("staleness-closed");
    let db = Db::create(dir.path(), closed_descriptor()).expect("create");
    db.write(|tx| {
        for (kind, value) in [(0u64, 100i64), (1, 210), (2, 220)] {
            tx.insert_dyn(READING, &[Value::U64(kind), Value::I64(value)])?;
        }
        Ok(())
    })
    .expect("seed readings");

    let prepared = db.prepare(&kept_query()).expect("prepare");
    db.read(|snap| {
        let staleness = prepared.staleness(snap)?;
        assert_eq!(
            staleness.per_occurrence.len(),
            2,
            "both occurrences participate and pin: {staleness:?}"
        );
        let kind = staleness
            .per_occurrence
            .iter()
            .find(|d| d.relation == KIND)
            .expect("the kept closed occurrence is pinned");
        assert_eq!(kind.pinned, 4, "pinned at |extension|");
        assert_eq!(kind.live, 4, "live reads the sealed extension");
        assert!((kind.ratio - 1.0).abs() < f64::EPSILON, "{kind:?}");
        assert!(
            (staleness.max_ratio - 1.0).abs() < f64::EPSILON,
            "{staleness:?}"
        );
        Ok(())
    })
    .expect("fresh read");

    // Grow Reading 3 → 12: the ordinary occurrence drifts to 4; the
    // closed occurrence stays exactly 1 — sealed rows cannot move.
    db.write(|tx| {
        for i in 0..9i64 {
            #[expect(clippy::cast_sign_loss, reason = "0..9 is nonnegative")]
            let kind = (i as u64) % 4;
            tx.insert_dyn(READING, &[Value::U64(kind), Value::I64(1000 + i)])?;
        }
        Ok(())
    })
    .expect("grow readings");
    db.read(|snap| {
        let staleness = prepared.staleness(snap)?;
        let reading = staleness
            .per_occurrence
            .iter()
            .find(|d| d.relation == READING)
            .expect("the ordinary occurrence is pinned");
        assert!(
            (reading.ratio - 4.0).abs() < f64::EPSILON,
            "the ordinary occurrence drifts: {reading:?}"
        );
        let kind = staleness
            .per_occurrence
            .iter()
            .find(|d| d.relation == KIND)
            .expect("the kept closed occurrence is pinned");
        assert!(
            (kind.ratio - 1.0).abs() < f64::EPSILON,
            "a closed pin never phantom-drifts: {kind:?}"
        );
        Ok(())
    })
    .expect("drifted read");
}
