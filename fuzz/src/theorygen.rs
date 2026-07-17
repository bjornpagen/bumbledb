//! The theory-generation seam: the structurally-free arm re-exported
//! from the bench crate, plus the well-formed-but-adversarial tier
//! (TODO.md § PHASE A-FUZZ). The free arm reaches every rejection class
//! but bounces off the validator on most draws — acceptance is a lucky
//! alignment of ids, names, and keys. This tier aims INSIDE accepted
//! shapes: resolvable ids, matching containment keys, distinct names,
//! and hostile values at the legal extremes (width edges 1/64, the
//! interval-width ceiling, `MAX_EXTENSION_ROWS`, huge windows, raw-word
//! literals), so executions reach the sealing pass, the fingerprint,
//! reopen, and the genesis-debt sweep behind the gate. The tier owns no
//! validity logic: it aims at acceptance and the ENGINE still judges —
//! a missed aim is a legal rejection, never asserted otherwise.

pub use bumbledb_bench::corpus_gen::theorygen::*;

use bumbledb::Value;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet, MAX_EXTENSION_ROWS,
    RelationDescriptor, RelationId, Row, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb_bench::corpus_gen::Rng;

/// The tier's fixed cast: two ordinary relations over one shared key
/// grid (the containment-key recipe the arity arm proved out) and one
/// closed axis for the domain-quantification and closed-parent shapes.
const SRC: RelationId = RelationId(0);
const TGT: RelationId = RelationId(1);
const AXIS: RelationId = RelationId(2);

/// A well-formed-but-adversarial descriptor: every id resolvable, every
/// containment keyed, every extension row exactly typed — and every
/// value drawn from the legal extremes. Acceptance is the aim, not a
/// guarantee; the verdict stays the engine's.
pub fn adversarial_descriptor(rng: &mut Rng) -> SchemaDescriptor {
    let key_arity = 1 + draw(rng, 3);
    let key_types: Vec<ValueType> = (0..key_arity).map(|_| key_type(rng)).collect();
    let tag_type = key_type(rng);
    let interval = interval_type(rng);

    let src = grid_relation(rng, "Src", &key_types, &interval, &tag_type, true);
    let tgt = grid_relation(rng, "Tgt", &key_types, &interval, &tag_type, false);
    let (axis, axis_rows) = axis_relation(rng);

    // Sealed field offsets of the grid relations, by construction.
    let during = field_id(key_arity);
    let gate = field_id(key_arity + 1);
    let load = field_id(key_arity + 2);
    let tag = field_id(key_arity + 3);

    let key: Box<[FieldId]> = (0..key_arity).map(field_id).collect();
    // A reversed determinant at arity ≥ 2: key order is free, the
    // sealed plan must not care.
    let determinant: Box<[FieldId]> = if key_arity >= 2 && rng.chance(1, 2) {
        key.iter().rev().copied().collect()
    } else {
        key.clone()
    };

    let mut statements = vec![
        StatementDescriptor::Functionality {
            relation: SRC,
            projection: determinant.clone(),
        },
        StatementDescriptor::Functionality {
            relation: TGT,
            projection: determinant,
        },
    ];

    // The pointwise-key shape: the interval column trails a scalar
    // prefix — legal exactly there, and the extreme widths ride along.
    if rng.chance(1, 3) {
        statements.push(StatementDescriptor::Functionality {
            relation: SRC,
            projection: key.iter().copied().chain([during]).collect(),
        });
    }

    // The keyed forward containment, selections on unprojected fields
    // only — gates as booleans, the tag as a literal SET of extremes.
    let src_side = grid_side(rng, SRC, &key, gate, tag, &tag_type);
    let tgt_side = grid_side(rng, TGT, &key, gate, tag, &tag_type);
    statements.push(StatementDescriptor::Containment {
        source: src_side.clone(),
        target: tgt_side.clone(),
    });
    // The keyed equality: both directions, both keys already present.
    if rng.chance(1, 2) {
        statements.push(StatementDescriptor::Containment {
            source: tgt_side,
            target: src_side,
        });
    }

    // A window over the key head — bounds legal but extreme (the empty
    // allowance, keyed ==, raw-word ceilings and floors). The head
    // needs its own target key at arity ≥ 2 (the determinant covers
    // arity 1, and re-declaring it would be the duplicate-key
    // rejection).
    if rng.chance(1, 2) {
        if key_arity >= 2 {
            statements.push(StatementDescriptor::Functionality {
                relation: TGT,
                projection: Box::new([field_id(0)]),
            });
        }
        let (lo, hi) = window(rng);
        statements.push(StatementDescriptor::Cardinality {
            source: bare_side(SRC, &[field_id(0)]),
            lo,
            hi,
            target: bare_side(TGT, &[field_id(0)]),
        });
    }

    // Domain quantification: the closed axis's φ-selected level column
    // must appear among Tgt loads — genesis debt by the recorded
    // division of authority, exercised behind acceptance.
    if rng.chance(1, 2) {
        statements.push(StatementDescriptor::Functionality {
            relation: TGT,
            projection: Box::new([load]),
        });
        statements.push(StatementDescriptor::Containment {
            source: Side {
                relation: AXIS,
                projection: Box::new([FieldId(1)]),
                selection: Box::new([(FieldId(2), LiteralSet::One(Value::Bool(true)))]),
            },
            target: bare_side(TGT, &[load]),
        });
    }

    // The closed-parent floor window: a ψ-selected axiom (a REAL row's
    // level, so the selection resolves) demanding children it cannot
    // have at genesis when `lo ≥ 1`.
    if rng.chance(1, 2) {
        let row = draw(rng, axis_rows);
        // A floor of 1 keeps a bounded ceiling (`1..*` is the banned
        // containment spelling); a floor of 2 may run to `*`.
        let (lo, hi) = if rng.chance(1, 2) {
            (1, Some(1 + rng.range(7)))
        } else {
            (
                2,
                if rng.chance(1, 2) {
                    None
                } else {
                    Some(rng.u64() | 3)
                },
            )
        };
        statements.push(StatementDescriptor::Cardinality {
            source: bare_side(SRC, &[load]),
            lo,
            hi,
            target: Side {
                relation: AXIS,
                projection: Box::new([FieldId(0)]),
                selection: Box::new([(FieldId(1), LiteralSet::One(axis_level(row)))]),
            },
        });
    }

    SchemaDescriptor {
        relations: vec![src, tgt, axis],
        statements,
    }
}

/// One grid relation: the shared key columns, a trailing interval, two
/// selection gates, a u64 load, and one free-typed tag — names distinct
/// by construction, `Fresh` only where legal (a u64 head on the
/// ordinary source).
fn grid_relation(
    rng: &mut Rng,
    name: &str,
    key_types: &[ValueType],
    interval: &ValueType,
    tag_type: &ValueType,
    fresh_head: bool,
) -> RelationDescriptor {
    let mut fields: Vec<FieldDescriptor> = key_types
        .iter()
        .enumerate()
        .map(|(index, value_type)| FieldDescriptor {
            name: format!("k{index}").into(),
            value_type: value_type.clone(),
            // Fresh only where its auto-key cannot duplicate the
            // declared determinant (arity ≥ 2 keeps them distinct).
            generation: if index == 0
                && fresh_head
                && key_types.len() >= 2
                && *value_type == ValueType::U64
                && rng.chance(1, 4)
            {
                Generation::Fresh
            } else {
                Generation::None
            },
        })
        .collect();
    fields.push(FieldDescriptor {
        name: "during".into(),
        value_type: interval.clone(),
        generation: Generation::None,
    });
    fields.push(FieldDescriptor {
        name: "gate".into(),
        value_type: ValueType::Bool,
        generation: Generation::None,
    });
    fields.push(FieldDescriptor {
        name: "load".into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    });
    fields.push(FieldDescriptor {
        name: "tag".into(),
        value_type: tag_type.clone(),
        generation: Generation::None,
    });
    RelationDescriptor {
        name: name.into(),
        fields,
        extension: None,
    }
}

/// The closed axis: `(level u64, live bool)` axioms — the row count
/// rides the legal boundary (`MAX_EXTENSION_ROWS` and its shoulder),
/// handles distinct by index, levels the deterministic extreme ladder.
fn axis_relation(rng: &mut Rng) -> (RelationDescriptor, usize) {
    let rows = match rng.range(8) {
        0 => MAX_EXTENSION_ROWS,
        1 => MAX_EXTENSION_ROWS - 1,
        n => 1 + usize::try_from(n).expect("small draw fits usize"),
    };
    let extension: Box<[Row]> = (0..rows)
        .map(|row| Row {
            handle: format!("a{row}").into(),
            values: Box::new([axis_level(row), Value::Bool(row % 2 == 0)]),
        })
        .collect();
    (
        RelationDescriptor {
            name: "Axis".into(),
            fields: vec![
                FieldDescriptor {
                    name: "level".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "live".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
            ],
            extension: Some(extension),
        },
        rows,
    )
}

/// The axis's level for one row — a deterministic function, so a
/// ψ-selection over a drawn row index cites a value that exists.
fn axis_level(row: usize) -> Value {
    let row = u64::try_from(row).expect("row index fits u64");
    Value::U64(row.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
}

/// One containment side over the key projection, selections drawn from
/// the unprojected columns: the boolean gate, a literal SET of extremes
/// on the tag, both, or none.
fn grid_side(
    rng: &mut Rng,
    relation: RelationId,
    key: &[FieldId],
    gate: FieldId,
    tag: FieldId,
    tag_type: &ValueType,
) -> Side {
    let mut selection: Vec<(FieldId, LiteralSet)> = Vec::new();
    if rng.chance(1, 2) {
        selection.push((gate, LiteralSet::One(Value::Bool(true))));
    }
    if rng.chance(1, 3) {
        selection.push((tag, LiteralSet::Many(extreme_pair(rng, tag_type))));
    }
    Side {
        relation,
        projection: key.into(),
        selection: selection.into(),
    }
}

fn bare_side(relation: RelationId, projection: &[FieldId]) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: Box::new([]),
    }
}

/// A legal-but-extreme window: the empty allowance, the keyed `==`, a
/// raw-word ceiling, a raw-word floor with the `*` end — never
/// inverted, never the vacuous `0..*`, never the `1..*` containment
/// spelling (both banned utterances, the free arm's to reach).
fn window(rng: &mut Rng) -> (u64, Option<u64>) {
    match rng.range(6) {
        0 => (0, Some(0)),
        1 => (1, Some(1)),
        2 => (0, Some(rng.u64() | 1)),
        3 => (1, Some(2 + rng.range(7))),
        4 => {
            let lo = rng.u64() | 1;
            (lo, Some(lo))
        }
        _ => (2 + rng.range(7), None),
    }
}

/// The key/tag column pool: every orderable and key-legal scalar with
/// its width extremes — `bytes<1>` and `bytes<64>` are the legal edges
/// of the fixed-width gate.
fn key_type(rng: &mut Rng) -> ValueType {
    match rng.range(6) {
        0 | 1 => ValueType::U64,
        2 => ValueType::I64,
        3 => ValueType::String,
        4 => ValueType::FixedBytes { len: 1 },
        _ => ValueType::FixedBytes { len: 64 },
    }
}

/// The trailing interval column: both element domains, the width knob
/// at its legal extremes (1 and `u64::MAX − 1`) beside the general
/// family.
fn interval_type(rng: &mut Rng) -> ValueType {
    let element = if rng.chance(1, 2) {
        IntervalElement::U64
    } else {
        IntervalElement::I64
    };
    let width = match rng.range(4) {
        0 => Some(1),
        1 => Some(u64::MAX - 1),
        _ => None,
    };
    ValueType::Interval { element, width }
}

/// Two DISTINCT extremes of one type — the honest literal set (the
/// degenerate and duplicate shapes are the free arm's to reach).
fn extreme_pair(rng: &mut Rng, value_type: &ValueType) -> Box<[Value]> {
    match value_type {
        ValueType::Bool => Box::new([Value::Bool(false), Value::Bool(true)]),
        ValueType::U64 => Box::new([Value::U64(rng.u64() | 1), Value::U64(0)]),
        ValueType::I64 => Box::new([Value::I64(i64::MIN), Value::I64(i64::MAX)]),
        ValueType::String => Box::new([
            Value::String(Box::from(&b""[..])),
            Value::String(Box::from(&b"\xE2\x88\x9E"[..])),
        ]),
        ValueType::FixedBytes { len } => {
            let len = usize::from(*len);
            Box::new([
                Value::FixedBytes(vec![0x00; len].into()),
                Value::FixedBytes(vec![0xFF; len].into()),
            ])
        }
        ValueType::Interval { .. } => {
            unreachable!("the tag pool is scalar (key_type)")
        }
    }
}

fn field_id(index: usize) -> FieldId {
    FieldId(u16::try_from(index).expect("grid field index fits u16"))
}

fn draw(rng: &mut Rng, n: usize) -> usize {
    let n = u64::try_from(n).expect("count fits u64");
    usize::try_from(rng.range(n)).expect("draw fits usize")
}

#[cfg(test)]
mod tests {
    use super::adversarial_descriptor;
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb_bench::corpus_gen::Rng;

    /// The tier is deterministic in its entropy, as every arm must be.
    #[test]
    fn the_same_bytes_yield_the_same_descriptor() {
        let bytes: Vec<u8> = (1..=64u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        assert_eq!(
            adversarial_descriptor(&mut Rng::from_bytes(&bytes)),
            adversarial_descriptor(&mut Rng::from_bytes(&bytes)),
            "same bytes, same descriptor"
        );
    }

    /// The tier's whole point: a strong majority of draws pass the
    /// acceptance gate (the free arm's accept rate is luck-bound), and
    /// the extension boundary is reached. A missed aim stays a legal
    /// rejection — the assertion is on the RATE, not on every draw.
    #[test]
    fn the_tier_biases_hard_toward_acceptance() {
        let mut accepted = 0u32;
        let mut boundary_rows = false;
        for seed in 0..512 {
            let descriptor = adversarial_descriptor(&mut Rng::new(seed));
            if descriptor.relations[2]
                .extension
                .as_deref()
                .is_some_and(|rows| rows.len() == bumbledb::schema::MAX_EXTENSION_ROWS)
            {
                boundary_rows = true;
            }
            if descriptor.validate().is_ok() {
                accepted += 1;
            }
        }
        assert!(
            accepted >= 384,
            "the adversarial tier must accept on at least 3/4 of draws, got {accepted}/512"
        );
        assert!(boundary_rows, "the extension ceiling never arose");
    }
}
