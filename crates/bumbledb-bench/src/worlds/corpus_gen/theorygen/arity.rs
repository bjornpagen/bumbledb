//! Projection-arity schema cases: a containment over every determinant arity
//! and type mix the sweep covers, each with the verdict the schema validator
//! must give it.
use bumbledb::Value;
use bumbledb::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};

use super::super::Rng;

/// The determinant width the arity sweep covers, not a schema restriction:
/// wider determinants use exact-checked fingerprint buckets.
pub const ARITY_COVERAGE_BYTES: usize = 496;

pub const MAX_COVERED_ARITY: usize = max_mixed_arity();

const SOURCE: RelationId = RelationId(0);
const TARGET: RelationId = RelationId(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPlacement {
    Source,
    Target,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArityExpectation {
    Accepted,
    WideDeterminant { width: usize },
    MissingSourceKey,
    MissingTargetKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArityCoverage {
    pub arity: usize,
    pub width: usize,
    pub type_counts: [usize; 6],
    pub selection: SelectionPlacement,
    pub equality: bool,
    pub reordered_key: bool,
    pub expectation: ArityExpectation,
}

#[derive(Debug, Clone)]
pub struct ArityDescriptorCase {
    pub descriptor: SchemaDescriptor,
    pub coverage: ArityCoverage,
}

#[must_use]
pub fn arity_descriptor(
    arity: usize,
    selection: SelectionPlacement,
    equality: bool,
) -> ArityDescriptorCase {
    assert!((1..=MAX_COVERED_ARITY).contains(&arity));
    build_case(arity, selection, equality, ArityExpectation::Accepted)
}

#[must_use]
pub fn random_arity_descriptor(rng: &mut Rng) -> ArityDescriptorCase {
    let selection = random_selection(rng);
    match rng.range(8) {
        0 => build_case(
            MAX_COVERED_ARITY + 1,
            selection,
            false,
            ArityExpectation::WideDeterminant {
                width: projection_width(MAX_COVERED_ARITY + 1),
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
            1 + usize::try_from(rng.range(MAX_COVERED_ARITY as u64)).expect("arity fits usize"),
            selection,
            false,
        ),
    }
}

#[must_use]
pub fn random_valid_arity_descriptor(rng: &mut Rng) -> ArityDescriptorCase {
    let selection = random_selection(rng);
    let equality = rng.chance(1, 3);
    let arity = if equality {
        equality_arity(rng)
    } else {
        1 + usize::try_from(rng.range(MAX_COVERED_ARITY as u64)).expect("arity fits usize")
    };
    arity_descriptor(arity, selection, equality)
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
            ArityExpectation::WideDeterminant { .. } => {
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
            value_type: *value_type,
        })
        .collect();
    fields.extend([
        bool_field("source_filter"),
        bool_field("target_filter"),
        FieldDescriptor {
            name: "payload".into(),
            value_type: ValueType::U64,
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
        selection: selected.map_or_default(|field| {
            Box::new([(
                field_id(field),
                bumbledb::schema::LiteralSet::One(Value::Bool(true)),
            )]) as Box<[_]>
        }),
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

fn type_counts(types: &[ValueType]) -> [usize; 6] {
    let mut counts = [0; 6];
    for value_type in types {
        let index = match value_type {
            ValueType::U64 => 0,
            ValueType::I64 => 1,
            ValueType::Bool => 2,
            ValueType::String => 3,
            ValueType::FixedBytes { .. } => 4,
            ValueType::F64 => 5,
            ValueType::Uuid | ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
                unreachable!("the arity mix draws no identity or interval columns")
            }
        };
        counts[index] += 1;
    }
    counts
}

fn projection_width(arity: usize) -> usize {
    projection_types(arity)
        .iter()
        .map(|value_type| value_type.width())
        .sum()
}

const fn max_mixed_arity() -> usize {
    let widths = [8, 8, 1, 8, 64];
    let mut arity = 0;
    let mut width = 0;
    while width + widths[arity % widths.len()] <= ARITY_COVERAGE_BYTES {
        width += widths[arity % widths.len()];
        arity += 1;
    }
    arity
}

fn equality_arity(rng: &mut Rng) -> usize {
    [1, 2, 3, MAX_COVERED_ARITY][usize::try_from(rng.range(4)).expect("equality arity index fits")]
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
    use bumbledb::error::{SchemaError, StatementErrorKind};
    use bumbledb::schema::ValidateDescriptor as _;

    use super::{
        ARITY_COVERAGE_BYTES, ArityExpectation, MAX_COVERED_ARITY, SelectionPlacement,
        arity_descriptor, build_case, projection_width, random_arity_descriptor,
        random_valid_arity_descriptor,
    };
    use crate::worlds::corpus_gen::Rng;

    #[test]
    fn seeded_sweep_covers_every_legal_arity_type_selection_and_equality_shape() {
        let mut descriptors = 0;
        for arity in 1..=MAX_COVERED_ARITY {
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
        for arity in [1, 2, 3, MAX_COVERED_ARITY] {
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
                        Err(SchemaError::Statement {
                            kind: StatementErrorKind::NoMatchingTargetKey { .. },
                            ..
                        })
                    ));
                }
                descriptors += 3;
            }
        }
        assert_eq!(descriptors, MAX_COVERED_ARITY * 3 + 36);
    }

    #[test]
    fn a_few_hundred_seeded_cases_per_rng_arm_receive_the_promised_verdict() {
        let mut accepted_arities = [false; MAX_COVERED_ARITY + 1];
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
                ArityExpectation::WideDeterminant { width } => {
                    hostile_classes[1] = true;
                    assert!(width > ARITY_COVERAGE_BYTES);
                    assert!(
                        verdict.is_ok(),
                        "wide determinants use collision-checked fingerprint buckets"
                    );
                }
                ArityExpectation::MissingSourceKey => {
                    hostile_classes[2] = true;
                    assert!(matches!(
                        verdict,
                        Err(SchemaError::Statement {
                            kind: StatementErrorKind::NoMatchingTargetKey { target, .. },
                            ..
                        }) if target == super::SOURCE
                    ));
                }
                ArityExpectation::MissingTargetKey => {
                    hostile_classes[3] = true;
                    assert!(matches!(
                        verdict,
                        Err(SchemaError::Statement {
                            kind: StatementErrorKind::NoMatchingTargetKey { target, .. },
                            ..
                        }) if target == super::TARGET
                    ));
                }
            }
        }
        assert!(accepted_arities[1..].iter().all(|seen| *seen));
        assert!(hostile_classes.iter().all(|seen| *seen));
    }

    #[test]
    fn wide_determinants_beyond_the_coverage_landmark_remain_legal() {
        assert_eq!(MAX_COVERED_ARITY, 29);
        assert!(projection_width(MAX_COVERED_ARITY - 1) <= ARITY_COVERAGE_BYTES);
        assert!(projection_width(MAX_COVERED_ARITY) <= ARITY_COVERAGE_BYTES);
        let over_width = projection_width(MAX_COVERED_ARITY + 1);
        assert!(over_width > ARITY_COVERAGE_BYTES);
        let over = build_case(
            MAX_COVERED_ARITY + 1,
            SelectionPlacement::Both,
            false,
            ArityExpectation::WideDeterminant { width: over_width },
        );
        assert!(
            over.descriptor.validate().is_ok(),
            "wide determinants remain legal through fingerprint routing"
        );
    }
}
