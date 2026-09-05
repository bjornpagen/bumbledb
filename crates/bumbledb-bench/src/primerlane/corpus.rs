use bumbledb::Value;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType, Weight,
};

use super::PrimerConfig;
use crate::corpus_gen::Rng;

pub const VOCABULARY: u64 = 1024;

const NOVEL_DEN: u64 = 8;

const WEIGHTS: &[u64] = &[13, 8, 5, 3, 2, 1, 1, 2];

const CAPACITY_CEILING: u64 = 1 << 40;

const PAYLOAD: &[ValueType] = &[
    ValueType::String,
    ValueType::U64,
    ValueType::String,
    ValueType::I64,
    ValueType::Bool,
    ValueType::String,
];

fn arity_of(rel: u32) -> usize {
    2 + (rel as usize % 7)
}

fn payload_type(rel: u32, slot: usize) -> ValueType {
    PAYLOAD[(rel as usize + slot) % PAYLOAD.len()]
}

fn side(relation: RelationId, projection: &[FieldId]) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: Vec::new().into_boxed_slice(),
    }
}

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
    // Declared id keys first (E-NO-RESERVE): the retired fresh auto-keys
    // are ordinary declared statements now; the generator supplies dense
    // index-aligned ids itself, so the keys hold by construction.
    for rel in 0..cfg.relations {
        statements.push(StatementDescriptor::Functionality {
            relation: RelationId(rel),
            projection: Box::new([FieldId(0)]),
        });
    }
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

fn fields_of(rel: u32) -> Vec<FieldDescriptor> {
    let mut fields = vec![FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
    }];
    if rel > 0 {
        fields.push(FieldDescriptor {
            name: "parent".into(),
            value_type: ValueType::U64,
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
        });
    }
    fields
}

/// # Panics
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

fn word(rng: &mut Rng, rel: RelationId, index: u64, slot: usize) -> Box<str> {
    if rng.chance(1, NOVEL_DEN) {
        format!("novel-{}-{index}-{slot}", rel.0).into()
    } else {
        let bucket = u64::from(rng.u64().trailing_zeros()).min(u64::from(VOCABULARY.ilog2()) - 1);
        let lo = (1u64 << bucket) - 1;
        let hi = ((1u64 << (bucket + 1)) - 1).min(VOCABULARY);
        format!("w{:04}", lo + rng.range(hi - lo)).into()
    }
}
