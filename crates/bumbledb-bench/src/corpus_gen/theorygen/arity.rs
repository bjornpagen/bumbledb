//! Projection-arity coverage cases layered after the structurally-free
//! descriptor arm. The legacy arm's decisions are deliberately untouched:
//! callers finish [`super::random_descriptor`] (or the fixed ops scenario)
//! before drawing one of these cases through a fresh [`Rng`] cursor. The
//! cursor separation keeps late coverage live even when the legacy generator
//! exhausted its cursor, while call order keeps every legacy decision fixed.

use bumbledb::Value;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};

use crate::naive::Delta;

use super::super::Rng;

/// The storage key budget, derived in `storage::keys` as `511 - 15`.
/// Storage is intentionally crate-private; this bench-side coverage constant
/// is pinned by the generated over-width diagnostic in the seeded sweep.
pub const ARITY_WIDTH_BOUND: usize = 496;

/// The five-type cycle reaches 470 bytes at arity 29; its next `bytes<64>`
/// field reaches 534 and is the first illegal arity for this mix.
pub const MAX_MIXED_ARITY: usize = max_mixed_arity();

const SOURCE: RelationId = RelationId(0);
const TARGET: RelationId = RelationId(1);

/// Which containment sides carry a non-projected boolean selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPlacement {
    Source,
    Target,
    Both,
}

/// The verdict class a generated theory case must receive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArityExpectation {
    Accepted,
    DeterminantKeyTooWide { width: usize },
    MissingSourceKey,
    MissingTargetKey,
}

/// Coverage facts carried beside a descriptor, never inferred by the oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArityCoverage {
    pub arity: usize,
    pub width: usize,
    pub type_counts: [usize; 5],
    pub selection: SelectionPlacement,
    pub equality: bool,
    pub reordered_key: bool,
    pub expectation: ArityExpectation,
}

/// One schema-acceptance case from the hostile arity arm.
#[derive(Debug, Clone)]
pub struct ArityDescriptorCase {
    pub descriptor: SchemaDescriptor,
    pub coverage: ArityCoverage,
}

/// One accepted theory plus a write stream for the ops parity oracle.
#[derive(Debug, Clone)]
pub struct ArityOpsCase {
    pub descriptor: SchemaDescriptor,
    pub deltas: Vec<Delta>,
    pub coverage: ArityCoverage,
}

/// A deterministic descriptor at one legal mixed-scalar arity.
///
/// # Panics
///
/// Panics when `arity` is outside `1..=MAX_MIXED_ARITY`.
#[must_use]
pub fn arity_descriptor(
    arity: usize,
    selection: SelectionPlacement,
    equality: bool,
) -> ArityDescriptorCase {
    assert!((1..=MAX_MIXED_ARITY).contains(&arity));
    build_case(arity, selection, equality, ArityExpectation::Accepted)
}

/// The hostile arm: accepted containments across every legal arity, accepted
/// keyed equalities at the constitutional arities, either missing equality
/// key, and the first over-width mixed projection.
///
/// # Panics
///
/// Only if the constitutional arity (at most 30) cannot fit `usize`.
#[must_use]
pub fn random_arity_descriptor(rng: &mut Rng) -> ArityDescriptorCase {
    let selection = random_selection(rng);
    match rng.range(8) {
        0 => build_case(
            MAX_MIXED_ARITY + 1,
            selection,
            false,
            ArityExpectation::DeterminantKeyTooWide {
                width: projection_width(MAX_MIXED_ARITY + 1),
            },
        ),
        1 => build_case(
            equality_arity(rng),
            selection,
            true,
            ArityExpectation::MissingSourceKey,
        ),
        2 => build_case(
            equality_arity(rng),
            selection,
            true,
            ArityExpectation::MissingTargetKey,
        ),
        3 | 4 => build_case(
            equality_arity(rng),
            selection,
            true,
            ArityExpectation::Accepted,
        ),
        _ => arity_descriptor(
            1 + usize::try_from(rng.range(MAX_MIXED_ARITY as u64)).expect("arity fits usize"),
            selection,
            false,
        ),
    }
}

/// The accepted sibling arm used by lifecycle generation.
///
/// # Panics
///
/// Only if the constitutional arity (at most 29) cannot fit `usize`.
#[must_use]
pub fn random_valid_arity_descriptor(rng: &mut Rng) -> ArityDescriptorCase {
    let selection = random_selection(rng);
    let equality = rng.chance(1, 3);
    let arity = if equality {
        equality_arity(rng)
    } else {
        1 + usize::try_from(rng.range(MAX_MIXED_ARITY as u64)).expect("arity fits usize")
    };
    arity_descriptor(arity, selection, equality)
}

/// A valid high-arity theory plus writes that commit once, then exercise a
/// functionality collision, a missing source witness, and target removal.
#[must_use]
pub fn random_valid_arity_ops(rng: &mut Rng) -> ArityOpsCase {
    let case = random_valid_arity_descriptor(rng);
    let arity = case.coverage.arity;
    let source = fact(arity, 7, 0);
    let target = fact(arity, 7, 0);
    let mut colliding_target = target.clone();
    colliding_target[arity + 2] = Value::U64(1);
    let missing_source = fact(arity, 91, 0);
    ArityOpsCase {
        descriptor: case.descriptor,
        deltas: vec![
            Delta {
                deletes: vec![],
                inserts: vec![(SOURCE, source.clone()), (TARGET, target.clone())],
            },
            Delta {
                deletes: vec![],
                inserts: vec![(TARGET, colliding_target)],
            },
            Delta {
                deletes: vec![],
                inserts: vec![(SOURCE, missing_source)],
            },
            Delta {
                deletes: vec![(TARGET, target)],
                inserts: vec![],
            },
        ],
        coverage: case.coverage,
    }
}

fn build_case(
    arity: usize,
    selection: SelectionPlacement,
    equality: bool,
    expectation: ArityExpectation,
) -> ArityDescriptorCase {
    let projection: Box<[FieldId]> = (0..arity).map(field_id).collect();
    let mut key_order = projection.to_vec();
    if arity >= 3 {
        key_order.reverse();
    }
    let source_side = side(SOURCE, &projection, arity, selection, true);
    let target_side = side(TARGET, &projection, arity, selection, false);
    let source_key = StatementDescriptor::Functionality {
        relation: SOURCE,
        projection: key_order.clone().into_boxed_slice(),
    };
    let target_key = StatementDescriptor::Functionality {
        relation: TARGET,
        projection: key_order.into_boxed_slice(),
    };
    let forward = StatementDescriptor::Containment {
        source: source_side.clone(),
        target: target_side.clone(),
    };
    let statements = if equality {
        match expectation {
            ArityExpectation::MissingSourceKey => vec![
                target_key,
                forward,
                StatementDescriptor::Containment {
                    source: target_side,
                    target: source_side,
                },
            ],
            ArityExpectation::MissingTargetKey => vec![
                source_key,
                forward,
                StatementDescriptor::Containment {
                    source: target_side,
                    target: source_side,
                },
            ],
            ArityExpectation::Accepted => vec![
                source_key,
                target_key,
                forward,
                StatementDescriptor::Containment {
                    source: target_side,
                    target: source_side,
                },
            ],
            ArityExpectation::DeterminantKeyTooWide { .. } => {
                unreachable!("the width case is a one-way containment")
            }
        }
    } else {
        vec![target_key, forward]
    };
    let types = projection_types(arity);
    ArityDescriptorCase {
        descriptor: SchemaDescriptor {
            relations: vec![
                relation("AritySource", &types),
                relation("ArityTarget", &types),
            ],
            statements,
        },
        coverage: ArityCoverage {
            arity,
            width: projection_width(arity),
            type_counts: type_counts(&types),
            selection,
            equality,
            reordered_key: arity >= 3,
            expectation,
        },
    }
}

fn relation(name: &str, types: &[ValueType]) -> RelationDescriptor {
    let mut fields: Vec<FieldDescriptor> = types
        .iter()
        .enumerate()
        .map(|(index, value_type)| FieldDescriptor {
            name: format!("key_{index}").into(),
            value_type: value_type.clone(),
            generation: Generation::None,
        })
        .collect();
    fields.extend([
        bool_field("source_filter"),
        bool_field("target_filter"),
        FieldDescriptor {
            name: "payload".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        },
    ]);
    RelationDescriptor {
        name: name.into(),
        fields,
        extension: None,
    }
}

fn bool_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::Bool,
        generation: Generation::None,
    }
}

fn side(
    relation: RelationId,
    projection: &[FieldId],
    arity: usize,
    placement: SelectionPlacement,
    source: bool,
) -> Side {
    let selected = match (placement, source) {
        (SelectionPlacement::Source | SelectionPlacement::Both, true) => Some(arity),
        (SelectionPlacement::Target | SelectionPlacement::Both, false) => Some(arity + 1),
        _ => None,
    };
    Side {
        relation,
        projection: projection.into(),
        selection: selected
            .map(|field| {
                Box::new([(
                    field_id(field),
                    bumbledb::schema::LiteralSet::One(Value::Bool(true)),
                )]) as Box<[_]>
            })
            .unwrap_or_default(),
    }
}

fn fact(arity: usize, discriminator: u64, payload: u64) -> Vec<Value> {
    let mut values: Vec<Value> = projection_types(arity)
        .iter()
        .enumerate()
        .map(|(index, value_type)| value(value_type, discriminator, index))
        .collect();
    values.extend([Value::Bool(true), Value::Bool(true), Value::U64(payload)]);
    values
}

fn value(value_type: &ValueType, discriminator: u64, index: usize) -> Value {
    let salt = discriminator.wrapping_mul(257).wrapping_add(index as u64);
    match value_type {
        ValueType::Bool => Value::Bool(salt & 1 == 0),
        ValueType::U64 => Value::U64(salt),
        ValueType::I64 => Value::I64(i64::try_from(salt).expect("small generated value")),
        ValueType::String => Value::String(format!("arity-{salt}").into_bytes().into()),
        ValueType::FixedBytes { len } => {
            Value::FixedBytes(vec![salt.to_le_bytes()[0]; usize::from(*len)].into())
        }
        ValueType::Interval { .. } => unreachable!("the arity mix is scalar"),
    }
}

fn projection_types(arity: usize) -> Vec<ValueType> {
    (0..arity).map(mixed_type).collect()
}

fn mixed_type(index: usize) -> ValueType {
    match index % 5 {
        0 => ValueType::U64,
        1 => ValueType::I64,
        2 => ValueType::Bool,
        3 => ValueType::String,
        _ => ValueType::FixedBytes { len: 64 },
    }
}

fn type_counts(types: &[ValueType]) -> [usize; 5] {
    let mut counts = [0; 5];
    for value_type in types {
        let index = match value_type {
            ValueType::U64 => 0,
            ValueType::I64 => 1,
            ValueType::Bool => 2,
            ValueType::String => 3,
            ValueType::FixedBytes { .. } => 4,
            ValueType::Interval { .. } => unreachable!("the arity mix is scalar"),
        };
        counts[index] += 1;
    }
    counts
}

fn projection_width(arity: usize) -> usize {
    projection_types(arity)
        .iter()
        .map(|value_type| value_type.type_desc().width())
        .sum()
}

const fn max_mixed_arity() -> usize {
    let widths = [8, 8, 1, 8, 64];
    let mut arity = 0;
    let mut width = 0;
    while width + widths[arity % widths.len()] <= ARITY_WIDTH_BOUND {
        width += widths[arity % widths.len()];
        arity += 1;
    }
    arity
}

fn equality_arity(rng: &mut Rng) -> usize {
    [1, 2, 3, MAX_MIXED_ARITY][usize::try_from(rng.range(4)).expect("equality arity index fits")]
}

fn random_selection(rng: &mut Rng) -> SelectionPlacement {
    match rng.range(3) {
        0 => SelectionPlacement::Source,
        1 => SelectionPlacement::Target,
        _ => SelectionPlacement::Both,
    }
}

fn field_id(index: usize) -> FieldId {
    FieldId(u16::try_from(index).expect("generated arity fits field id"))
}

#[cfg(test)]
mod tests {
    use bumbledb::error::SchemaError;

    use super::{
        ARITY_WIDTH_BOUND, ArityExpectation, MAX_MIXED_ARITY, SelectionPlacement, arity_descriptor,
        build_case, projection_width, random_arity_descriptor, random_valid_arity_descriptor,
    };
    use crate::corpus_gen::Rng;

    #[test]
    fn seeded_sweep_covers_every_legal_arity_type_selection_and_equality_shape() {
        let mut descriptors = 0;
        for arity in 1..=MAX_MIXED_ARITY {
            for selection in [
                SelectionPlacement::Source,
                SelectionPlacement::Target,
                SelectionPlacement::Both,
            ] {
                let case = arity_descriptor(arity, selection, false);
                assert!(case.descriptor.validate().is_ok(), "arity {arity}");
                assert_eq!(case.coverage.arity, arity);
                descriptors += 1;
            }
        }
        for arity in [1, 2, 3, MAX_MIXED_ARITY] {
            for selection in [
                SelectionPlacement::Source,
                SelectionPlacement::Target,
                SelectionPlacement::Both,
            ] {
                let valid = arity_descriptor(arity, selection, true);
                assert!(valid.descriptor.validate().is_ok());
                for expectation in [
                    ArityExpectation::MissingSourceKey,
                    ArityExpectation::MissingTargetKey,
                ] {
                    let rejected = build_case(arity, selection, true, expectation)
                        .descriptor
                        .validate();
                    assert!(matches!(
                        rejected,
                        Err(SchemaError::NoMatchingTargetKey { .. })
                    ));
                }
                descriptors += 3;
            }
        }
        assert_eq!(descriptors, MAX_MIXED_ARITY * 3 + 36);
    }

    #[test]
    fn a_few_hundred_seeded_cases_per_rng_arm_receive_the_promised_verdict() {
        let mut accepted_arities = [false; MAX_MIXED_ARITY + 1];
        let mut hostile_classes = [false; 4];
        for seed in 0..512 {
            let mut valid_rng = Rng::new(seed);
            let valid = random_valid_arity_descriptor(&mut valid_rng);
            accepted_arities[valid.coverage.arity] = true;
            assert!(valid.descriptor.validate().is_ok(), "valid seed {seed}");

            let mut hostile_rng = Rng::new(seed ^ 0x0A11_CE55);
            let hostile = random_arity_descriptor(&mut hostile_rng);
            let verdict = hostile.descriptor.validate();
            match hostile.coverage.expectation {
                ArityExpectation::Accepted => {
                    hostile_classes[0] = true;
                    assert!(verdict.is_ok(), "hostile accepted seed {seed}");
                }
                ArityExpectation::DeterminantKeyTooWide { width } => {
                    hostile_classes[1] = true;
                    assert!(matches!(
                        verdict,
                        Err(SchemaError::DeterminantKeyTooWide { width: actual, .. })
                            if actual == width
                    ));
                }
                ArityExpectation::MissingSourceKey => {
                    hostile_classes[2] = true;
                    assert!(matches!(
                        verdict,
                        Err(SchemaError::NoMatchingTargetKey { target, .. })
                            if target == super::SOURCE
                    ));
                }
                ArityExpectation::MissingTargetKey => {
                    hostile_classes[3] = true;
                    assert!(matches!(
                        verdict,
                        Err(SchemaError::NoMatchingTargetKey { target, .. })
                            if target == super::TARGET
                    ));
                }
            }
        }
        assert!(accepted_arities[1..].iter().all(|seen| *seen));
        assert!(hostile_classes.iter().all(|seen| *seen));
    }

    #[test]
    fn mixed_width_boundary_and_overflow_diagnostic_are_generated() {
        assert_eq!(MAX_MIXED_ARITY, 29);
        assert!(projection_width(MAX_MIXED_ARITY - 1) <= ARITY_WIDTH_BOUND);
        assert!(projection_width(MAX_MIXED_ARITY) <= ARITY_WIDTH_BOUND);
        let over_width = projection_width(MAX_MIXED_ARITY + 1);
        assert!(over_width > ARITY_WIDTH_BOUND);
        let over = build_case(
            MAX_MIXED_ARITY + 1,
            SelectionPlacement::Both,
            false,
            ArityExpectation::DeterminantKeyTooWide { width: over_width },
        );
        assert!(matches!(
            over.descriptor.validate(),
            Err(SchemaError::DeterminantKeyTooWide { width, .. }) if width == over_width
        ));
    }
}
