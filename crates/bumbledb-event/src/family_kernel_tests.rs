#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{and, c, limits, mul, p, ratio, shared_bias, space, sub, work};
use crate::*;

fn value(
    source: &Space,
    numerator: ExactPolynomial,
    denominator: ExactPolynomial,
) -> GuardedRationalFunction {
    GuardedRationalFunction::new(
        source.parameter_domain().unwrap().clone(),
        numerator,
        denominator,
        limits().parameters.region,
        &mut work(),
    )
    .unwrap()
}
fn family(source: &Space, pieces: &[(Event, ExactPolynomial, ExactPolynomial)]) -> FamilyFunction {
    FamilyFunction::new(
        source,
        &pieces
            .iter()
            .map(|(region, n, d)| FamilyFunctionPiece {
                region: region.clone(),
                value: value(source, n.clone(), d.clone()),
            })
            .collect::<Vec<_>>(),
        limits(),
        &mut work(),
    )
    .unwrap()
}
fn coin(source: &Space, bit: u8) -> FamilyFunction {
    let head = source.coordinate(bit, &()).unwrap();
    family(
        source,
        &[
            (head.clone(), p(), c(1)),
            (head.complement(), sub(&c(1), &p()), c(1)),
        ],
    )
}
fn assert_same(a: &ParameterFunction, b: &ParameterFunction) {
    assert!(
        a.equivalent(
            b,
            limits().parameters.region,
            limits().functions,
            &mut work()
        )
        .unwrap()
    );
}

#[test]
fn family_channels_preserve_marginals_and_share_the_same_unknown_parameter() {
    let raw = space(204, 0, &[]);
    let unit = family(&raw, &[(raw.full(), c(1), c(1))])
        .designate(limits(), &mut work())
        .unwrap();
    let first_raw = space(205, 1, &[]);
    let first_map = CoordinateMap::coordinates(&first_raw, &unit, &[], &()).unwrap();
    let first_kernel =
        FamilyKernel::new(&first_map, &coin(&first_raw, 0), limits(), &mut work()).unwrap();
    let first = first_kernel.close(&unit, limits(), &mut work()).unwrap();
    let second_raw = space(206, 2, &[]);
    let second_map = CoordinateMap::coordinates(&second_raw, first.space(), &[0], &()).unwrap();
    let second_kernel =
        FamilyKernel::new(&second_map, &coin(&second_raw, 1), limits(), &mut work()).unwrap();
    let second = second_kernel
        .close(first.space(), limits(), &mut work())
        .unwrap();
    let old_head = first.space().coordinate(0, &()).unwrap();
    let a = second.parent().pullback(&old_head, &()).unwrap();
    let b = second.space().coordinate(1, &()).unwrap();
    assert_same(
        &a.parameter_mass(limits(), &mut work()).unwrap(),
        &old_head.parameter_mass(limits(), &mut work()).unwrap(),
    );
    let both = and(&a, &b);
    let expected = ParameterFunction::new(
        second.space().parameter_domain().unwrap().clone(),
        &[value(second.space(), mul(&p(), &p()), c(1))],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    assert_same(
        &both.parameter_mass(limits(), &mut work()).unwrap(),
        &expected,
    );
    // Distinct outcomes survive: this is a fresh draw, not the same Event twice.
    assert!(!a.equivalent(&b).unwrap());
    assert_eq!(second.space().full().atom_count(&()).unwrap(), 4);
    let marginal = FamilyFunction::density(second.space(), limits(), &mut work())
        .unwrap()
        .pushforward(second.parent(), limits(), &mut work())
        .unwrap();
    assert!(
        marginal
            .equivalent(
                &FamilyFunction::density(first.space(), limits(), &mut work()).unwrap(),
                limits(),
                &mut work()
            )
            .unwrap()
    );
    let bytes = both.to_bytes(&()).unwrap();
    drop((
        raw,
        unit,
        first_raw,
        first_map,
        first_kernel,
        first,
        second_raw,
        second_map,
        second_kernel,
        second,
        a,
        b,
        both,
    ));
    let restored = Event::from_bytes(&bytes, &()).unwrap();
    assert_eq!(
        restored
            .parameter_mass(limits(), &mut work())
            .unwrap()
            .value_at(
                &ratio("1", "3"),
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap(),
        Some(ratio("1", "9"))
    );
}

#[test]
fn family_channel_information_fd_compares_values_not_authored_cells() {
    let parent = space(207, 2, &[]);
    let source = space(208, 3, &[]);
    let visible = space(209, 2, &[]);
    let parent_map = CoordinateMap::coordinates(&source, &parent, &[0, 1], &()).unwrap();
    let observation = CoordinateMap::coordinates(&source, &visible, &[1, 2], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let hidden = source.coordinate(0, &()).unwrap();
    let fresh = source.coordinate(2, &()).unwrap();
    let q = sub(&c(1), &p());
    let equivalent = family(
        &source,
        &[
            (and(&hidden, &fresh), mul(&c(2), &p()), c(2)),
            (and(&hidden.complement(), &fresh), p(), c(1)),
            (and(&hidden, &fresh.complement()), mul(&c(2), &q), c(2)),
            (
                and(&hidden.complement(), &fresh.complement()),
                q.clone(),
                c(1),
            ),
        ],
    );
    let kernel = FamilyKernel::new(&parent_map, &equivalent, limits(), &mut work()).unwrap();
    assert!(
        kernel
            .factors_through(&observation, limits(), &mut work())
            .unwrap()
    );
    assert!(matches!(
        observation.descend(&and(&hidden, &fresh), &()),
        Err(Error::RoleMismatch)
    ));
    let leaked = family(
        &source,
        &[
            (and(&hidden, &fresh), q.clone(), c(1)),
            (and(&hidden.complement(), &fresh), p(), c(1)),
            (and(&hidden, &fresh.complement()), p(), c(1)),
            (and(&hidden.complement(), &fresh.complement()), q, c(1)),
        ],
    );
    let leaked = FamilyKernel::new(&parent_map, &leaked, limits(), &mut work()).unwrap();
    assert!(
        !leaked
            .factors_through(&observation, limits(), &mut work())
            .unwrap()
    );
    // A channel can be reused with an explicitly replaced law in the same named source.
    let prior = coin(&parent, 0)
        .multiply(
            &family(&parent, &[(parent.full(), c(1), c(2))]),
            limits(),
            &mut work(),
        )
        .unwrap()
        .designate(limits(), &mut work())
        .unwrap();
    let closed = leaked.close(&prior, limits(), &mut work()).unwrap();
    let marginal = FamilyFunction::density(closed.space(), limits(), &mut work())
        .unwrap()
        .pushforward(closed.parent(), limits(), &mut work())
        .unwrap();
    assert!(
        marginal
            .equivalent(
                &FamilyFunction::density(&prior, limits(), &mut work()).unwrap(),
                limits(),
                &mut work()
            )
            .unwrap()
    );
    assert!(matches!(
        leaked.close(&parent, limits(), &mut work()),
        Err(Error::MissingLaw)
    ));
    assert!(matches!(
        leaked.close(&visible, limits(), &mut work()),
        Err(Error::SpaceMismatch)
    ));
}

#[test]
fn family_channels_check_zero_prior_rows_and_exclude_unused_outcome_codes() {
    let prior = shared_bias();
    let source = space(210, 5, prior.parameter_guards().unwrap());
    let map = CoordinateMap::coordinates(&source, &prior, &[0, 1, 2, 3], &()).unwrap();
    let bad = and(
        &source.coordinate(0, &()).unwrap(),
        &source.coordinate(2, &()).unwrap(),
    );
    let next = source.coordinate(4, &()).unwrap();
    let malformed = family(
        &source,
        &[
            (and(&bad.complement(), &next), p(), c(1)),
            (
                and(&bad.complement(), &next.complement()),
                sub(&c(1), &p()),
                c(1),
            ),
        ],
    );
    assert!(matches!(
        FamilyKernel::new(&map, &malformed, limits(), &mut work()),
        Err(Error::KernelNotNormalized)
    ));
    let negative = family(
        &source,
        &[(bad, ExactPolynomial::constant((-1i64).into()), c(1))],
    );
    assert!(matches!(
        FamilyKernel::new(&map, &negative, limits(), &mut work()),
        Err(Error::NegativeMass)
    ));
    let parent = space(211, 1, &[]);
    let raw = space(212, 3, &[]);
    let legal = and(
        &raw.coordinate(1, &()).unwrap(),
        &raw.coordinate(2, &()).unwrap(),
    )
    .complement();
    let ternary = raw
        .restrict_with_parameters(&legal, limits(), &mut work())
        .unwrap();
    let a = ternary.coordinate(1, &()).unwrap();
    let b = ternary.coordinate(2, &()).unwrap();
    let density = family(
        &ternary,
        &[
            (a.clone(), p(), c(1)),
            (b.clone(), sub(&c(1), &p()), c(2)),
            (
                and(&a.complement(), &b.complement()),
                sub(&c(1), &p()),
                c(2),
            ),
        ],
    );
    let map = CoordinateMap::coordinates(&ternary, &parent, &[0], &()).unwrap();
    let kernel = FamilyKernel::new(&map, &density, limits(), &mut work()).unwrap();
    let prior = coin(&parent, 0).designate(limits(), &mut work()).unwrap();
    let closed = kernel.close(&prior, limits(), &mut work()).unwrap();
    assert_eq!(closed.space().full().atom_count(&()).unwrap(), 6);
    assert!(matches!(
        density.value_at(&ratio("1", "2"), 6, limits(), &mut work()),
        Err(Error::IllegalWorld(6))
    ));
}

#[test]
fn family_channels_preserve_operational_failures() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let parent = space(213, 0, &[]);
    let source = space(214, 1, &[]);
    let map = CoordinateMap::coordinates(&source, &parent, &[], &()).unwrap();
    let density = coin(&source, 0);
    assert!(matches!(
        FamilyKernel::new(
            &map,
            &density,
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        FamilyKernel::new(
            &map,
            &density,
            ParameterSourceLimits {
                steps: 0,
                ..limits()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
    assert!(matches!(
        FamilyKernel::new(
            &map,
            &density,
            limits(),
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
    let kernel = FamilyKernel::new(&map, &density, limits(), &mut work()).unwrap();
    let prior = family(&parent, &[(parent.full(), c(1), c(1))])
        .designate(limits(), &mut work())
        .unwrap();
    assert!(matches!(
        kernel.close(
            &prior,
            ParameterSourceLimits {
                steps: 0,
                ..limits()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
}
