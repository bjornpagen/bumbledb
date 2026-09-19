use std::cell::Cell;

use crate::*;

fn math() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: i64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut math()).unwrap()
}
fn root(space: &Space, mask: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[mask], &())
        .unwrap()
}
fn function(space: &Space, values: [i64; 4], denominator: i64) -> FiniteFunction {
    let pieces: Vec<_> = values
        .into_iter()
        .enumerate()
        .filter(|(i, _)| space.full().contains(*i as u64).is_ok())
        .map(|(i, v)| FunctionPiece {
            region: root(space, 1 << i),
            value: q(v, denominator),
        })
        .collect();
    FiniteFunction::new(space, &pieces, FunctionLimits::default(), &mut math()).unwrap()
}
fn prior(masses: [i64; 4]) -> Space {
    let space = Space::new(SpaceId([161; 32]), 2, &()).unwrap();
    function(&space, masses, masses.iter().sum())
        .designate(LawLimits::default(), &mut math())
        .unwrap()
}
fn capture(value: &AdmittedSourceDescriptor) -> SourceDescriptor {
    SourceDescriptor::capture(value, SourceDescriptorLimits::default(), &mut math()).unwrap()
}
fn roundtrip(value: AdmittedSourceDescriptor) -> AdmittedSourceDescriptor {
    let descriptor = capture(&value);
    drop(value);
    let limits = SourceDescriptorLimits::default();
    let bytes = descriptor.to_bytes(limits, &()).unwrap();
    let parsed = SourceDescriptor::from_bytes(&bytes, limits, &()).unwrap();
    assert_eq!(descriptor, parsed);
    assert_eq!(bytes, parsed.to_bytes(limits, &()).unwrap());
    let imported = SourceDescriptor::import(&bytes, limits, &mut math()).unwrap();
    assert_eq!(descriptor, capture(&imported));
    imported
}
fn revision(value: SourceRevision) -> SourceRevision {
    let AdmittedSourceDescriptor::Revision(value) =
        roundtrip(AdmittedSourceDescriptor::Revision(value))
    else {
        panic!()
    };
    value
}
fn condition(space: &Space, evidence: &Event) -> SourceRevision {
    space
        .condition(
            evidence,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap()
}
fn likelihood(space: &Space, factor: &FiniteFunction) -> SourceRevision {
    space
        .likelihood(
            factor,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap()
}
fn partition(space: &Space, mask: u64) -> EventPartition {
    let a = root(space, mask);
    EventPartition::on(
        &space.full(),
        &[a.clone(), a.complement(), space.empty()],
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
            &mut math(),
        )
        .unwrap()
}

#[test]
fn scalar_transport_matches_255_signed_functions_on_all_four_world_supports() {
    let mut cases = 0;
    for support in 1u64..16 {
        let raw = Space::new(SpaceId([160; 32]), 2, &()).unwrap();
        let space = raw.restrict(&root(&raw, support), &()).unwrap();
        let independent = Event::from_bytes_with_order(
            &space.full().to_bytes(&()).unwrap(),
            Some(&[1, 0]),
            Limits::default(),
            &(),
        )
        .unwrap()
        .space();
        for code in 0..3u32.pow(support.count_ones()) {
            let mut digits = code;
            let values = std::array::from_fn(|i| {
                if support & (1 << i) == 0 {
                    return 0;
                }
                let v = [-2, 0, 3][(digits % 3) as usize];
                digits /= 3;
                v
            });
            let f = function(&space, values, 2);
            let reordered = function(&independent, values, 2);
            assert_eq!(
                capture(&AdmittedSourceDescriptor::Function(f.clone())),
                capture(&AdmittedSourceDescriptor::Function(reordered))
            );
            let AdmittedSourceDescriptor::Function(restored) =
                roundtrip(AdmittedSourceDescriptor::Function(f))
            else {
                panic!()
            };
            for (i, expected) in values.into_iter().enumerate() {
                if support & (1 << i) != 0 {
                    assert_eq!(restored.at(i as u64, &()).unwrap(), q(expected, 2));
                } else {
                    assert!(matches!(
                        restored.at(i as u64, &()),
                        Err(Error::IllegalWorld(_))
                    ));
                }
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 255);
}

#[test]
fn revision_transport_matches_independent_finite_reweighting_and_keeps_zero_worlds() {
    let mut cases = 0;
    for masses in [
        [1, 2, 3, 4],
        [1, 0, 1, 0],
        [0, 3, 0, 0],
        [0, 0, 0, 5],
        [1, 1, 1, 1],
    ] {
        let space = prior(masses);
        let total: i64 = masses.iter().sum();
        for mask in 0..16 {
            let update = revision(condition(&space, &root(&space, mask)));
            let z: i64 = (0..4)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| masses[i])
                .sum();
            let RevisionReceipt::Condition { mass, .. } = update.receipt() else {
                panic!()
            };
            assert_eq!(*mass, q(z, total));
            if z == 0 {
                assert!(matches!(
                    update.outcome(),
                    RevisionOutcome::Impossible(RevisionImpossible::ZeroEvidence)
                ));
            } else {
                let result = update.revised().unwrap();
                assert_eq!(result.space().full().count(&()).unwrap(), 4);
                for (i, m) in masses.into_iter().enumerate() {
                    let translated = result
                        .translation()
                        .pullback(&root(&space, 1 << i), &())
                        .unwrap();
                    assert_eq!(
                        translated.mass(&mut math()).unwrap(),
                        q(if mask & (1 << i) == 0 { 0 } else { m }, z)
                    );
                    assert!(!translated.is_empty());
                }
            }
            cases += 1;
        }
        for code in 0..81u32 {
            let weights =
                std::array::from_fn(|i| i64::from(code / 3u32.pow(u32::try_from(i).unwrap()) % 3));
            let update = revision(likelihood(&space, &function(&space, weights, 1)));
            let z: i64 = masses.iter().zip(weights).map(|(m, w)| m * w).sum();
            let RevisionReceipt::Likelihood { normalizer, .. } = update.receipt() else {
                panic!()
            };
            assert_eq!(*normalizer, q(z, total));
            if z == 0 {
                assert!(matches!(
                    update.outcome(),
                    RevisionOutcome::Impossible(RevisionImpossible::ZeroLikelihood)
                ));
            } else {
                let result = update.revised().unwrap().space();
                for (i, m) in masses.into_iter().enumerate() {
                    assert_eq!(
                        root(result, 1 << i).mass(&mut math()).unwrap(),
                        q(m * weights[i], z)
                    );
                }
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 485);
}

#[test]
fn indexed_jeffrey_receipts_keep_empty_cells_and_all_unsupported_targets() {
    let mut cases = 0;
    for masses in [[1, 2, 3, 4], [1, 0, 1, 0], [0, 3, 0, 0]] {
        let space = prior(masses);
        let total: i64 = masses.iter().sum();
        for mask in 0..16 {
            for target in 0..=4 {
                let update = revision(jeffrey(
                    &space,
                    &partition(&space, mask),
                    &[q(target, 4), q(4 - target, 4), q(0, 1)],
                ));
                let old: i64 = (0..4)
                    .filter(|i| mask & (1 << i) != 0)
                    .map(|i| masses[i])
                    .sum();
                let unsupported: Vec<_> = [(0, old, target), (1, total - old, 4 - target)]
                    .into_iter()
                    .filter_map(|(i, m, t)| (m == 0 && t > 0).then_some(i))
                    .collect();
                let RevisionReceipt::Jeffrey {
                    partition,
                    old_masses,
                    ..
                } = update.receipt()
                else {
                    panic!()
                };
                assert_eq!(partition.cells().len(), 3);
                assert!(partition.cells()[2].is_empty());
                assert_eq!(
                    old_masses.as_ref(),
                    [q(old, total), q(total - old, total), q(0, 1)]
                );
                if unsupported.is_empty() {
                    let result = update.revised().unwrap().space();
                    for (i, m) in masses.into_iter().enumerate() {
                        let (target, old) = if mask & (1 << i) == 0 {
                            (4 - target, total - old)
                        } else {
                            (target, old)
                        };
                        let expected = if target == 0 {
                            q(0, 1)
                        } else {
                            q(m * target, 4 * old)
                        };
                        assert_eq!(root(result, 1 << i).mass(&mut math()).unwrap(), expected);
                    }
                } else {
                    assert!(
                        matches!(update.outcome(), RevisionOutcome::Impossible(RevisionImpossible::UnsupportedTargets { cells }) if cells.as_ref() == unsupported)
                    );
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 240);
    let space = prior([1, 0, 0, 0]);
    let cells: Vec<_> = (0..4)
        .map(|i| root(&space, 1 << i))
        .chain([space.empty()])
        .collect();
    let p = EventPartition::on(&space.full(), &cells, PartitionLimits::default(), &()).unwrap();
    let update = revision(jeffrey(
        &space,
        &p,
        &[q(0, 1), q(1, 3), q(0, 1), q(1, 3), q(1, 3)],
    ));
    assert!(
        matches!(update.outcome(), RevisionOutcome::Impossible(RevisionImpossible::UnsupportedTargets { cells }) if cells.as_ref() == [1,3,4])
    );
}

fn kernel() -> FiniteKernel {
    let raw = Space::new(SpaceId([162; 32]), 1, &()).unwrap();
    let parent = raw
        .with_density(
            &[DensityPiece {
                region: raw.coordinate(0, &()).unwrap().complement(),
                density: q(1, 1),
            }],
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    let extended = Space::new(SpaceId([163; 32]), 2, &()).unwrap();
    let map = CoordinateMap::coordinates(&extended, &parent, &[0], &()).unwrap();
    FiniteKernel::new(
        &map,
        &function(&extended, [1, 2, 3, 2], 4),
        FunctionLimits::default(),
        &mut math(),
    )
    .unwrap()
}

#[test]
fn imported_channel_preserves_shared_parent_and_checks_zero_prior_rows() {
    let AdmittedSourceDescriptor::Kernel(k) = roundtrip(AdmittedSourceDescriptor::Kernel(kernel()))
    else {
        panic!()
    };
    let prior = k.parent().map().target();
    let extended = k
        .close(
            prior,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    for (i, n) in [1, 0, 3, 0].into_iter().enumerate() {
        assert_eq!(
            root(extended.space(), 1 << i).mass(&mut math()).unwrap(),
            q(n, 4)
        );
    }
    assert_eq!(extended.space().full().count(&()).unwrap(), 4);
    // Alter only the row that has zero prior mass. A joint-normalization-only
    // import would accept this channel, but every-fibre admission must refuse.
    let SourceDescriptor::Kernel(mut forged) =
        capture(&AdmittedSourceDescriptor::Kernel(k.clone()))
    else {
        panic!()
    };
    let SourceDescriptor::Function(bad) = capture(&AdmittedSourceDescriptor::Function(function(
        k.density().space(),
        [1, 0, 3, 0],
        4,
    ))) else {
        panic!()
    };
    forged.density = bad;
    assert!(matches!(
        SourceDescriptor::Kernel(forged).admit(SourceDescriptorLimits::default(), &mut math()),
        Err(Error::KernelNotNormalized)
    ));
}

fn reject_claim(descriptor: RevisionDescriptor) {
    // Syntax may faithfully carry a false claim. Import must not endorse it.
    let limits = SourceDescriptorLimits::default();
    let bytes = SourceDescriptor::Revision(descriptor)
        .to_bytes(limits, &())
        .unwrap();
    assert!(matches!(
        SourceDescriptor::import(&bytes, limits, &mut math()),
        Err(Error::DescriptorClaimMismatch)
    ));
}

#[test]
fn transport_rejects_forged_normalizers_posteriors_and_impossibility_receipts() {
    let space = prior([1, 2, 3, 4]);
    let SourceDescriptor::Revision(original) = capture(&AdmittedSourceDescriptor::Revision(
        condition(&space, &root(&space, 0b0101)),
    )) else {
        panic!()
    };
    let mut wrong_mass = original.clone();
    let RevisionReceiptDescriptor::Condition { mass, .. } = &mut wrong_mass.receipt else {
        panic!()
    };
    *mass = q(1, 1).to_bytes(&mut math()).unwrap();
    reject_claim(wrong_mass);
    let mut wrong_posterior = original.clone();
    // Normalized, same name and support, but the wrong posterior.
    wrong_posterior.outcome = RevisionOutcomeDescriptor::Revised {
        posterior: space.full().to_bytes(&()).unwrap(),
    };
    reject_claim(wrong_posterior);
    let mut wrong_outcome = original;
    wrong_outcome.outcome = RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroEvidence);
    reject_claim(wrong_outcome);
    let SourceDescriptor::Revision(mut scaled) = capture(&AdmittedSourceDescriptor::Revision(
        likelihood(&space, &function(&space, [0, 2, 0, 4], 1)),
    )) else {
        panic!()
    };
    let RevisionReceiptDescriptor::Likelihood { likelihood, .. } = &mut scaled.receipt else {
        panic!()
    };
    for piece in &mut likelihood.pieces {
        let value = ExactRational::from_bytes(&piece.value, &mut math()).unwrap();
        piece.value = value
            .mul(&q(2, 1), &mut math())
            .unwrap()
            .to_bytes(&mut math())
            .unwrap();
    }
    // Scale preserves the posterior; it must still change the receipt normalizer.
    reject_claim(scaled);
    let SourceDescriptor::Revision(mut wrong_masses) =
        capture(&AdmittedSourceDescriptor::Revision(jeffrey(
            &space,
            &partition(&space, 5),
            &[q(1, 2), q(1, 2), q(0, 1)],
        )))
    else {
        panic!()
    };
    let RevisionReceiptDescriptor::Jeffrey { old_masses, .. } = &mut wrong_masses.receipt else {
        panic!()
    };
    old_masses.swap(0, 1);
    reject_claim(wrong_masses);
    let concentrated = prior([1, 0, 0, 0]);
    let SourceDescriptor::Revision(mut unsupported) =
        capture(&AdmittedSourceDescriptor::Revision(jeffrey(
            &concentrated,
            &partition(&concentrated, 2),
            &[q(1, 2), q(0, 1), q(1, 2)],
        )))
    else {
        panic!()
    };
    // Both a legal zero-mass cell and an empty indexed cell are unsupported.
    for indices in [vec![0], vec![2], vec![2, 0], vec![0, 0, 2], vec![0, 2, 3]] {
        unsupported.outcome =
            RevisionOutcomeDescriptor::Impossible(RevisionImpossible::UnsupportedTargets {
                cells: indices.into(),
            });
        reject_claim(unsupported.clone());
    }
    unsupported.outcome = RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroLikelihood);
    reject_claim(unsupported);
}

#[test]
fn transport_admits_redundant_function_presentations_but_never_hides_invalid_cells() {
    let space = prior([1, 2, 3, 4]);
    let limits = SourceDescriptorLimits::default();
    let descriptor = FunctionDescriptor {
        space: space.full().to_bytes(&()).unwrap(),
        pieces: (0..4)
            .map(|i| FunctionPieceDescriptor {
                region: root(&space, 1 << i).to_bytes(&()).unwrap(),
                value: q(if i == 0 { 0 } else { 2 }, 1)
                    .to_bytes(&mut math())
                    .unwrap(),
            })
            .collect(),
    };
    let admitted = SourceDescriptor::Function(descriptor.clone())
        .admit(limits, &mut math())
        .unwrap();
    let AdmittedSourceDescriptor::Function(f) = admitted else {
        panic!()
    };
    assert_eq!(f.pieces().len(), 1);
    assert_eq!(f.at(0, &()).unwrap(), q(0, 1));
    let mut bad = descriptor.clone();
    bad.pieces[0].region = space.full().to_bytes(&()).unwrap();
    assert!(matches!(
        SourceDescriptor::Function(bad).admit(limits, &mut math()),
        Err(Error::FunctionOverlap)
    ));
    let mut bad = descriptor.clone();
    bad.pieces[0].region = Space::new(SpaceId([164; 32]), 2, &())
        .unwrap()
        .empty()
        .to_bytes(&())
        .unwrap();
    assert!(matches!(
        SourceDescriptor::Function(bad).admit(limits, &mut math()),
        Err(Error::SpaceMismatch)
    ));
    let mut bad = descriptor;
    bad.space = root(&space, 1).to_bytes(&()).unwrap();
    assert!(matches!(
        SourceDescriptor::Function(bad).admit(limits, &mut math()),
        Err(Error::InvalidEncoding)
    ));
}

struct StopAfter(Cell<usize>);
impl Control for StopAfter {
    fn checkpoint(&self) -> Result<()> {
        let remaining = self.0.get().checked_sub(1).ok_or(Error::Cancelled)?;
        self.0.set(remaining);
        Ok(())
    }
}

#[test]
fn source_descriptor_extent_arithmetic_law_and_cancellation_limits_are_enforced() {
    let original = AdmittedSourceDescriptor::Revision(likelihood(
        &prior([1, 2, 3, 4]),
        &function(&prior([1, 2, 3, 4]), [1, 2, 3, 4], 1),
    ));
    let descriptor = capture(&original);
    let limits = SourceDescriptorLimits::default();
    let bytes = descriptor.to_bytes(limits, &()).unwrap();
    let mut small = limits;
    small.descriptors.bytes = bytes.len() - 1;
    assert!(matches!(
        descriptor.to_bytes(small, &()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    assert!(matches!(
        SourceDescriptor::from_bytes(&bytes, small, &()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    let mut small = limits;
    small.functions.cells = 0;
    assert!(matches!(
        descriptor.admit(small, &mut math()),
        Err(Error::Capacity(Capacity::FunctionCells))
    ));
    assert!(matches!(
        SourceDescriptor::from_bytes(&bytes, small, &()),
        Err(Error::Capacity(Capacity::FunctionCells))
    ));
    let mut small = limits;
    small.laws.cells = 0;
    assert!(matches!(
        descriptor.admit(small, &mut math()),
        Err(Error::Capacity(Capacity::LawCells))
    ));
    // Find the exact descriptor item boundary; all three paths use one roster.
    let boundary = (0..100)
        .find(|&items| {
            let mut small = limits;
            small.descriptors.items = items;
            descriptor.to_bytes(small, &()).is_ok()
        })
        .unwrap();
    for items in [boundary - 1, boundary] {
        let mut small = limits;
        small.descriptors.items = items;
        assert_eq!(
            SourceDescriptor::capture(&original, small, &mut math()).is_ok(),
            items == boundary
        );
        assert_eq!(
            SourceDescriptor::from_bytes(&bytes, small, &()).is_ok(),
            items == boundary
        );
        assert_eq!(
            descriptor.admit(small, &mut math()).is_ok(),
            items == boundary
        );
    }
    let mut work = math();
    descriptor.admit(limits, &mut work).unwrap();
    let needed = work.operations();
    assert!(needed > 100);
    let mut work = ExactArithmetic::new(
        ArithmeticLimits {
            operations: needed - 1,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(matches!(
        descriptor.admit(limits, &mut work),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let mut work = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 1,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(matches!(
        descriptor.admit(limits, &mut work),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    ));
    for remaining in [0, 1, 10, 40] {
        assert!(matches!(
            SourceDescriptor::from_bytes(&bytes, limits, &StopAfter(Cell::new(remaining))),
            Err(Error::Cancelled)
        ));
        assert!(matches!(
            descriptor.to_bytes(limits, &StopAfter(Cell::new(remaining))),
            Err(Error::Cancelled)
        ));
        let stop = StopAfter(Cell::new(remaining));
        assert!(matches!(
            descriptor.admit(
                limits,
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &stop)
            ),
            Err(Error::Cancelled)
        ));
    }
}

#[test]
fn malformed_source_envelopes_never_become_executable_claims() {
    let space = prior([1, 2, 3, 4]);
    let fixtures = [
        capture(&AdmittedSourceDescriptor::Function(function(
            &space,
            [-3, 0, 1, 7],
            2,
        ))),
        capture(&AdmittedSourceDescriptor::Kernel(kernel())),
        capture(&AdmittedSourceDescriptor::Revision(condition(
            &space,
            &root(&space, 5),
        ))),
        capture(&AdmittedSourceDescriptor::Revision(likelihood(
            &space,
            &function(&space, [0, 1, 2, 3], 1),
        ))),
        capture(&AdmittedSourceDescriptor::Revision(jeffrey(
            &space,
            &partition(&space, 5),
            &[q(1, 2), q(1, 2), q(0, 1)],
        ))),
        capture(&AdmittedSourceDescriptor::Revision(jeffrey(
            &space,
            &partition(&space, 5),
            &[q(1, 2), q(0, 1), q(1, 2)],
        ))),
    ];
    let limits = SourceDescriptorLimits::default();
    for descriptor in fixtures {
        let bytes = descriptor.to_bytes(limits, &()).unwrap();
        for end in 0..bytes.len() {
            assert!(
                SourceDescriptor::from_bytes(&bytes[..end], limits, &()).is_err(),
                "prefix {end}"
            );
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(SourceDescriptor::from_bytes(&bad, limits, &()).is_err());
        let mut bad = bytes.clone();
        bad[4] = 255;
        assert!(matches!(
            SourceDescriptor::from_bytes(&bad, limits, &()),
            Err(Error::UnsupportedVersion(255))
        ));
        let mut bad = bytes.clone();
        bad[5] = 255;
        assert!(SourceDescriptor::from_bytes(&bad, limits, &()).is_err());
        let mut bad = bytes;
        bad[6..14].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(SourceDescriptor::from_bytes(&bad, limits, &()).is_err());
    }
    let SourceDescriptor::Function(mut f) = capture(&AdmittedSourceDescriptor::Function(function(
        &space,
        [0, 1, 2, 3],
        1,
    ))) else {
        panic!()
    };
    f.pieces[0].value.push(0);
    let bad = SourceDescriptor::Function(f).to_bytes(limits, &()).unwrap();
    assert!(SourceDescriptor::from_bytes(&bad, limits, &()).is_ok());
    assert!(matches!(
        SourceDescriptor::import(&bad, limits, &mut math()),
        Err(Error::InvalidRational)
    ));
}

#[test]
fn source_transport_stays_symbolic_on_sixty_two_coordinates() {
    let space = Space::new(SpaceId([165; 32]), 62, &()).unwrap();
    let a = space.coordinate(61, &()).unwrap();
    let measured = space
        .with_density(
            &[DensityPiece {
                region: a.clone(),
                density: q(1, 1i64 << 61),
            }],
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    let a = a.in_space(&measured, &()).unwrap();
    let update = revision(condition(&measured, &a.complement()));
    assert_eq!(update.prior().full().count(&()).unwrap(), 1u64 << 62);
    assert!(matches!(
        update.outcome(),
        RevisionOutcome::Impossible(RevisionImpossible::ZeroEvidence)
    ));
    let update = revision(likelihood(
        &measured,
        &FiniteFunction::constant(&measured, q(2, 1), FunctionLimits::default(), &mut math())
            .unwrap(),
    ));
    assert_eq!(
        update
            .revised()
            .unwrap()
            .space()
            .full()
            .to_bytes(&())
            .unwrap(),
        measured.full().to_bytes(&()).unwrap()
    );
}

#[test]
fn pinned_v1_scalar_source_fixture() {
    let space = Space::new(SpaceId([166; 32]), 0, &()).unwrap();
    let f =
        FiniteFunction::constant(&space, q(-1, 2), FunctionLimits::default(), &mut math()).unwrap();
    let descriptor = capture(&AdmittedSourceDescriptor::Function(f));
    let bytes = descriptor
        .to_bytes(SourceDescriptorLimits::default(), &())
        .unwrap();
    let hex = include_str!("../tests/fixtures/source-v1-function.hex").trim();
    let fixture: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(bytes, fixture);
    let AdmittedSourceDescriptor::Function(f) =
        SourceDescriptor::import(&fixture, SourceDescriptorLimits::default(), &mut math()).unwrap()
    else {
        panic!()
    };
    assert_eq!(f.at(0, &()).unwrap(), q(-1, 2));
}
