use bumbledb::Value;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType, Weight,
};

use super::Rng;

mod arity;

pub use arity::{
    ARITY_WIDTH_BOUND, ArityCoverage, ArityDescriptorCase, ArityExpectation, ArityOpsCase,
    MAX_MIXED_ARITY, SelectionPlacement, arity_descriptor, random_arity_descriptor,
    random_valid_arity_descriptor, random_valid_arity_ops,
};

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

const FIXED_LENS: &[u16] = &[0, 1, 16, 32, 64, 65, 300];

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
    let name = if rng.chance(1, 8) {
        pick(rng, RELATION_NAMES)
    } else {
        RELATION_NAMES[idx % RELATION_NAMES.len()]
    };
    let field_count = draw(rng, 5);
    let fields: Vec<_> = (0..field_count)
        .map(|field_idx| random_field(rng, field_idx))
        .collect();

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

    // The successor has no generated-field attribute: the retired fresh
    // draw is gone WITH its mechanism (E-NO-RESERVE), so the descriptor
    // grammar this generator samples is exactly the declared one.
    // Checked-in corpora regenerate in F3 (deferred command recorded in
    // implementation/packets/P11.md).
    FieldDescriptor {
        name: name.into(),
        value_type: random_type(rng),
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

        6 => ValueType::Interval {
            element: IntervalElement::U64,
        },
        _ => ValueType::Interval {
            element: IntervalElement::I64,
        },
    }
}

fn random_extension(rng: &mut Rng, fields: &[FieldDescriptor]) -> Box<[Row]> {
    let rows = draw(rng, 4);
    (0..rows)
        .map(|row| {
            let handle = if rng.chance(1, 8) {
                pick(rng, HANDLES)
            } else {
                HANDLES[row % HANDLES.len()]
            };

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
            if rng.chance(1, 8) {
                Value::U64(u64::from(u16::MAX) + 1 + rng.range(16))
            } else {
                Value::U64(rng.range(16))
            }
        }
        ValueType::I64 => Value::I64(signed(rng)),
        ValueType::F64 => Value::F64(bumbledb::F64::from_bits(rng.u64())),
        ValueType::String => Value::String(pick(rng, HANDLES).into()),
        ValueType::FixedBytes { len } => {
            let declared = usize::from(*len);

            let width = if rng.chance(7, 8) {
                declared
            } else {
                declared + 1
            };
            Value::FixedBytes(vec![0xA5; width].into())
        }
        ValueType::Id128 => {
            let mut bytes = [0u8; 16];
            bytes[..8].copy_from_slice(&rng.u64().to_be_bytes());
            bytes[8..].copy_from_slice(&rng.u64().to_be_bytes());
            Value::Id128(bumbledb::Id128::from_bytes(bytes))
        }
        ValueType::Interval { element } => interval_value(rng, *element),
        ValueType::FixedInterval { element, .. } => interval_value(rng, element.element()),
    }
}

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
        IntervalElement::F64 => {
            // `signed` draws stay in [-8, 8): every endpoint is exact.
            let start = i32::try_from(signed(rng)).expect("small draw fits i32");
            let end = if rng.chance(1, 8) {
                bumbledb::F64::INFINITY
            } else {
                let width = i32::try_from(signed(rng).abs()).expect("small draw fits i32");
                bumbledb::F64::from(f64::from(start + 1 + width))
            };
            Value::IntervalF64(
                bumbledb::Interval::new(bumbledb::F64::from(f64::from(start)), end)
                    .expect("start < end by construction"),
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

        _ => {
            let target = random_side(rng, relations);
            let source = random_side(rng, relations);
            let weight = random_weight(rng, relations, source.relation);
            let hi = if rng.chance(1, 3) {
                None
            } else {
                Some(random_bound(rng, relations, target.relation))
            };
            StatementDescriptor::Capacity {
                target,
                weight,
                lo: rng.range(4),
                hi,
                source,
            }
        }
    }
}

fn random_weight(rng: &mut Rng, relations: &[RelationDescriptor], source: RelationId) -> Weight {
    let span = field_span(relations, source) + 1;
    match rng.range(3) {
        0 => Weight::Unit,
        1 => Weight::Field(random_field_id(rng, span)),
        _ => Weight::DurationOf(random_field_id(rng, span)),
    }
}

/// A structurally-free ceiling: literal, dependent u64 field, or dependent
/// Duration off the TARGET's row — dependent bounds are hi-slot only by
/// representation (C6: the descriptor's `lo` is a bare literal), so only the
/// ceiling draws the ident forms; the span is one past the target's so the
/// dangling-field refusal stays reachable.
fn random_bound(rng: &mut Rng, relations: &[RelationDescriptor], target: RelationId) -> Bound {
    let span = field_span(relations, target) + 1;
    match rng.range(4) {
        0 | 1 => Bound::Lit(rng.range(5)),
        2 => Bound::TargetField(random_field_id(rng, span)),
        _ => Bound::TargetDuration(random_field_id(rng, span)),
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
    let len = draw(rng, 4);
    (0..len).map(|_| random_field_id(rng, span)).collect()
}

fn random_field_id(rng: &mut Rng, span: u64) -> FieldId {
    let id = if span > 0 && rng.chance(7, 8) {
        rng.range(span)
    } else {
        rng.range(span + 3)
    };
    FieldId(u16::try_from(id).expect("field id fits u16"))
}

fn field_span(relations: &[RelationDescriptor], relation: RelationId) -> u64 {
    relations
        .get(usize::try_from(relation.0).expect("relation id fits usize"))
        .map_or(0, |rel| {
            u64::try_from(rel.fields.len()).expect("field count fits u64")
        })
}

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

    #[test]
    fn the_descriptor_sweep_reaches_every_statement_form_without_panicking() {
        use bumbledb::schema::{LiteralSet, StatementDescriptor};
        use std::panic::{AssertUnwindSafe, catch_unwind};

        const SWEEP: u64 = 12_000;
        let mut accepted = 0u64;
        let mut rejected = 0u64;
        let mut functionality = 0u64;
        let mut containment = 0u64;
        let mut capacity_bounded = 0u64;
        let mut capacity_star = 0u64;
        let mut weight_unit = 0u64;
        let mut weight_field = 0u64;
        let mut weight_duration = 0u64;
        let mut dependent_bound = 0u64;
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
                    StatementDescriptor::Capacity { weight, hi, .. } => {
                        match hi {
                            Some(bound) => {
                                capacity_bounded += 1;
                                if !matches!(bound, bumbledb::schema::Bound::Lit(_)) {
                                    dependent_bound += 1;
                                }
                            }
                            None => capacity_star += 1,
                        }
                        match weight {
                            bumbledb::schema::Weight::Unit => weight_unit += 1,
                            bumbledb::schema::Weight::Field(_) => weight_field += 1,
                            bumbledb::schema::Weight::DurationOf(_) => weight_duration += 1,
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
            ("bounded capacity", capacity_bounded),
            ("star capacity", capacity_star),
            ("unit weight", weight_unit),
            ("field weight", weight_field),
            ("duration weight", weight_duration),
            ("dependent bound", dependent_bound),
            ("set-selection", set_selection),
        ] {
            assert!(count > 0, "the sweep never reached: {label}");
        }
        eprintln!(
            "sweep: {accepted} accepted / {rejected} rejected; forms: fd {functionality}, \
             ind {containment}, capacity {capacity_bounded}+{capacity_star}* \
             (w {weight_unit}/{weight_field}/{weight_duration}, dep {dependent_bound}), \
             sets {set_selection}"
        );
    }
}
