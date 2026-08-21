//! The Primer-shaped corpus generator: schema and rows derive from
//! [`PrimerConfig`] alone — a generator config checked in, never a data
//! file (10-measurement.md). Every row is a pure function of
//! `(seed, relation, index)` through the shared per-row seed mix
//! ([`crate::corpus_gen::mix`]), so streams are restartable and the two
//! write lanes generate identical facts without a materialized corpus.

use bumbledb::Value;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValueType, Weight,
};

use super::PrimerConfig;
use crate::corpus_gen::Rng;

/// Zipf vocabulary size for the str columns — the committed-repeat case
/// 30-string-ownership.md targets.
pub const VOCABULARY: u64 = 1024;

/// One novel long-tail string per this many draws (the pending-mint
/// case): `1/NOVEL_DEN` of str cells are unique to their row.
const NOVEL_DEN: u64 = 8;

/// Per-relation row-count skew, cycled over the roster — a few big
/// relations dominate, the Primer shape.
const WEIGHTS: &[u64] = &[13, 8, 5, 3, 2, 1, 1, 2];

/// The capacity ceiling. Deliberately never binding: the lane prices
/// the judgment sweep on the accept path, not violation handling.
const CAPACITY_CEILING: u64 = 1 << 40;

/// Payload column types, cycled with a per-relation offset — several
/// str columns per relation across the roster, mixed with the scalar
/// kinds. [`descriptor`] and [`row`] both read this table through
/// [`payload_type`]; the schema and the generator cannot drift.
const PAYLOAD: &[ValueType] = &[
    ValueType::String,
    ValueType::U64,
    ValueType::String,
    ValueType::I64,
    ValueType::Bool,
    ValueType::String,
];

/// Arity of relation `rel`: 2–8, cycling — the mixed-arity mandate.
fn arity_of(rel: u32) -> usize {
    2 + (rel as usize % 7)
}

/// The payload type of sealed slot `slot` in relation `rel`.
fn payload_type(rel: u32, slot: usize) -> ValueType {
    PAYLOAD[(rel as usize + slot) % PAYLOAD.len()]
}

/// An unselected side — projection only.
fn side(relation: RelationId, projection: &[FieldId]) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: Vec::new().into_boxed_slice(),
    }
}

/// Derived per-relation row counts: `facts` split by the skew weights,
/// floored at 2 so every relation has a nonempty half in the delta
/// lane's two commits.
#[must_use]
pub fn relation_rows(cfg: &PrimerConfig) -> Vec<u64> {
    let weights: Vec<u64> = (0..cfg.relations)
        .map(|i| WEIGHTS[i as usize % WEIGHTS.len()])
        .collect();
    let total: u64 = weights.iter().sum();
    weights
        .iter()
        .map(|w| (cfg.facts * w / total).max(2))
        .collect()
}

/// The generated schema: `relations` ordinary relations, every one
/// fresh-keyed at field 0 (`id: u64, fresh` — the auto-functionality IS
/// the key the containment targets resolve); a containment chain
/// `Rel{i}(parent) <= Rel{i-1}(id)`; one capacity statement over the
/// chain's first edge.
#[must_use]
pub fn descriptor(cfg: &PrimerConfig) -> SchemaDescriptor {
    let relations = (0..cfg.relations)
        .map(|rel| RelationDescriptor {
            name: format!("Rel{rel}").into(),
            fields: fields_of(rel),
            extension: None,
        })
        .collect();
    let mut statements = Vec::new();
    for rel in 1..cfg.relations {
        statements.push(StatementDescriptor::Containment {
            source: side(RelationId(rel), &[FieldId(1)]),
            target: side(RelationId(rel - 1), &[FieldId(0)]),
        });
    }
    statements.push(StatementDescriptor::Capacity {
        target: side(RelationId(0), &[FieldId(0)]),
        weight: Weight::Unit,
        lo: 0,
        hi: Some(Bound::Lit(CAPACITY_CEILING)),
        source: side(RelationId(1), &[FieldId(1)]),
    });
    SchemaDescriptor {
        relations,
        statements,
    }
}

/// Relation `rel`'s field roster: `id` (fresh), `parent` on every
/// non-root relation, then payload columns from the cycle.
fn fields_of(rel: u32) -> Vec<FieldDescriptor> {
    let mut fields = vec![FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }];
    if rel > 0 {
        fields.push(FieldDescriptor {
            name: "parent".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        });
    }
    for slot in fields.len()..arity_of(rel) {
        let value_type = payload_type(rel, slot);
        let tag = match value_type {
            ValueType::String => 's',
            ValueType::U64 => 'n',
            ValueType::I64 => 'z',
            _ => 'b',
        };
        fields.push(FieldDescriptor {
            name: format!("{tag}{slot}").into(),
            value_type,
            generation: Generation::None,
        });
    }
    fields
}

/// One row of relation `rel`, in sealed field order. The fresh id IS
/// the row index — both write lanes reserve each relation's ids from 0
/// in index order, so the generator never needs a minted range handed
/// back. Parent references stay inside the parent's first half whenever
/// the child row is in its own first half, so the delta lane's seed
/// commit is containment-closed on its own.
///
/// # Panics
///
/// Never in practice: the I64 payload draw is bounded well inside `i64`.
#[must_use]
pub fn row(cfg: &PrimerConfig, counts: &[u64], rel: RelationId, index: u64) -> Vec<Value> {
    let mut rng = Rng::new(crate::corpus_gen::mix(cfg.seed, rel, index));
    let arity = arity_of(rel.0);
    let mut out = Vec::with_capacity(arity);
    out.push(Value::U64(index));
    if rel.0 > 0 {
        let parents = counts[rel.0 as usize - 1];
        let bound = if index < counts[rel.0 as usize] / 2 {
            (parents / 2).max(1)
        } else {
            parents
        };
        out.push(Value::U64(rng.range(bound)));
    }
    for slot in out.len()..arity {
        out.push(match payload_type(rel.0, slot) {
            ValueType::String => Value::String(word(&mut rng, rel, index, slot)),
            ValueType::U64 => Value::U64(rng.range(1_000_000)),
            ValueType::I64 => {
                let raw = i64::try_from(rng.range(2_000_000)).expect("bounded draw fits i64");
                Value::I64(raw - 1_000_000)
            }
            _ => Value::Bool(rng.chance(1, 2)),
        });
    }
    out
}

/// One str cell: mostly a Zipf-vocabulary word (the committed-repeat
/// skew), a long-tail row-unique novel string one draw in
/// [`NOVEL_DEN`] (the pending-mint population).
fn word(rng: &mut Rng, rel: RelationId, index: u64, slot: usize) -> Box<str> {
    if rng.chance(1, NOVEL_DEN) {
        format!("novel-{}-{index}-{slot}", rel.0).into()
    } else {
        // Integer Zipf-ish draw: a geometric bucket (P = 2^-(b+1)) then
        // uniform within — density falls ~1/rank, no floats, so the
        // stream is bit-identical on every platform.
        let bucket = u64::from(rng.u64().trailing_zeros()).min(u64::from(VOCABULARY.ilog2()) - 1);
        let lo = (1u64 << bucket) - 1;
        let hi = ((1u64 << (bucket + 1)) - 1).min(VOCABULARY);
        format!("w{:04}", lo + rng.range(hi - lo)).into()
    }
}
