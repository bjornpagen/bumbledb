use crate::parameter_source_tests::{c, domain, limits, p, ratio, sign, space, sub, work};
use crate::*;

fn finite(source: &Space, values: [ExactRational; 2]) -> FiniteFunction {
    FiniteFunction::new(
        source,
        &values
            .into_iter()
            .enumerate()
            .map(|(i, value)| FunctionPiece {
                region: source.table(1, &[1 << i], &()).unwrap(),
                value,
            })
            .collect::<Vec<_>>(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap()
}
fn patch<F>(region: Event, function: F) -> FunctionPatch<F> {
    FunctionPatch { region, function }
}
fn parameter(source: &Space, polynomial: ExactPolynomial) -> FamilyFunction {
    FamilyFunction::constant(
        source,
        GuardedRationalFunction::new(
            domain(),
            polynomial,
            c(1),
            limits().parameters.region,
            &mut work(),
        )
        .unwrap(),
        limits(),
        &mut work(),
    )
    .unwrap()
}

#[test]
fn finite_covers_match_complete_pointwise_oracle_for_all_two_world_patches() {
    let source = Space::new(SpaceId([210; 32]), 1, &()).unwrap();
    let value = |code: u64, world: u64| {
        if code & (1 << world) == 0 {
            ratio("-2", "3")
        } else {
            ratio("7", "5")
        }
    };
    for left_value in 0..4 {
        for right_value in 0..4 {
            let left = finite(&source, [value(left_value, 0), value(left_value, 1)]);
            let right = finite(&source, [value(right_value, 0), value(right_value, 1)]);
            for left_region in 0..4 {
                for right_region in 0..4 {
                    for parent in 0..4 {
                        let regions = [left_region, right_region];
                        let given = source.table(1, &[parent], &()).unwrap();
                        let patches = [
                            patch(source.table(1, &[left_region], &()).unwrap(), left.clone()),
                            patch(
                                source.table(1, &[right_region], &()).unwrap(),
                                right.clone(),
                            ),
                        ];
                        let covered = (left_region | right_region) & parent == parent;
                        let agrees = (0..2).all(|w| {
                            parent & left_region & right_region & (1 << w) == 0
                                || value(left_value, w) == value(right_value, w)
                        });
                        let result = FiniteFunction::glue(
                            &given,
                            &patches,
                            FunctionLimits::default(),
                            &mut work(),
                        );
                        assert_eq!(result.is_ok(), covered && agrees);
                        if let Ok(cover) = result {
                            assert_eq!(cover.parent(), &given);
                            assert_eq!(cover.patches().len(), 2);
                            for w in 0..2 {
                                let expected = if parent & (1 << w) == 0 {
                                    ExactRational::zero()
                                } else {
                                    let index =
                                        regions.iter().position(|r| r & (1 << w) != 0).unwrap();
                                    value([left_value, right_value][index], w)
                                };
                                assert_eq!(cover.function().at(w, &()).unwrap(), expected);
                            }
                            let reversed = [patches[1].clone(), patches[0].clone()];
                            let reordered = FiniteFunction::glue(
                                &given,
                                &reversed,
                                FunctionLimits::default(),
                                &mut work(),
                            )
                            .unwrap();
                            assert!(
                                cover
                                    .function()
                                    .equivalent(reordered.function(), &())
                                    .unwrap()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn finite_cover_preserves_supplied_zeros_and_refuses_zero_mass_gaps_or_conflicts() {
    let raw = Space::new(SpaceId([211; 32]), 1, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.coordinate(0, &()).unwrap(),
                density: 1u64.into(),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let zero = FiniteFunction::constant(
        &source,
        ExactRational::zero(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    let one = FiniteFunction::constant(
        &source,
        ExactRational::one(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    let positive = source.coordinate(0, &()).unwrap();
    let null = positive.complement();
    assert!(matches!(
        FiniteFunction::glue(
            &source.full(),
            &[patch(positive.clone(), zero.clone())],
            FunctionLimits::default(),
            &mut work()
        ),
        Err(Error::FunctionCoverGap)
    ));
    assert!(matches!(
        FiniteFunction::glue(
            &source.full(),
            &[
                patch(source.full(), zero.clone()),
                patch(null.clone(), one.clone())
            ],
            FunctionLimits::default(),
            &mut work()
        ),
        Err(Error::FunctionCoverConflict)
    ));
    let cover = FiniteFunction::glue(
        &positive,
        &[
            patch(source.full(), zero.clone()),
            patch(null, one),
            patch(source.empty(), zero),
        ],
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(cover.patches().len(), 3);
    assert!(cover.function().is_zero());
    assert!(cover.patches()[0].region.is_full());
    assert!(cover.patches()[2].region.is_empty());
    assert!(
        FiniteFunction::glue(&source.empty(), &[], FunctionLimits::default(), &mut work())
            .unwrap()
            .function()
            .is_zero()
    );
}

#[test]
fn family_cover_checks_actual_guard_points_without_requiring_global_function_equality() {
    let twice = p()
        .mul(&c(2), limits().parameters.region.polynomial, &mut work())
        .unwrap();
    let threshold = sub(&twice, &c(1));
    let source = space(
        212,
        2,
        &[
            ParameterGuard {
                coordinate: 0,
                region: sign(&threshold, PolynomialSigns::NON_POSITIVE),
            },
            ParameterGuard {
                coordinate: 1,
                region: sign(&threshold, PolynomialSigns::NON_NEGATIVE),
            },
        ],
    );
    let left = source.coordinate(0, &()).unwrap();
    let right = source.coordinate(1, &()).unwrap();
    let rising = parameter(&source, p());
    let falling = parameter(&source, sub(&c(1), &p()));
    assert!(!rising.equivalent(&falling, limits(), &mut work()).unwrap());
    let roster = [
        patch(left.clone(), rising.clone()),
        patch(right.clone(), falling.clone()),
    ];
    let cover = FamilyFunction::glue(&source.full(), &roster, limits(), &mut work()).unwrap();
    for (n, d, expected) in [
        (0, 1, "0"),
        (1, 4, "1/4"),
        (1, 2, "1/2"),
        (3, 4, "1/4"),
        (1, 1, "0"),
    ] {
        assert_eq!(
            cover
                .function()
                .value_at(
                    &ratio(&n.to_string(), &d.to_string()),
                    0,
                    limits(),
                    &mut work()
                )
                .unwrap()
                .unwrap()
                .to_string(),
            expected
        );
    }
    let reversed = FamilyFunction::glue(
        &source.full(),
        &[roster[1].clone(), roster[0].clone()],
        limits(),
        &mut work(),
    )
    .unwrap();
    assert!(
        cover
            .function()
            .equivalent(reversed.function(), limits(), &mut work())
            .unwrap()
    );
    let overlap = left.apply(BoolOp4::AND, &right, &()).unwrap();
    let point_wrong = parameter(&source, c(7));
    assert!(matches!(
        FamilyFunction::glue(
            &source.full(),
            &[
                roster[0].clone(),
                roster[1].clone(),
                patch(overlap, point_wrong)
            ],
            limits(),
            &mut work()
        ),
        Err(Error::FunctionCoverConflict)
    ));
    assert!(matches!(
        FamilyFunction::glue(
            &source.full(),
            &[
                patch(left, rising),
                patch(
                    right,
                    falling
                        .mask(&source.empty(), limits(), &mut work())
                        .unwrap()
                )
            ],
            limits(),
            &mut work()
        ),
        Err(Error::FunctionCoverConflict)
    ));
}

#[test]
fn covers_validate_empty_functions_and_regions_before_shortcuts_and_keep_owners() {
    let source = Space::new(SpaceId([213; 32]), 62, &()).unwrap();
    let foreign = Space::new(SpaceId([214; 32]), 62, &()).unwrap();
    let one = FiniteFunction::constant(
        &source,
        ExactRational::one(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    let foreign_zero = FiniteFunction::constant(
        &foreign,
        ExactRational::zero(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    for bad in [
        patch(source.empty(), foreign_zero),
        patch(foreign.empty(), one.clone()),
    ] {
        assert!(matches!(
            FiniteFunction::glue(
                &source.full(),
                &[patch(source.full(), one.clone()), bad],
                FunctionLimits::default(),
                &mut work()
            ),
            Err(Error::SpaceMismatch)
        ));
    }
    let detached = Event::from_bytes(&source.full().to_bytes(&()).unwrap(), &()).unwrap();
    let cover = FiniteFunction::glue(
        &detached,
        &[patch(source.full(), one)],
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    drop((source, foreign, detached));
    assert_eq!(
        cover.function().at((1 << 62) - 1, &()).unwrap(),
        ExactRational::one()
    );
    let low = FunctionLimits {
        steps: 0,
        ..FunctionLimits::default()
    };
    assert!(matches!(
        FiniteFunction::glue(cover.parent(), cover.patches(), low, &mut work()),
        Err(Error::Capacity(Capacity::FunctionSteps))
    ));
    let low = FunctionLimits {
        cells: 0,
        ..FunctionLimits::default()
    };
    assert!(matches!(
        FiniteFunction::glue(cover.parent(), cover.patches(), low, &mut work()),
        Err(Error::Capacity(Capacity::FunctionCells))
    ));
    let mut cancelled = ExactArithmetic::new(ArithmeticLimits::default(), &Cancel);
    assert!(matches!(
        FiniteFunction::glue(
            cover.parent(),
            cover.patches(),
            FunctionLimits::default(),
            &mut cancelled
        ),
        Err(Error::Cancelled)
    ));
}

struct Cancel;
impl Control for Cancel {
    fn checkpoint(&self) -> Result<()> {
        Err(Error::Cancelled)
    }
}
