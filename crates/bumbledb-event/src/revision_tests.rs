use crate::{
    ArithmeticLimits, BoolOp4, Capacity, Control, DensityPiece, Error, Event, EventPartition,
    ExactArithmetic, ExactRational, FiniteFunction, FunctionLimits, FunctionPiece, LawLimits,
    PartitionLimits, RevisionImpossible, RevisionOutcome, RevisionReceipt, SourceRevision, Space,
    SpaceId,
};

struct Stop;
impl Control for Stop {
    fn checkpoint(&self) -> crate::Result<()> {
        Err(Error::Cancelled)
    }
}

fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn ratio(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn source(masses: [i64; 4]) -> Space {
    let raw = Space::new(SpaceId([123; 32]), 2, &()).unwrap();
    let total = u64::try_from(masses.iter().sum::<i64>()).unwrap();
    let pieces: Vec<_> = masses
        .into_iter()
        .enumerate()
        .map(|(world, mass)| DensityPiece {
            region: raw.table(3, &[1 << world], &()).unwrap(),
            density: ratio(mass, total),
        })
        .collect();
    raw.with_density(&pieces, LawLimits::default(), &mut work())
        .unwrap()
}
fn function(space: &Space, values: [i64; 4]) -> FiniteFunction {
    let pieces: Vec<_> = values
        .into_iter()
        .enumerate()
        .map(|(world, value)| FunctionPiece {
            region: space.table(3, &[1 << world], &()).unwrap(),
            value: value.into(),
        })
        .collect();
    FiniteFunction::new(space, &pieces, FunctionLimits::default(), &mut work()).unwrap()
}
fn partition(space: &Space, bits: u64) -> EventPartition {
    let event = space.table(3, &[bits], &()).unwrap();
    EventPartition::on(
        &space.full(),
        &[event.clone(), event.complement()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap()
}
fn jeffrey(space: &Space, cells: &EventPartition, targets: &[ExactRational]) -> SourceRevision {
    space
        .jeffrey(
            cells,
            targets,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut work(),
        )
        .unwrap()
}
fn likelihood(space: &Space, factor: &FiniteFunction) -> SourceRevision {
    space
        .likelihood(
            factor,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut work(),
        )
        .unwrap()
}

#[test]
fn conditioning_and_signed_expectation_match_every_four_world_evidence() {
    let values = [-3, 0, 5, 2];
    let mut cases = 0;
    for code in 1..81u32 {
        let masses =
            std::array::from_fn(|i| i64::from(code / 3u32.pow(u32::try_from(i).unwrap()) % 3));
        let total = u64::try_from(masses.iter().sum::<i64>()).unwrap();
        let prior = source(masses);
        let original = prior.full().to_bytes(&()).unwrap();
        let payoff = function(&prior, values);
        for mask in 0..16 {
            let evidence = prior.table(3, &[mask], &()).unwrap();
            let z: i64 = (0..4)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| masses[i])
                .sum();
            let numerator: i64 = (0..4)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| masses[i] * values[i])
                .sum();
            let observation = payoff.expectation(&evidence, &mut work()).unwrap();
            assert_eq!(observation.evidence(), &evidence);
            assert_eq!(observation.numerator(), &ratio(numerator, total));
            assert_eq!(observation.evidence_mass(), &ratio(z, total));
            assert_eq!(
                observation.value(&mut work()).unwrap(),
                (z != 0).then(|| ratio(numerator, z.cast_unsigned()))
            );
            let update = prior
                .condition(
                    &evidence,
                    FunctionLimits::default(),
                    LawLimits::default(),
                    &mut work(),
                )
                .unwrap();
            let RevisionReceipt::Condition {
                evidence: retained,
                mass,
            } = update.receipt()
            else {
                panic!("conditioning")
            };
            assert_eq!(retained, &evidence);
            assert_eq!(mass, &ratio(z, total));
            if z == 0 {
                assert!(matches!(
                    update.outcome(),
                    RevisionOutcome::Impossible(RevisionImpossible::ZeroEvidence)
                ));
            } else {
                let revised = update.revised().unwrap();
                assert_eq!(revised.space().full().count(&()).unwrap(), 4);
                for (i, old) in masses.iter().enumerate() {
                    let cell = prior.table(3, &[1 << i], &()).unwrap();
                    let next = revised.translation().pullback(&cell, &()).unwrap();
                    let expected = if mask & (1 << i) != 0 { *old } else { 0 };
                    assert_eq!(
                        next.mass(&mut work()).unwrap(),
                        ratio(expected, z.cast_unsigned())
                    );
                    assert!(!next.is_empty());
                }
                let again = revised
                    .space()
                    .condition(
                        &revised.translation().pullback(&evidence, &()).unwrap(),
                        FunctionLimits::default(),
                        LawLimits::default(),
                        &mut work(),
                    )
                    .unwrap();
                assert_eq!(
                    again
                        .revised()
                        .unwrap()
                        .space()
                        .full()
                        .to_bytes(&())
                        .unwrap(),
                    revised.space().full().to_bytes(&()).unwrap()
                );
            }
            cases += 1;
        }
        assert_eq!(prior.full().to_bytes(&()).unwrap(), original);
    }
    assert_eq!(cases, 1280);
}

#[test]
fn jeffrey_preserves_conditionals_targets_and_same_partition_idempotence() {
    let mut cases = 0;
    for masses in [[1, 2, 3, 4], [0, 1, 0, 3], [2, 0, 0, 0]] {
        let prior = source(masses);
        for mask in 0..16 {
            let cells = partition(&prior, mask);
            let old: Vec<_> = cells
                .cells()
                .iter()
                .map(|e| e.mass(&mut work()).unwrap())
                .collect();
            for q in 0..=4 {
                let targets = [ratio(q, 4), ratio(4 - q, 4)];
                let update = jeffrey(&prior, &cells, &targets);
                let RevisionReceipt::Jeffrey {
                    partition: retained,
                    old_masses,
                    targets: new,
                } = update.receipt()
                else {
                    panic!("Jeffrey")
                };
                assert_eq!(retained.cells(), cells.cells());
                assert_eq!(&**old_masses, &old);
                assert_eq!(&**new, &targets);
                let bad: Vec<_> = (0..2)
                    .filter(|&i| old[i].is_zero() && !targets[i].is_zero())
                    .collect();
                if bad.is_empty() {
                    let revised = update.revised().unwrap();
                    let new_cells = cells.pullback(revised.translation(), &()).unwrap();
                    for (i, target) in targets.iter().enumerate() {
                        assert_eq!(new_cells.cells()[i].mass(&mut work()).unwrap(), targets[i]);
                        if !target.is_zero() {
                            let bit = prior.coordinate(0, &()).unwrap();
                            let next = revised.translation().pullback(&bit, &()).unwrap();
                            assert_eq!(
                                next.probability(&new_cells.cells()[i], &mut work())
                                    .unwrap()
                                    .value(&mut work())
                                    .unwrap(),
                                bit.probability(&cells.cells()[i], &mut work())
                                    .unwrap()
                                    .value(&mut work())
                                    .unwrap()
                            );
                        }
                    }
                    let again = jeffrey(revised.space(), &new_cells, &targets);
                    assert_eq!(
                        again
                            .revised()
                            .unwrap()
                            .space()
                            .full()
                            .to_bytes(&())
                            .unwrap(),
                        revised.space().full().to_bytes(&()).unwrap()
                    );
                } else {
                    assert!(
                        matches!(update.outcome(), RevisionOutcome::Impossible(RevisionImpossible::UnsupportedTargets { cells }) if **cells == bad)
                    );
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 240);
}

#[test]
fn posterior_assessment_is_not_a_likelihood_and_scale_is_retained() {
    let prior = source([2, 1, 2, 0]);
    let cells = partition(&prior, 0b1010);
    let update = jeffrey(&prior, &cells, &[ratio(4, 5), ratio(1, 5)]);
    let revised = update.revised().unwrap();
    assert_eq!(
        revised
            .translation()
            .pullback(&cells.cells()[0], &())
            .unwrap()
            .mass(&mut work())
            .unwrap(),
        ratio(4, 5)
    );
    let factor = function(&prior, [1, 4, 1, 4]);
    let scaled = likelihood(&prior, &factor);
    let small = FiniteFunction::new(
        &prior,
        &[
            FunctionPiece {
                region: cells.cells()[0].clone(),
                value: ratio(4, 5),
            },
            FunctionPiece {
                region: cells.cells()[1].clone(),
                value: ratio(1, 5),
            },
        ],
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    let unscaled = likelihood(&prior, &small);
    let RevisionReceipt::Likelihood {
        likelihood: retained,
        normalizer,
    } = scaled.receipt()
    else {
        panic!("likelihood")
    };
    assert!(retained.equivalent(&factor, &()).unwrap());
    assert_eq!(normalizer, &ratio(8, 5)); // not a probability of an observation
    let RevisionReceipt::Likelihood { normalizer, .. } = unscaled.receipt() else {
        panic!("likelihood")
    };
    assert_eq!(normalizer, &ratio(8, 25));
    let revised = scaled.revised().unwrap();
    assert_eq!(
        revised
            .translation()
            .pullback(&cells.cells()[0], &())
            .unwrap()
            .mass(&mut work())
            .unwrap(),
        ratio(1, 2)
    );
    assert_eq!(
        revised.space().full().to_bytes(&()).unwrap(),
        unscaled
            .revised()
            .unwrap()
            .space()
            .full()
            .to_bytes(&())
            .unwrap()
    );
    assert_eq!(
        prior.full().align_to(revised.space(), &()).err(),
        Some(Error::SpaceMismatch)
    );
}

#[test]
fn fixed_likelihoods_commute_but_overlapping_jeffrey_updates_need_not() {
    let prior = source([1, 2, 3, 4]);
    let first = function(&prior, [1, 3, 2, 0]);
    let second = function(&prior, [2, 1, 0, 4]);
    let sequential = |a: &FiniteFunction, b: &FiniteFunction| {
        let update = likelihood(&prior, a);
        let next = update.revised().unwrap();
        let b = b
            .pullback(next.translation(), FunctionLimits::default(), &mut work())
            .unwrap();
        likelihood(next.space(), &b)
            .revised()
            .unwrap()
            .space()
            .full()
            .to_bytes(&())
            .unwrap()
    };
    assert_eq!(sequential(&first, &second), sequential(&second, &first));
    let product = first
        .multiply(&second, FunctionLimits::default(), &mut work())
        .unwrap();
    assert_eq!(
        sequential(&first, &second),
        likelihood(&prior, &product)
            .revised()
            .unwrap()
            .space()
            .full()
            .to_bytes(&())
            .unwrap()
    );
    let a = partition(&prior, 0b1010);
    let b = partition(&prior, 0b1100);
    let targets = [ratio(3, 4), ratio(1, 4)];
    let sequence = |a: &EventPartition, b: &EventPartition| {
        let update = jeffrey(&prior, a, &targets);
        let next = update.revised().unwrap();
        let b = b.pullback(next.translation(), &()).unwrap();
        jeffrey(next.space(), &b, &targets)
            .revised()
            .unwrap()
            .space()
            .full()
            .to_bytes(&())
            .unwrap()
    };
    assert_ne!(sequence(&a, &b), sequence(&b, &a));
}

#[test]
fn impossible_revisions_retain_zero_mass_and_empty_cells_without_division() {
    let prior = source([1, 0, 0, 0]);
    let cells: Vec<_> = (0..4)
        .map(|i| prior.table(3, &[1 << i], &()).unwrap())
        .chain([prior.empty()])
        .collect();
    let partition =
        EventPartition::on(&prior.full(), &cells, PartitionLimits::default(), &()).unwrap();
    let impossible = jeffrey(
        &prior,
        &partition,
        &[
            ratio(0, 1),
            ratio(1, 3),
            ratio(0, 1),
            ratio(1, 3),
            ratio(1, 3),
        ],
    );
    assert!(
        matches!(impossible.outcome(), RevisionOutcome::Impossible(RevisionImpossible::UnsupportedTargets { cells }) if **cells == [1, 3, 4])
    );
    assert!(impossible.prior().is_measured());
    let identity = jeffrey(
        &prior,
        &partition,
        &[
            ratio(1, 1),
            ratio(0, 1),
            ratio(0, 1),
            ratio(0, 1),
            ratio(0, 1),
        ],
    );
    assert!(identity.revised().is_some());
    let zero = function(&prior, [0, 3, 2, 1]);
    let impossible = likelihood(&prior, &zero);
    drop((prior, zero));
    assert!(matches!(
        impossible.outcome(),
        RevisionOutcome::Impossible(RevisionImpossible::ZeroLikelihood)
    ));
    let RevisionReceipt::Likelihood {
        likelihood,
        normalizer,
    } = impossible.receipt()
    else {
        panic!("likelihood")
    };
    assert_eq!(likelihood.at(1, &()).unwrap(), 3u64.into());
    assert!(normalizer.is_zero());
}

#[test]
fn signed_expectation_is_linear_and_owned_and_indicator_matches_probability() {
    let prior = source([1, 2, 3, 4]);
    let evidence = prior.coordinate(1, &()).unwrap();
    let a = function(&prior, [-8, 0, -4, 2]);
    let b = function(&prior, [1, -3, 5, -7]);
    let sum = a.add(&b, FunctionLimits::default(), &mut work()).unwrap();
    let x = a.expectation(&evidence, &mut work()).unwrap();
    let y = b.expectation(&evidence, &mut work()).unwrap();
    let combined = sum.expectation(&evidence, &mut work()).unwrap();
    assert_eq!(
        combined.value(&mut work()).unwrap(),
        Some(
            x.value(&mut work())
                .unwrap()
                .unwrap()
                .add(&y.value(&mut work()).unwrap().unwrap(), &mut work())
                .unwrap()
        )
    );
    let indicator = function(&prior, [0, 1, 0, 1]);
    assert_eq!(
        indicator
            .expectation(&evidence, &mut work())
            .unwrap()
            .value(&mut work())
            .unwrap(),
        prior
            .coordinate(0, &())
            .unwrap()
            .probability(&evidence, &mut work())
            .unwrap()
            .value(&mut work())
            .unwrap()
    );
    let restored = Event::from_bytes(&evidence.to_bytes(&()).unwrap(), &()).unwrap();
    assert_eq!(
        a.expectation(&restored, &mut work())
            .unwrap()
            .value(&mut work())
            .unwrap(),
        x.value(&mut work()).unwrap()
    );
    drop((prior, a, b, sum, evidence));
    assert_eq!(x.value(&mut work()).unwrap(), Some(ratio(-4, 7)));
    assert_eq!(x.function().at(2, &()).unwrap(), (-4i64).into());
}

#[test]
fn revisions_keep_restricted_support_aliases_and_large_symbolic_worlds() {
    let raw = Space::new(SpaceId([45; 32]), 62, &()).unwrap();
    let legal = raw
        .coordinate(0, &())
        .unwrap()
        .apply(BoolOp4::OR, &raw.coordinate(61, &()).unwrap(), &())
        .unwrap();
    let restricted = raw.restrict(&legal, &()).unwrap();
    let size = 3u64 * (1u64 << 60);
    let prior = restricted
        .with_density(
            &[DensityPiece {
                region: restricted.full(),
                density: ratio(1, size),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let evidence = prior.coordinate(0, &()).unwrap();
    let update = prior
        .condition(
            &evidence,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let revised = update.revised().unwrap();
    assert_eq!(revised.space().full().count(&()).unwrap(), size);
    assert_eq!(
        revised.space().full().contains(0),
        Err(Error::IllegalWorld(0))
    );
    let x = revised.translation().pullback(&evidence, &()).unwrap();
    assert_eq!(x.mass(&mut work()).unwrap(), ratio(1, 1));
    assert!(!x.is_full());
    assert!(!x.complement().is_empty());
    assert_eq!(x.complement().mass(&mut work()).unwrap(), ratio(0, 1));
    assert!(x.to_bytes(&()).unwrap().len() < 600);
}

#[test]
fn revisions_refuse_malformed_inputs_contexts_missing_laws_and_resources() {
    let prior = source([1, 2, 3, 4]);
    let cells = partition(&prior, 0b1010);
    let call = |p: &EventPartition, q: &[ExactRational]| {
        prior
            .jeffrey(
                p,
                q,
                FunctionLimits::default(),
                LawLimits::default(),
                &mut work(),
            )
            .err()
    };
    assert_eq!(call(&cells, &[ratio(1, 1)]), Some(Error::PartitionArity));
    assert_eq!(
        call(&cells, &[ratio(-1, 1), ratio(2, 1)]),
        Some(Error::NegativeMass)
    );
    assert_eq!(
        call(&cells, &[ratio(1, 5), ratio(1, 5)]),
        Some(Error::LawNotNormalized)
    );
    let partial = EventPartition::on(
        &cells.cells()[0],
        &[cells.cells()[0].clone()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    assert_eq!(call(&partial, &[ratio(1, 1)]), Some(Error::PartitionGap));
    let foreign = Space::new(SpaceId([1; 32]), 2, &()).unwrap();
    let zero = function(&prior, [0; 4]);
    assert_eq!(
        zero.expectation(&foreign.empty(), &mut work()).err(),
        Some(Error::SpaceMismatch)
    );
    assert_eq!(
        prior
            .condition(
                &foreign.empty(),
                FunctionLimits::default(),
                LawLimits::default(),
                &mut work()
            )
            .err(),
        Some(Error::SpaceMismatch)
    );
    assert_eq!(
        foreign
            .likelihood(
                &zero,
                FunctionLimits::default(),
                LawLimits::default(),
                &mut work()
            )
            .err(),
        Some(Error::SpaceMismatch)
    );
    let absent = function(&foreign, [0; 4]);
    assert_eq!(
        absent.expectation(&foreign.full(), &mut work()).err(),
        Some(Error::MissingLaw)
    );
    assert_eq!(
        foreign
            .likelihood(
                &absent,
                FunctionLimits::default(),
                LawLimits::default(),
                &mut work()
            )
            .err(),
        Some(Error::MissingLaw)
    );
    let negative = function(&prior, [0, -1, 0, 0]);
    assert_eq!(
        prior
            .likelihood(
                &negative,
                FunctionLimits::default(),
                LawLimits::default(),
                &mut work()
            )
            .err(),
        Some(Error::NegativeMass)
    );
}

#[test]
fn revisions_refuse_resources_and_cancellation() {
    let prior = source([1, 2, 3, 4]);
    let cells = partition(&prior, 0b1010);
    let zero = function(&prior, [0; 4]);
    assert_eq!(
        prior
            .condition(
                &prior.full(),
                FunctionLimits::default(),
                LawLimits {
                    cells: 0,
                    ..LawLimits::default()
                },
                &mut work()
            )
            .err(),
        Some(Error::Capacity(Capacity::LawCells))
    );
    assert_eq!(
        prior
            .jeffrey(
                &cells,
                &[ratio(1, 2), ratio(1, 2)],
                FunctionLimits {
                    cells: 1,
                    ..FunctionLimits::default()
                },
                LawLimits::default(),
                &mut work()
            )
            .err(),
        Some(Error::Capacity(Capacity::PartitionCells))
    );
    let mut tiny = ExactArithmetic::new(
        ArithmeticLimits {
            operations: 0,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert_eq!(
        zero.expectation(&prior.full(), &mut tiny).err(),
        Some(Error::Capacity(Capacity::ArithmeticSteps))
    );
    let mut cancelled = ExactArithmetic::new(ArithmeticLimits::default(), &Stop);
    assert_eq!(
        prior
            .condition(
                &prior.empty(),
                FunctionLimits::default(),
                LawLimits::default(),
                &mut cancelled
            )
            .err(),
        Some(Error::Cancelled)
    );
    assert_eq!(
        zero.expectation(&prior.full(), &mut cancelled).err(),
        Some(Error::Cancelled)
    );
}
