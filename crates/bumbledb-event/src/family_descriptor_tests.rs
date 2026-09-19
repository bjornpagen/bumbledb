#![allow(clippy::too_many_lines, clippy::many_single_char_names)]
use crate::parameter_source_tests::{
    and, c, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn dl() -> SourceDescriptorLimits {
    SourceDescriptorLimits::default()
}
fn coefficient(s: &Space, n: ExactPolynomial, d: ExactPolynomial) -> GuardedRationalFunction {
    GuardedRationalFunction::new(
        s.parameter_domain().unwrap().clone(),
        n,
        d,
        limits().parameters.region,
        &mut work(),
    )
    .unwrap()
}
fn parameter(s: &Space, n: ExactPolynomial, d: ExactPolynomial) -> ParameterFunction {
    ParameterFunction::new(
        s.parameter_domain().unwrap().clone(),
        &[coefficient(s, n, d)],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}
fn capture(v: AdmittedFamilyDescriptor) -> SourceDescriptor {
    SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Family(Box::new(v)),
        dl(),
        &mut work(),
    )
    .unwrap()
}
fn family(d: &mut SourceDescriptor) -> &mut FamilyDescriptor {
    let SourceDescriptor::Family(v) = d else {
        panic!("family descriptor");
    };
    v
}
fn restore(d: &SourceDescriptor) -> AdmittedFamilyDescriptor {
    let bytes = d.to_bytes(dl(), &()).unwrap();
    assert_eq!(&bytes[..5], b"BESC\x02");
    assert_eq!(SourceDescriptor::from_bytes(&bytes, dl(), &()).unwrap(), *d);
    let v = SourceDescriptor::import(&bytes, dl(), &mut work()).unwrap();
    let normalized = SourceDescriptor::capture(&v, dl(), &mut work()).unwrap();
    assert_eq!(normalized.to_bytes(dl(), &()).unwrap(), bytes);
    let AdmittedSourceDescriptor::Family(v) = v else {
        panic!("admitted family");
    };
    *v
}
fn at(f: &ParameterFunction, n: &str, d: &str) -> Option<ExactRational> {
    f.value_at(
        &ratio(n, d),
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}
fn partition(s: &Space) -> EventPartition {
    let a = s.coordinate(0, &()).unwrap();
    EventPartition::on(
        &s.full(),
        &[a.clone(), a.complement(), s.empty()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap()
}
fn jeffrey(s: &Space, impossible: bool) -> ParameterJeffrey {
    let targets = if impossible {
        [
            parameter(s, c(0), c(1)),
            parameter(s, c(0), c(1)),
            parameter(s, c(1), c(1)),
        ]
    } else {
        [
            parameter(s, c(1), c(2)),
            parameter(s, c(1), c(2)),
            parameter(s, c(0), c(1)),
        ]
    };
    s.parameter_jeffrey(
        SpaceId([230; 32]),
        &partition(s),
        &targets,
        limits(),
        &mut work(),
    )
    .unwrap()
}

#[test]
fn family_transport_preserves_partial_holes_signed_values_and_declared_zero_cells() {
    let s = shared_bias();
    let partial = parameter(&s, p(), p());
    let d = capture(AdmittedFamilyDescriptor::Parameter(partial));
    let AdmittedFamilyDescriptor::Parameter(f) = restore(&d) else {
        panic!("parameter");
    };
    assert_eq!(at(&f, "0", "1"), None);
    assert_eq!(at(&f, "1", "2"), Some(1u64.into()));
    let a = s.coordinate(0, &()).unwrap();
    let f = FamilyFunction::new(
        &s,
        &[
            FamilyFunctionPiece {
                region: a.clone(),
                value: coefficient(&s, ExactPolynomial::constant((-2i64).into()), c(1)),
            },
            FamilyFunctionPiece {
                region: a.complement(),
                value: coefficient(&s, c(0), c(1)),
            },
        ],
        limits(),
        &mut work(),
    )
    .unwrap();
    let d = capture(AdmittedFamilyDescriptor::Function(f));
    let density = capture(AdmittedFamilyDescriptor::Function(
        FamilyFunction::density(&s, limits(), &mut work()).unwrap(),
    ));
    let original = s.full().to_bytes(&()).unwrap();
    drop((s, a));
    let AdmittedFamilyDescriptor::Function(f) = restore(&d) else {
        panic!("function");
    };
    assert_eq!(f.pieces().len(), 2);
    assert_eq!(
        f.value_at(&ratio("1", "3"), 1, limits(), &mut work())
            .unwrap(),
        Some((-2i64).into())
    );
    assert_eq!(
        f.value_at(&ratio("1", "3"), 0, limits(), &mut work())
            .unwrap(),
        Some(0u64.into())
    );
    let AdmittedFamilyDescriptor::Function(f) = restore(&density) else {
        panic!("density");
    };
    assert_eq!(
        f.designate(limits(), &mut work())
            .unwrap()
            .full()
            .to_bytes(&())
            .unwrap(),
        original
    );
}

#[test]
fn family_transport_replays_refinements_and_restrictions_without_forgetting_original_events() {
    let s = shared_bias();
    let a = s.coordinate(0, &()).unwrap();
    let predicate = sign(
        &sub(&p(), &ExactPolynomial::constant(ratio("1", "2"))),
        PolynomialSigns::POSITIVE,
    );
    let refinement = ParameterRefinement::new(
        SpaceId([231; 32]),
        &s,
        std::slice::from_ref(&predicate),
        limits(),
        &mut work(),
    )
    .unwrap();
    let refined = capture(AdmittedFamilyDescriptor::Refinement(refinement.clone()));
    let restriction =
        ParameterRestriction::from_refinement(&refinement, &predicate, limits(), &mut work())
            .unwrap();
    let restricted = capture(AdmittedFamilyDescriptor::Restriction(restriction));
    assert!(matches!(
        ParameterRestriction::from_refinement(
            &refinement,
            &ParameterRegion::empty(s.parameter_domain().unwrap().parameter()),
            limits(),
            &mut work()
        ),
        Err(Error::EmptyParameterDomain)
    ));
    let AdmittedFamilyDescriptor::Refinement(r) = restore(&refined) else {
        panic!("refinement");
    };
    assert_eq!(
        r.descend(&r.lift(&a, &()).unwrap(), &())
            .unwrap()
            .to_bytes(&())
            .unwrap(),
        a.to_bytes(&()).unwrap()
    );
    let AdmittedFamilyDescriptor::Restriction(r) = restore(&restricted) else {
        panic!("restriction");
    };
    let narrow = r.pullback(&a, &()).unwrap();
    assert_eq!(
        at(
            &narrow.parameter_mass(limits(), &mut work()).unwrap(),
            "1",
            "4"
        ),
        None
    );
    assert_eq!(
        at(
            &narrow.parameter_mass(limits(), &mut work()).unwrap(),
            "3",
            "4"
        ),
        Some(ratio("3", "4"))
    );
    let mut bad = refined.clone();
    let FamilyDescriptor::Refinement(r) = family(&mut bad) else {
        panic!("refinement");
    };
    r.predicates[0] = predicate
        .complement()
        .to_bytes(limits().parameters, &mut work())
        .unwrap();
    assert!(matches!(
        bad.admit(dl(), &mut work()),
        Err(Error::DescriptorClaimMismatch)
    ));
    let mut bad = restricted.clone();
    let FamilyDescriptor::Restriction(r) = family(&mut bad) else {
        panic!("restriction");
    };
    r.predicate = s
        .parameter_domain()
        .unwrap()
        .to_bytes(limits().parameters, &mut work())
        .unwrap();
    assert!(matches!(
        bad.admit(dl(), &mut work()),
        Err(Error::DescriptorClaimMismatch)
    ));
}

#[test]
fn family_transport_rechecks_channels_on_zero_prior_worlds_and_closes_after_owner_release() {
    let s = shared_bias();
    let extension = space(232, 5, s.parameter_guards().unwrap());
    let next = extension.coordinate(4, &()).unwrap();
    let density = FamilyFunction::new(
        &extension,
        &[
            FamilyFunctionPiece {
                region: next.clone(),
                value: coefficient(&extension, p(), c(1)),
            },
            FamilyFunctionPiece {
                region: next.complement(),
                value: coefficient(&extension, sub(&c(1), &p()), c(1)),
            },
        ],
        limits(),
        &mut work(),
    )
    .unwrap();
    let map = CoordinateMap::coordinates(&extension, &s, &[0, 1, 2, 3], &()).unwrap();
    let kernel = FamilyKernel::new(&map, &density, limits(), &mut work()).unwrap();
    let data = capture(AdmittedFamilyDescriptor::Kernel(kernel));
    let bad_cell = and(
        &extension.coordinate(0, &()).unwrap(),
        &extension.coordinate(2, &()).unwrap(),
    );
    let bad = density
        .multiply(
            &FamilyFunction::new(
                &extension,
                &[FamilyFunctionPiece {
                    region: bad_cell.complement(),
                    value: coefficient(&extension, c(1), c(1)),
                }],
                limits(),
                &mut work(),
            )
            .unwrap(),
            limits(),
            &mut work(),
        )
        .unwrap();
    let mut bad_data = data.clone();
    let SourceDescriptor::Family(f) = capture(AdmittedFamilyDescriptor::Function(bad)) else {
        panic!("family");
    };
    let FamilyDescriptor::Function(bad) = *f else {
        panic!("function");
    };
    let FamilyDescriptor::Kernel(k) = family(&mut bad_data) else {
        panic!("kernel");
    };
    k.density = bad;
    assert!(matches!(
        bad_data.admit(dl(), &mut work()),
        Err(Error::KernelNotNormalized)
    ));
    drop((s, extension, next, density, map));
    let AdmittedFamilyDescriptor::Kernel(kernel) = restore(&data) else {
        panic!("kernel");
    };
    let prior = kernel.parent().map().target();
    let closed = kernel.close(prior, limits(), &mut work()).unwrap();
    let old = prior.coordinate(0, &()).unwrap();
    let lifted = closed.parent().pullback(&old, &()).unwrap();
    assert!(
        lifted
            .parameter_mass(limits(), &mut work())
            .unwrap()
            .equivalent(
                &old.parameter_mass(limits(), &mut work()).unwrap(),
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
}

#[test]
fn family_transport_owns_successful_and_impossible_receipts_with_requested_identities() {
    let s = shared_bias();
    let a = s.coordinate(0, &()).unwrap();
    let f = FamilyFunction::new(
        &s,
        &[FamilyFunctionPiece {
            region: a.clone(),
            value: coefficient(&s, c(3), c(1)),
        }],
        limits(),
        &mut work(),
    )
    .unwrap();
    let zero = FamilyFunction::new(&s, &[], limits(), &mut work()).unwrap();
    let cases = [
        capture(AdmittedFamilyDescriptor::Conditioning(
            s.parameter_condition(SpaceId([233; 32]), &a, limits(), &mut work())
                .unwrap(),
        )),
        capture(AdmittedFamilyDescriptor::Conditioning(
            s.parameter_condition(SpaceId([234; 32]), &s.empty(), limits(), &mut work())
                .unwrap(),
        )),
        capture(AdmittedFamilyDescriptor::Likelihood(
            s.parameter_likelihood(SpaceId([235; 32]), &f, limits(), &mut work())
                .unwrap(),
        )),
        capture(AdmittedFamilyDescriptor::Likelihood(
            s.parameter_likelihood(SpaceId([236; 32]), &zero, limits(), &mut work())
                .unwrap(),
        )),
        capture(AdmittedFamilyDescriptor::Jeffrey(jeffrey(&s, false))),
        capture(AdmittedFamilyDescriptor::Jeffrey(jeffrey(&s, true))),
    ];
    drop((s, a, f, zero));
    for (i, d) in cases.iter().enumerate() {
        match restore(d) {
            AdmittedFamilyDescriptor::Conditioning(r) => {
                assert_eq!(r.identity(), SpaceId([if i == 0 { 233 } else { 234 }; 32]));
                assert_eq!(r.is_impossible(), i == 1);
            }
            AdmittedFamilyDescriptor::Likelihood(r) => {
                assert_eq!(r.identity(), SpaceId([if i == 2 { 235 } else { 236 }; 32]));
                assert_eq!(r.is_impossible(), i == 3);
                assert_eq!(
                    at(r.normalizer(), "1", "2"),
                    Some(if i == 2 { ratio("3", "2") } else { 0u64.into() })
                );
            }
            AdmittedFamilyDescriptor::Jeffrey(r) => {
                assert_eq!(r.identity(), SpaceId([230; 32]));
                assert_eq!(r.is_impossible(), i == 5);
                assert_eq!(r.partition().cells().len(), 3);
                assert!(r.partition().cells()[2].is_empty());
                assert_eq!(r.unsupported_regions().len(), 3);
            }
            _ => panic!("revision"),
        }
    }
}

#[test]
fn family_transport_rejects_forged_receipt_numbers_domains_and_normalized_posteriors() {
    let s = shared_bias();
    let d = capture(AdmittedFamilyDescriptor::Jeffrey(jeffrey(&s, false)));
    let empty = ParameterRegion::empty(s.parameter_domain().unwrap().parameter())
        .to_bytes(limits().parameters, &mut work())
        .unwrap();
    for choice in 0..5 {
        let mut bad = d.clone();
        let FamilyDescriptor::Revision(r) = family(&mut bad) else {
            panic!("revision");
        };
        match choice {
            0 => r.defined = empty.clone(),
            1 => r.outcome = None,
            2 => {
                let FamilyReceiptDescriptor::Jeffrey { unsupported, .. } = &mut r.receipt else {
                    panic!("Jeffrey")
                };
                unsupported[0] = empty.clone();
            }
            3 => {
                let FamilyReceiptDescriptor::Jeffrey { old_masses, .. } = &mut r.receipt else {
                    panic!("Jeffrey")
                };
                old_masses.swap(0, 2);
            }
            _ => {
                let claimed = &mut r.outcome.as_mut().unwrap().posterior;
                let post = Event::from_bytes(claimed, &()).unwrap().space();
                *claimed = FamilyFunction::constant(
                    &post,
                    coefficient(&post, c(1), c(4)),
                    limits(),
                    &mut work(),
                )
                .unwrap()
                .designate(limits(), &mut work())
                .unwrap()
                .full()
                .to_bytes(&())
                .unwrap();
            }
        }
        assert!(
            matches!(
                bad.admit(dl(), &mut work()),
                Err(Error::DescriptorClaimMismatch)
            ),
            "case {choice}"
        );
    }
    let a = s.coordinate(0, &()).unwrap();
    let mut bad = capture(AdmittedFamilyDescriptor::Conditioning(
        s.parameter_condition(SpaceId([237; 32]), &a, limits(), &mut work())
            .unwrap(),
    ));
    let FamilyDescriptor::Revision(r) = family(&mut bad) else {
        panic!("revision")
    };
    let FamilyReceiptDescriptor::Condition { mass, .. } = &mut r.receipt else {
        panic!("condition")
    };
    let SourceDescriptor::Family(v) = capture(AdmittedFamilyDescriptor::Parameter(parameter(
        &s,
        c(1),
        c(1),
    ))) else {
        panic!("family")
    };
    let FamilyDescriptor::Parameter(one) = *v else {
        panic!("parameter")
    };
    *mass = one;
    assert!(matches!(
        bad.admit(dl(), &mut work()),
        Err(Error::DescriptorClaimMismatch)
    ));
}

#[test]
fn family_transport_refuses_malformed_bytes_active_holes_contexts_and_operational_failures() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let s = shared_bias();
    let d = capture(AdmittedFamilyDescriptor::Function(
        FamilyFunction::density(&s, limits(), &mut work()).unwrap(),
    ));
    let bytes = d.to_bytes(dl(), &()).unwrap();
    for i in 0..bytes.len() {
        assert!(SourceDescriptor::from_bytes(&bytes[..i], dl(), &()).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(matches!(
        SourceDescriptor::from_bytes(&extra, dl(), &()),
        Err(Error::InvalidEncoding)
    ));
    let mut version = bytes.clone();
    version[4] = 255;
    assert!(matches!(
        SourceDescriptor::from_bytes(&version, dl(), &()),
        Err(Error::UnsupportedVersion(255))
    ));
    let mut tag = bytes.clone();
    tag[5] = 255;
    assert!(matches!(
        SourceDescriptor::from_bytes(&tag, dl(), &()),
        Err(Error::InvalidEncoding)
    ));
    let mut bad = d.clone();
    let FamilyDescriptor::Function(f) = family(&mut bad) else {
        panic!("function")
    };
    let hole = crate::parameter::source::wire::encode_function(
        &coefficient(&s, p(), p()),
        limits(),
        &mut work(),
    )
    .unwrap();
    f.pieces[0].value = hole;
    assert!(matches!(
        bad.admit(dl(), &mut work()),
        Err(Error::UndefinedFunction)
    ));
    let mut bad = d.clone();
    let FamilyDescriptor::Function(f) = family(&mut bad) else {
        panic!("function")
    };
    f.pieces[0].region = space(238, 4, s.parameter_guards().unwrap())
        .empty()
        .to_bytes(&())
        .unwrap();
    assert!(matches!(
        bad.admit(dl(), &mut work()),
        Err(Error::SpaceMismatch)
    ));
    let mut small = dl();
    small.descriptors.items = 0;
    assert!(matches!(
        d.admit(small, &mut work()),
        Err(Error::Capacity(Capacity::DescriptorItems))
    ));
    small = dl();
    small.parameters.steps = 0;
    assert!(matches!(
        d.admit(small, &mut work()),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
    assert!(matches!(
        d.admit(
            dl(),
            &mut ExactArithmetic::new(
                ArithmeticLimits {
                    operations: 0,
                    ..ArithmeticLimits::default()
                },
                &()
            )
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    assert!(matches!(
        d.admit(
            dl(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        SourceDescriptor::from_bytes(&bytes, dl(), &Cancel),
        Err(Error::Cancelled)
    ));
}

#[test]
fn family_transport_replays_piecewise_targets_and_rejects_missing_defined_points() {
    let s = shared_bias();
    let a = s.coordinate(0, &()).unwrap();
    let lower = sign(&sub(&mul(&c(2), &p()), &c(1)), PolynomialSigns::NEGATIVE);
    let mut targets = Vec::new();
    for invert in [false, true] {
        let pieces = [
            (lower.clone(), u64::from(invert)),
            (lower.complement(), u64::from(!invert)),
        ]
        .into_iter()
        .map(|(region, n)| {
            coefficient(&s, c(n), c(1))
                .restrict(&region, limits().parameters.region, &mut work())
                .unwrap()
        })
        .collect::<Vec<_>>();
        targets.push(
            ParameterFunction::new(
                s.parameter_domain().unwrap().clone(),
                &pieces,
                limits().parameters.region,
                limits().functions,
                &mut work(),
            )
            .unwrap(),
        );
    }
    targets.push(parameter(&s, c(0), c(1)));
    let receipt = s
        .parameter_jeffrey(
            SpaceId([239; 32]),
            &partition(&s),
            &targets,
            limits(),
            &mut work(),
        )
        .unwrap();
    let data = capture(AdmittedFamilyDescriptor::Jeffrey(receipt));
    let AdmittedFamilyDescriptor::Jeffrey(receipt) = restore(&data) else {
        panic!("Jeffrey");
    };
    let revised = receipt.revised().unwrap();
    assert!(revised.refined_prior().dimensions() > s.dimensions());
    let mass = revised
        .pullback(&a, &())
        .unwrap()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    for n in 0..=4 {
        assert_eq!(
            at(&mass, &n.to_string(), "4"),
            Some(u64::from(n >= 2).into())
        );
    }

    // p²/p and p agree away from zero; a receipt must also preserve definedness.
    let mut data = capture(AdmittedFamilyDescriptor::Conditioning(
        s.parameter_condition(SpaceId([240; 32]), &a, limits(), &mut work())
            .unwrap(),
    ));
    let FamilyDescriptor::Revision(r) = family(&mut data) else {
        panic!("revision");
    };
    let FamilyReceiptDescriptor::Condition { mass, .. } = &mut r.receipt else {
        panic!("condition");
    };
    let mut partial = capture(AdmittedFamilyDescriptor::Parameter(parameter(
        &s,
        mul(&p(), &p()),
        p(),
    )));
    let FamilyDescriptor::Parameter(partial) = family(&mut partial) else {
        panic!("parameter");
    };
    *mass = partial.clone();
    assert!(matches!(
        data.admit(dl(), &mut work()),
        Err(Error::DescriptorClaimMismatch)
    ));
}

#[test]
fn family_transport_checks_likelihood_scale_independently_of_posterior() {
    let s = shared_bias();
    let mut receipts = Vec::new();
    for n in [1, 2] {
        let factor =
            FamilyFunction::constant(&s, coefficient(&s, c(n), c(1)), limits(), &mut work())
                .unwrap();
        receipts.push(
            s.parameter_likelihood(SpaceId([241; 32]), &factor, limits(), &mut work())
                .unwrap(),
        );
    }
    let densities = receipts
        .iter()
        .map(|r| {
            r.revised()
                .unwrap()
                .space()
                .parameter_density_pieces()
                .unwrap()
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for ((_, a), (_, b)) in densities[0].iter().zip(&densities[1]) {
        assert!(
            a.equivalent(b, limits().parameters.region, &mut work())
                .unwrap()
        );
    }
    let mut data = capture(AdmittedFamilyDescriptor::Likelihood(receipts.remove(0)));
    let mut scaled = capture(AdmittedFamilyDescriptor::Likelihood(receipts.remove(0)));
    let FamilyDescriptor::Revision(r) = family(&mut data) else {
        panic!("revision")
    };
    let FamilyDescriptor::Revision(other) = family(&mut scaled) else {
        panic!("revision")
    };
    let FamilyReceiptDescriptor::Likelihood { normalizer, .. } = &mut r.receipt else {
        panic!("likelihood")
    };
    let FamilyReceiptDescriptor::Likelihood {
        normalizer: wrong, ..
    } = &other.receipt
    else {
        panic!("likelihood")
    };
    *normalizer = wrong.clone();
    assert!(matches!(
        data.admit(dl(), &mut work()),
        Err(Error::DescriptorClaimMismatch)
    ));
}

#[test]
fn family_transport_bounds_nested_arithmetic_rosters_and_envelopes() {
    let s = shared_bias();
    let data = capture(AdmittedFamilyDescriptor::Jeffrey(jeffrey(&s, false)));
    let bytes = data.to_bytes(dl(), &()).unwrap();
    let mut small = dl();
    small.descriptors.bytes = bytes.len() - 1;
    assert!(matches!(
        data.to_bytes(small, &()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    assert!(matches!(
        SourceDescriptor::from_bytes(&bytes, small, &()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    small = dl();
    small.partitions.cells = 2;
    assert!(matches!(
        data.admit(small, &mut work()),
        Err(Error::Capacity(Capacity::PartitionCells))
    ));
    assert!(matches!(
        SourceDescriptor::from_bytes(&bytes, small, &()),
        Err(Error::Capacity(Capacity::PartitionCells))
    ));
    let mut bad = data.clone();
    let FamilyDescriptor::Revision(r) = family(&mut bad) else {
        panic!("revision")
    };
    let FamilyReceiptDescriptor::Jeffrey { unsupported, .. } = &mut r.receipt else {
        panic!("Jeffrey")
    };
    unsupported.pop();
    assert!(matches!(
        bad.to_bytes(dl(), &()),
        Err(Error::PartitionArity)
    ));
    assert!(matches!(
        bad.admit(dl(), &mut work()),
        Err(Error::PartitionArity)
    ));

    // A budget sufficient to decode the prior must not reset for later inputs,
    // claimed masses or the actual replay of the update.
    let mut measured = work();
    Event::from_bytes_with_parameter_limits(
        &s.full().to_bytes(&()).unwrap(),
        None,
        dl().descriptors.events,
        limits(),
        &mut measured,
    )
    .unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: measured.operations(),
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(matches!(
        data.admit(dl(), &mut bounded),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));

    // A malicious count is refused before vector allocation. Parameter-function
    // bodies start with one ambient blob and then their piece count.
    let partial = capture(AdmittedFamilyDescriptor::Parameter(parameter(&s, p(), p())));
    let mut bytes = partial.to_bytes(dl(), &()).unwrap();
    let ambient = usize::try_from(u64::from_le_bytes(bytes[6..14].try_into().unwrap())).unwrap();
    bytes[14 + ambient..22 + ambient].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(matches!(
        SourceDescriptor::from_bytes(&bytes, dl(), &()),
        Err(Error::Capacity(Capacity::DescriptorItems))
    ));
}

#[test]
fn family_transport_pinned_v2_partial_parameter_function() {
    let s = shared_bias();
    let data = capture(AdmittedFamilyDescriptor::Parameter(parameter(&s, p(), p())));
    let bytes = data.to_bytes(dl(), &()).unwrap();
    let hex = include_str!("../tests/fixtures/source-v2-parameter-function.hex").trim();
    let fixture: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(bytes, fixture);
    let AdmittedFamilyDescriptor::Parameter(f) = restore(&data) else {
        panic!("parameter");
    };
    assert_eq!(at(&f, "0", "1"), None);
    assert_eq!(at(&f, "1", "2"), Some(1u64.into()));
    assert_eq!(at(&f, "1", "1"), Some(1u64.into()));
}
