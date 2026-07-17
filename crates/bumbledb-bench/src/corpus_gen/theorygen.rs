//! The random-descriptor arm (docs/architecture/60-validation.md § the
//! fuzzing charter): structurally-free [`SchemaDescriptor`]s for the fuzz
//! lanes. Unlike the fixed ledger theory (`crate::schema`), this arm
//! deliberately reaches invalid shapes — dangling relation/field ids,
//! arity mismatches, duplicate names, closed-relation member abuse,
//! interval misuse (empty bounds, the ray end) — alongside valid ones,
//! and the ENGINE judges. The generator shares the ledger's vocabulary
//! and owns no validity logic: index-anchored name draws and typed-value
//! hints bias toward acceptance, but nothing here re-implements the
//! acceptance gate (refusal: a generator that knows the rules can only
//! confirm them).

use bumbledb::Value;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
};

use super::Rng;

mod arity;

pub use arity::{
    ARITY_WIDTH_BOUND, ArityCoverage, ArityDescriptorCase, ArityExpectation, ArityOpsCase,
    MAX_MIXED_ARITY, SelectionPlacement, arity_descriptor, random_arity_descriptor,
    random_valid_arity_descriptor, random_valid_arity_ops,
};

/// The ledger's relation vocabulary — shared names, so mutated inputs
/// collide meaningfully instead of drifting into gibberish.
const RELATION_NAMES: &[&str] = &[
    "Holder",
    "Account",
    "Instrument",
    "Posting",
    "Org",
    "Mandate",
    "Currency",
    "Tag",
];

const FIELD_NAMES: &[&str] = &[
    "id", "holder", "name", "amount", "at", "account", "org", "active", "tag", "symbol",
];

const HANDLES: &[&str] = &["Usd", "Eur", "Gbp", "Manual", "Import", "Fee"];

/// `bytes<N>` widths: legal points, both illegal edges (0 and 65), and a
/// far-out width — the roster's `FixedBytesWidthOutOfRange` is a draw
/// away, never a special case.
const FIXED_LENS: &[u16] = &[0, 1, 16, 32, 64, 65, 300];

/// A structurally-free schema descriptor: 0–4 relations, 0–5 statements,
/// every shape the descriptor type can spell reachable by some byte
/// string. Valid and invalid descriptors both arise; the verdict is the
/// engine's.
pub fn random_descriptor(rng: &mut Rng) -> SchemaDescriptor {
    let relation_count = draw(rng, 5);
    let relations: Vec<_> = (0..relation_count)
        .map(|idx| random_relation(rng, idx))
        .collect();
    let statement_count = draw(rng, 6);
    let statements = (0..statement_count)
        .map(|_| random_statement(rng, &relations))
        .collect();
    SchemaDescriptor {
        relations,
        statements,
    }
}

fn random_relation(rng: &mut Rng, idx: usize) -> RelationDescriptor {
    // Index-anchored names are mostly distinct; the free draw forces the
    // occasional `DuplicateRelationName` collision.
    let name = if rng.chance(1, 8) {
        pick(rng, RELATION_NAMES)
    } else {
        RELATION_NAMES[idx % RELATION_NAMES.len()]
    };
    let field_count = draw(rng, 5);
    let fields: Vec<_> = (0..field_count)
        .map(|field_idx| random_field(rng, field_idx))
        .collect();
    // A quarter of relations declare an extension (closed): rows over the
    // handle vocabulary, values hinted by the columns but free to abuse
    // arity, type, interval bounds, and the ray end.
    let extension = if rng.chance(1, 4) {
        Some(random_extension(rng, &fields))
    } else {
        None
    };
    RelationDescriptor {
        name: name.into(),
        fields,
        extension,
    }
}

fn random_field(rng: &mut Rng, idx: usize) -> FieldDescriptor {
    let name = if rng.chance(1, 8) {
        pick(rng, FIELD_NAMES)
    } else {
        FIELD_NAMES[idx % FIELD_NAMES.len()]
    };
    // `Fresh` lands on any type and any relation kind — `FreshOnNonU64`
    // and `FreshOnClosedRelation` are the engine's to refuse.
    let generation = if rng.chance(1, 5) {
        Generation::Fresh
    } else {
        Generation::None
    };
    FieldDescriptor {
        name: name.into(),
        value_type: random_type(rng),
        generation,
    }
}

fn random_type(rng: &mut Rng) -> ValueType {
    match rng.range(7) {
        0 => ValueType::Bool,
        1 | 2 => ValueType::U64,
        3 => ValueType::I64,
        4 => ValueType::String,
        5 => ValueType::FixedBytes {
            len: FIXED_LENS[draw(rng, FIXED_LENS.len())],
        },
        // The corpus generator deliberately draws only the GENERAL
        // interval type: fixture provenance is byte-pinned, and the
        // fixed-width family's storage seam is covered by the engine's
        // own suites.
        6 => ValueType::Interval {
            element: IntervalElement::U64,
            width: None,
        },
        _ => ValueType::Interval {
            element: IntervalElement::I64,
            width: None,
        },
    }
}

fn random_extension(rng: &mut Rng, fields: &[FieldDescriptor]) -> Box<[Row]> {
    let rows = draw(rng, 4); // zero rows: the vocabulary of nothing
    (0..rows)
        .map(|row| {
            let handle = if rng.chance(1, 8) {
                pick(rng, HANDLES)
            } else {
                HANDLES[row % HANDLES.len()]
            };
            // Mostly the declared arity with column-typed values; the
            // free draw reaches `ExtensionArityMismatch`.
            let arity = if rng.chance(7, 8) {
                fields.len()
            } else {
                draw(rng, 4)
            };
            let values = (0..arity)
                .map(|col| random_value(rng, fields.get(col).map(|f| &f.value_type)))
                .collect();
            Row {
                handle: handle.into(),
                values,
            }
        })
        .collect()
}

/// A literal, usually inhabiting the hinted column type and sometimes any
/// shape at all — the type-mismatch rosters stay reachable.
fn random_value(rng: &mut Rng, hint: Option<&ValueType>) -> Value {
    match hint {
        Some(value_type) if rng.chance(7, 8) => typed_value(rng, value_type),
        _ => {
            let value_type = random_type(rng);
            typed_value(rng, &value_type)
        }
    }
}

fn typed_value(rng: &mut Rng, value_type: &ValueType) -> Value {
    match value_type {
        ValueType::Bool => Value::Bool(rng.chance(1, 2)),
        ValueType::U64 => {
            // Mostly the small handle vocabulary; occasionally a word
            // beyond `u16::MAX`, so a closed-reference draw reaches the
            // out-of-range → non-membership narrowing at validate (a
            // corpus whose draws stay small never exercises that arm).
            if rng.chance(1, 8) {
                Value::U64(u64::from(u16::MAX) + 1 + rng.range(16))
            } else {
                Value::U64(rng.range(16))
            }
        }
        ValueType::I64 => Value::I64(signed(rng)),
        ValueType::String => {
            if rng.chance(1, 8) {
                // Non-UTF-8 bytes: `SelectionLiteralNotUtf8` and the
                // extension's value check both see hostile strings.
                Value::String(Box::from(&[0xFF, 0xFE, 0x00][..]))
            } else {
                Value::String(pick(rng, HANDLES).as_bytes().into())
            }
        }
        ValueType::FixedBytes { len } => {
            let declared = usize::from(*len);
            // The width is the type: an off-by-one draw is a mismatch.
            let width = if rng.chance(7, 8) {
                declared
            } else {
                declared + 1
            };
            Value::FixedBytes(vec![0xA5; width].into())
        }
        ValueType::Interval { element, .. } => interval_value(rng, *element),
    }
}

/// Interval bounds over the representable shape ladder: unit, wide, and
/// the ray end (`MAX` = ∞). The former empty rung maps to unit without an
/// extra entropy draw; malformed `Value` payloads are no longer a schema
/// descriptor state.
fn interval_value(rng: &mut Rng, element: IntervalElement) -> Value {
    match element {
        IntervalElement::U64 => {
            let start = rng.range(8);
            let end = match rng.range(4) {
                0 | 1 => start + 1,
                2 => start + 2 + rng.range(5),
                _ => u64::MAX,
            };
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
            )
        }
        IntervalElement::I64 => {
            let start = signed(rng);
            let end = match rng.range(4) {
                0 => start.saturating_add(1),
                1 => start + 1,
                2 => start + 2 + signed(rng).abs(),
                _ => i64::MAX,
            };
            Value::IntervalI64(
                bumbledb::Interval::<i64>::new(start, end).expect("nonempty interval"),
            )
        }
    }
}

fn random_statement(rng: &mut Rng, relations: &[RelationDescriptor]) -> StatementDescriptor {
    match rng.range(7) {
        0 | 1 => {
            let relation = random_relation_id(rng, relations.len());
            StatementDescriptor::Functionality {
                relation,
                projection: random_projection(rng, relations, relation),
            }
        }
        2..=4 => StatementDescriptor::Containment {
            source: random_side(rng, relations),
            target: random_side(rng, relations),
        },
        // The cardinality window, bounds structurally free: inverted
        // windows (`lo > hi`), the `0..*` vacuity, the `1..1` keyed-`==`
        // shape, and interval-projected sides
        // (`CardinalityIntervalPosition`) are all a draw away.
        _ => StatementDescriptor::Cardinality {
            source: random_side(rng, relations),
            lo: rng.range(4),
            hi: if rng.chance(1, 3) {
                None // the `*` spelling
            } else {
                Some(rng.range(5))
            },
            target: random_side(rng, relations),
        },
    }
}

fn random_side(rng: &mut Rng, relations: &[RelationDescriptor]) -> Side {
    let relation = random_relation_id(rng, relations.len());
    let projection = random_projection(rng, relations, relation);
    let bindings = draw(rng, 3);
    let selection = (0..bindings)
        .map(|_| {
            let field = random_field_id(rng, field_span(relations, relation));
            let hint = relations
                .get(usize::try_from(relation.0).expect("relation id fits usize"))
                .and_then(|rel| rel.fields.get(usize::from(field.0)))
                .map(|f| &f.value_type);
            // A quarter of bindings are literal SETS — sized 0–3, so the
            // degenerate shapes (`DegenerateSelectionSet`) and free
            // duplicates (`DuplicateSelectionLiteral`) arise beside the
            // honest disjunctions; the engine judges.
            let literals = if rng.chance(1, 4) {
                let len = draw(rng, 4);
                bumbledb::schema::LiteralSet::Many(
                    (0..len).map(|_| random_value(rng, hint)).collect(),
                )
            } else {
                bumbledb::schema::LiteralSet::One(random_value(rng, hint))
            };
            (field, literals)
        })
        .collect();
    Side {
        relation,
        projection,
        selection,
    }
}

/// Mostly a declared relation, sometimes a dangling id — the
/// `StatementUnknownRelation` roster line stays a draw away.
fn random_relation_id(rng: &mut Rng, count: usize) -> RelationId {
    let count = u64::try_from(count).expect("relation count fits u64");
    let id = if count > 0 && rng.chance(7, 8) {
        rng.range(count)
    } else {
        rng.range(count + 3)
    };
    RelationId(u32::try_from(id).expect("relation id fits u32"))
}

fn random_projection(
    rng: &mut Rng,
    relations: &[RelationDescriptor],
    relation: RelationId,
) -> Box<[FieldId]> {
    let span = field_span(relations, relation);
    let len = draw(rng, 4); // zero fields: the empty projection
    (0..len).map(|_| random_field_id(rng, span)).collect()
}

/// Mostly within the relation's declared fields (duplicates arise freely),
/// sometimes dangling.
fn random_field_id(rng: &mut Rng, span: u64) -> FieldId {
    let id = if span > 0 && rng.chance(7, 8) {
        rng.range(span)
    } else {
        rng.range(span + 3)
    };
    FieldId(u16::try_from(id).expect("field id fits u16"))
}

/// The relation's declared field count, zero when the id dangles.
fn field_span(relations: &[RelationDescriptor], relation: RelationId) -> u64 {
    relations
        .get(usize::try_from(relation.0).expect("relation id fits usize"))
        .map_or(0, |rel| {
            u64::try_from(rel.fields.len()).expect("field count fits u64")
        })
}

/// A small signed draw centered on zero.
fn signed(rng: &mut Rng) -> i64 {
    i64::try_from(rng.range(16)).expect("small draw fits i64") - 8
}

fn draw(rng: &mut Rng, n: usize) -> usize {
    let n = u64::try_from(n).expect("count fits u64");
    usize::try_from(rng.range(n)).expect("draw fits usize")
}

fn pick<'pool>(rng: &mut Rng, pool: &'pool [&'pool str]) -> &'pool str {
    pool[draw(rng, pool.len())]
}

#[cfg(test)]
mod tests {
    use super::random_descriptor;
    use crate::corpus_gen::Rng;
    use bumbledb::schema::ValidateDescriptor as _;

    /// The arm is deterministic in its entropy: the same byte string
    /// yields the identical descriptor, and a different one steers away.
    #[test]
    fn the_same_bytes_yield_the_same_descriptor() {
        let bytes: Vec<u8> = (1..=64u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        let first = random_descriptor(&mut Rng::from_bytes(&bytes));
        assert_eq!(
            first,
            random_descriptor(&mut Rng::from_bytes(&bytes)),
            "same bytes, same descriptor"
        );
        let other: Vec<u8> = (1..=64u64)
            .flat_map(|i| i.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).to_le_bytes())
            .collect();
        assert_ne!(
            first,
            random_descriptor(&mut Rng::from_bytes(&other)),
            "bytes steer the descriptor"
        );
    }

    /// The arm's whole point: across a modest seed sweep the engine both
    /// accepts and rejects — a generator that only produces one verdict
    /// class fuzzes nothing.
    #[test]
    fn the_arm_reaches_both_verdict_classes() {
        let mut accepted = 0u32;
        let mut rejected = 0u32;
        for seed in 0..256 {
            let descriptor = random_descriptor(&mut Rng::new(seed));
            match descriptor.validate() {
                Ok(_) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
        assert!(accepted > 0, "no accepted schema in 256 seeds");
        assert!(rejected > 0, "no rejected schema in 256 seeds");
        eprintln!("mix: {accepted} accepted / {rejected} rejected");
    }

    /// The statement-form sweep (docs/architecture/60-validation.md §
    /// negative validation, the adversarial estate's schema half): 10⁴+
    /// structurally-free descriptors through the acceptance gate —
    /// every outcome `Ok` or a typed error, any panic a red run with
    /// its seed named — with the generated surface itself asserted:
    /// both verdict classes, both classical forms, the cardinality
    /// window at both spellings (a ceiling and the `*`), and the
    /// literal-set selections, each reached.
    #[test]
    fn the_descriptor_sweep_reaches_every_statement_form_without_panicking() {
        use bumbledb::schema::{LiteralSet, StatementDescriptor};
        use std::panic::{AssertUnwindSafe, catch_unwind};

        const SWEEP: u64 = 12_000;
        let mut accepted = 0u64;
        let mut rejected = 0u64;
        let mut functionality = 0u64;
        let mut containment = 0u64;
        let mut window_bounded = 0u64;
        let mut window_star = 0u64;
        let mut set_selection = 0u64;
        for seed in 0..SWEEP {
            let descriptor = random_descriptor(&mut Rng::new(seed));
            for statement in &descriptor.statements {
                match statement {
                    StatementDescriptor::Functionality { .. } => functionality += 1,
                    StatementDescriptor::Containment { source, target } => {
                        containment += 1;
                        for side in [source, target] {
                            set_selection += side
                                .selection
                                .iter()
                                .filter(|(_, set)| matches!(set, LiteralSet::Many(_)))
                                .count() as u64;
                        }
                    }
                    StatementDescriptor::Cardinality { hi, .. } => {
                        if hi.is_some() {
                            window_bounded += 1;
                        } else {
                            window_star += 1;
                        }
                    }
                }
            }
            let verdict = catch_unwind(AssertUnwindSafe(|| {
                descriptor.clone().validate().map(|_| ())
            }))
            .unwrap_or_else(|_| {
                panic!("descriptor validation panicked (seed {seed}): {descriptor:#?}")
            });
            match verdict {
                Ok(()) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
        assert_eq!(accepted + rejected, SWEEP);
        for (label, count) in [
            ("accepted", accepted),
            ("rejected", rejected),
            ("functionality", functionality),
            ("containment", containment),
            ("bounded window", window_bounded),
            ("star window", window_star),
            ("set-selection", set_selection),
        ] {
            assert!(count > 0, "the sweep never reached: {label}");
        }
        eprintln!(
            "sweep: {accepted} accepted / {rejected} rejected; forms: fd {functionality}, \
             ind {containment}, window {window_bounded}+{window_star}*, \
             sets {set_selection}"
        );
    }
}
