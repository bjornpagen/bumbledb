use crate::{
    ArithmeticLimits, BoolOp4, Capacity, DensityPiece, Error, Event, ExactArithmetic,
    ExactRational, LawLimits, Limits, Registry, Space, SpaceId,
};

fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn ratio(n: u64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn raw(order: &[u8]) -> Space {
    Space::with_order(SpaceId([47; 32]), order, Limits::default(), &()).unwrap()
}
fn piece(region: Event, n: u64, d: u64) -> DensityPiece {
    DensityPiece {
        region,
        density: ratio(n, d),
    }
}
fn measured(space: &Space, pieces: &[DensityPiece]) -> Space {
    space
        .with_density(pieces, LawLimits::default(), &mut work())
        .unwrap()
}

#[test]
fn finite_joint_contraction_matches_exhaustive_four_world_oracle() {
    let mut cases = 0;
    for support in 1u64..16 {
        let base = raw(&[0, 1]);
        let space = base
            .restrict(&base.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        for weights in 1u64..81 {
            let mass: Vec<_> = (0..4).map(|i| weights / 3u64.pow(i) % 3).collect();
            if (0..4).any(|i| support & (1 << i) == 0 && mass[i] != 0) {
                continue;
            }
            let total: u64 = mass.iter().sum();
            if total == 0 {
                continue;
            }
            let pieces: Vec<_> = (0..4)
                .filter(|&i| support & (1 << i) != 0)
                .map(|i| piece(space.table(3, &[1 << i], &()).unwrap(), mass[i], total))
                .collect();
            let source = measured(&space, &pieces);
            let mut w = work();
            for e in 0u64..16 {
                let event = source.table(3, &[e], &()).unwrap();
                for g in 0u64..16 {
                    let evidence = source.table(3, &[g], &()).unwrap();
                    let numerator: u64 = (0..4)
                        .filter(|&i| e & g & (1 << i) != 0)
                        .map(|i| mass[i])
                        .sum();
                    let denominator: u64 =
                        (0..4).filter(|&i| g & (1 << i) != 0).map(|i| mass[i]).sum();
                    let observation = event.probability(&evidence, &mut w).unwrap();
                    assert_eq!(observation.numerator(), &ratio(numerator, total));
                    assert_eq!(observation.evidence_mass(), &ratio(denominator, total));
                    assert_eq!(
                        observation.value(&mut w).unwrap(),
                        (denominator != 0).then(|| ratio(numerator, denominator))
                    );
                    let complement = event.complement().probability(&evidence, &mut w).unwrap();
                    assert_eq!(
                        observation
                            .numerator()
                            .add(complement.numerator(), &mut w)
                            .unwrap(),
                        *observation.evidence_mass()
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 61_440);
}

#[test]
fn skipped_sixty_two_bit_outcomes_contract_symbolically() {
    let raw = Space::new(SpaceId([12; 32]), 62, &()).unwrap();
    let source = measured(&raw, &[piece(raw.full(), 1, 1 << 62)]);
    let x = source.coordinate(61, &()).unwrap();
    assert_eq!(
        source.full().mass(&mut work()).unwrap(),
        ExactRational::one()
    );
    assert_eq!(x.mass(&mut work()).unwrap(), ratio(1, 2));
    assert_eq!(source.density_pieces().unwrap().len(), 1);
    let bytes = x.to_bytes(&()).unwrap();
    assert!(bytes.len() < 300);
    let reverse: Vec<_> = (0..62).rev().collect();
    let decoded =
        Event::from_bytes_with_order(&bytes, Some(&reverse), Limits::default(), &()).unwrap();
    assert_eq!(decoded.mass(&mut work()).unwrap(), ratio(1, 2));
    assert_eq!(decoded.to_bytes(&()).unwrap(), bytes);
}

#[test]
fn zero_mass_retains_possibilities_missing_law_and_impossible_are_distinct() {
    let base = raw(&[0]);
    let x = base.coordinate(0, &()).unwrap();
    assert_eq!(x.mass(&mut work()), Err(Error::MissingLaw));
    let source = measured(&base, &[piece(x.clone(), 1, 1)]);
    let x = x.in_space(&source, &()).unwrap();
    let zero = x.complement();
    assert!(!zero.is_empty());
    assert_eq!(zero.count(&()).unwrap(), 1);
    let observation = x.probability(&zero, &mut work()).unwrap();
    assert!(observation.is_impossible());
    assert!(observation.space().is_measured());
    assert_eq!(observation.value(&mut work()).unwrap(), None);
    assert_eq!(
        x.probability(&x, &mut work())
            .unwrap()
            .value(&mut work())
            .unwrap(),
        Some(ExactRational::one())
    );
    assert_eq!(
        x.probability(&base.empty(), &mut work()).err(),
        Some(Error::SpaceMismatch)
    );
    let restricted = source.restrict(&zero, &()).unwrap();
    assert!(!restricted.is_measured());
    assert_eq!(
        zero.in_space(&restricted, &()).unwrap().mass(&mut work()),
        Err(Error::MissingLaw)
    );
}

#[test]
fn canonical_law_erases_partitioning_but_preserves_correlation_and_revision() {
    let base = raw(&[0, 1]);
    let even = base.table(3, &[0b1001], &()).unwrap();
    let first = measured(&base, &[piece(even.clone(), 1, 2)]);
    let other = raw(&[1, 0]);
    let split = measured(
        &other,
        &[
            piece(other.table(3, &[8], &()).unwrap(), 1, 2),
            piece(other.table(3, &[6], &()).unwrap(), 0, 1),
            piece(other.table(3, &[1], &()).unwrap(), 1, 2),
        ],
    );
    assert_eq!(
        first.full().to_bytes(&()).unwrap(),
        split.full().to_bytes(&()).unwrap()
    );
    assert!(split.full().align_to(&first, &()).unwrap().is_full());
    let uniform = measured(&base, &[piece(base.full(), 1, 4)]);
    let x = first.coordinate(0, &()).unwrap();
    let y = first.coordinate(1, &()).unwrap();
    assert_eq!(x.mass(&mut work()).unwrap(), ratio(1, 2));
    assert_eq!(y.mass(&mut work()).unwrap(), ratio(1, 2));
    assert_eq!(
        x.apply(BoolOp4::AND, &y, &())
            .unwrap()
            .mass(&mut work())
            .unwrap(),
        ratio(1, 2)
    );
    assert_eq!(
        uniform
            .coordinate(0, &())
            .unwrap()
            .apply(BoolOp4::AND, &uniform.coordinate(1, &()).unwrap(), &())
            .unwrap()
            .mass(&mut work())
            .unwrap(),
        ratio(1, 4)
    );
    assert_eq!(
        first.empty().align_to(&uniform, &()),
        Err(Error::SpaceMismatch)
    );
    assert_eq!(first.full().align_to(&base, &()), Err(Error::SpaceMismatch));
    assert_eq!(
        x.diagram(&()).unwrap().rebuild(&uniform, &()),
        Err(Error::SpaceMismatch)
    );
    let graph = x.diagram(&()).unwrap();
    assert_eq!(
        graph.rebuild(&split, &()).unwrap().to_bytes(&()).unwrap(),
        x.to_bytes(&()).unwrap()
    );
    let mut registry = Registry::default();
    let registered = registry.intern(&first.empty(), &()).unwrap();
    let equivalent = registry.intern(&split.empty(), &()).unwrap();
    let revised = registry.intern(&uniform.empty(), &()).unwrap();
    assert_eq!(registered, equivalent);
    assert_ne!(registered, revised);
}

#[test]
fn admission_refuses_bad_laws_and_even_inert_foreign_pieces() {
    let base = raw(&[0]);
    let x = base.coordinate(0, &()).unwrap();
    let foreign = raw(&[0]).empty();
    for (pieces, error) in [
        (vec![], Error::LawNotNormalized),
        (vec![piece(base.full(), 1, 1)], Error::LawNotNormalized),
        (
            vec![piece(base.full(), 1, 2), piece(x.clone(), 0, 1)],
            Error::LawOverlap,
        ),
        (
            vec![piece(base.full(), 1, 2), piece(foreign, 0, 1)],
            Error::SpaceMismatch,
        ),
        (
            vec![
                DensityPiece {
                    region: base.empty(),
                    density: ExactRational::from(-1i64),
                },
                piece(base.full(), 1, 2),
            ],
            Error::NegativeMass,
        ),
    ] {
        assert_eq!(
            base.with_density(&pieces, LawLimits::default(), &mut work())
                .err(),
            Some(error)
        );
    }
    let inputs = [piece(base.full(), 1, 2)];
    assert_eq!(
        base.with_density(
            &inputs,
            LawLimits {
                cells: 0,
                bytes: 1024
            },
            &mut work()
        )
        .err(),
        Some(Error::Capacity(Capacity::LawCells))
    );
    assert_eq!(
        base.with_density(&inputs, LawLimits { cells: 1, bytes: 1 }, &mut work())
            .err(),
        Some(Error::Capacity(Capacity::DescriptorBytes))
    );
}

#[test]
fn measured_wire_rechecks_canonicality_context_normalization_and_limits() {
    let base = raw(&[0]);
    let source = measured(&base, &[piece(base.full(), 1, 2)]);
    let bytes = source.coordinate(0, &()).unwrap().to_bytes(&()).unwrap();
    assert_eq!(&bytes[..5], b"BEVT\x02");
    for end in 0..bytes.len() {
        assert!(Event::from_bytes(&bytes[..end], &()).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Event::from_bytes(&trailing, &()).is_err());
    let mut nested = bytes.clone();
    nested[17] = 2;
    assert!(Event::from_bytes(&nested, &()).is_err());
    let mut wrong = bytes.clone();
    let bera = wrong
        .windows(5)
        .position(|part| part == b"BERA\x01")
        .unwrap();
    wrong[bera + 23] = 3; // valid rational 1/3, invalid normalization
    assert_eq!(
        Event::from_bytes(&wrong, &()).err(),
        Some(Error::LawNotNormalized)
    );
    let mut wrong = bytes.clone();
    let second_event = wrong
        .windows(5)
        .enumerate()
        .filter(|(_, part)| *part == b"BEVT\x01")
        .nth(1)
        .unwrap()
        .0;
    wrong[second_event + 8] ^= 1;
    assert_eq!(
        Event::from_bytes(&wrong, &()).err(),
        Some(Error::SpaceMismatch)
    );
    assert_eq!(
        Event::from_bytes_with_source_limits(
            &bytes,
            None,
            Limits::default(),
            LawLimits {
                cells: 0,
                bytes: 1024
            },
            ArithmeticLimits::default(),
            &()
        )
        .err(),
        Some(Error::Capacity(Capacity::LawCells))
    );
    assert_eq!(
        Event::from_bytes_with_source_limits(
            &bytes,
            None,
            Limits::default(),
            LawLimits {
                cells: 10,
                bytes: 10
            },
            ArithmeticLimits::default(),
            &()
        )
        .err(),
        Some(Error::Capacity(Capacity::DescriptorBytes))
    );
    let retained = Event::from_bytes(&bytes, &()).unwrap();
    drop(source);
    drop(base);
    assert_eq!(retained.mass(&mut work()).unwrap(), ratio(1, 2));
    assert_eq!(retained.to_bytes(&()).unwrap(), bytes);
}

#[test]
fn finite_map_descriptors_retain_laws_but_products_do_not_infer_couplings() {
    use crate::{AdmittedDescriptor, CoordinateMap, Descriptor, DescriptorLimits, FaceProduct};
    let base = raw(&[0]);
    let source = measured(&base, &[piece(base.full(), 1, 2)]);
    let map = CoordinateMap::identity(&source, &()).unwrap();
    let limits = DescriptorLimits::default();
    let bytes = Descriptor::capture(&AdmittedDescriptor::Map(map), limits, &())
        .unwrap()
        .to_bytes(limits, &())
        .unwrap();
    let AdmittedDescriptor::Map(restored) = Descriptor::import(&bytes, limits, &()).unwrap() else {
        panic!("map")
    };
    let input = restored.source().coordinate(0, &()).unwrap();
    assert_eq!(
        restored
            .image(&input, &())
            .unwrap()
            .mass(&mut work())
            .unwrap(),
        ratio(1, 2)
    );
    let unit = Space::new(SpaceId([99; 32]), 0, &()).unwrap();
    let environment = CoordinateMap::coordinates(&source, &unit, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let product =
        FaceProduct::new(SpaceId([98; 32]), &[environment.clone(), environment], &()).unwrap();
    assert!(!product.space().is_measured());
    assert_eq!(
        product.space().full().mass(&mut work()),
        Err(Error::MissingLaw)
    );
}

#[test]
fn cancellation_and_arithmetic_exhaustion_publish_no_source_or_observation() {
    use std::cell::Cell;
    struct Stop(Cell<usize>);
    impl crate::Control for Stop {
        fn checkpoint(&self) -> crate::Result<()> {
            let left = self.0.get();
            if left == 0 {
                return Err(Error::Cancelled);
            }
            self.0.set(left - 1);
            Ok(())
        }
    }
    let base = raw(&[0]);
    let inputs = [piece(base.full(), 1, 2)];
    let source = measured(&base, &inputs);
    let bytes = source.full().to_bytes(&()).unwrap();
    for allowance in [0, 1, 5, 10] {
        let stop = Stop(Cell::new(allowance));
        let mut controlled = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert_eq!(
            base.with_density(&inputs, LawLimits::default(), &mut controlled)
                .err(),
            Some(Error::Cancelled)
        );
        let stop = Stop(Cell::new(allowance));
        assert_eq!(
            Event::from_bytes(&bytes, &stop).err(),
            Some(Error::Cancelled)
        );
    }
    let stop = Stop(Cell::new(0));
    assert_eq!(
        source.empty().mass(&mut ExactArithmetic::new(
            ArithmeticLimits::default(),
            &stop
        )),
        Err(Error::Cancelled)
    );
    let mut limited = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 100,
            operations: 0,
        },
        &(),
    );
    assert_eq!(
        source
            .full()
            .probability(&source.full(), &mut limited)
            .err(),
        Some(Error::Capacity(Capacity::ArithmeticSteps))
    );
    assert_eq!(base.full().mass(&mut work()), Err(Error::MissingLaw));
    assert_eq!(
        source.full().mass(&mut work()).unwrap(),
        ExactRational::one()
    );
}
