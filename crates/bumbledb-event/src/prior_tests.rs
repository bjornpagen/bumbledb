use crate::parameter_source_tests::{
    c, domain, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn prior(source: &Space, a: u64, b: u64) -> BetaSource {
    BetaSource::new(source, a.into(), b.into(), limits(), &mut work()).unwrap()
}
fn table(source: &Space, bits: u64) -> Event {
    source.table(3, &[bits], &()).unwrap()
}
fn guarded(n: ExactPolynomial, d: ExactPolynomial) -> GuardedRationalFunction {
    GuardedRationalFunction::new(domain(), n, d, limits().parameters.region, &mut work()).unwrap()
}
fn function(parts: &[GuardedRationalFunction]) -> ParameterFunction {
    ParameterFunction::new(
        domain(),
        parts,
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}

// Independent beta-binomial path formula, with distinct labelled draws.
fn weights(a: u64, b: u64) -> [ExactRational; 4] {
    let denominator = (a + b) * (a + b + 1);
    [b * (b + 1), a * b, a * b, a * (a + 1)]
        .map(|n| ratio(&n.to_string(), &denominator.to_string()))
}
fn sum(mask: u64, masses: &[ExactRational; 4]) -> ExactRational {
    masses
        .iter()
        .enumerate()
        .filter(|&(i, _)| mask & (1 << i) != 0)
        .fold(ExactRational::zero(), |total, (_, m)| {
            total.add(m, &mut work()).unwrap()
        })
}

#[test]
fn explicit_prior_observations_match_all_labelled_two_draw_events_and_evidence() {
    let source = shared_bias();
    for (a, b) in [(1, 1), (2, 3), (7, 2)] {
        let bound = prior(&source, a, b);
        let masses = weights(a, b);
        for event in 0..16 {
            for given in 0..16 {
                let result = bound
                    .probability(
                        &table(&source, event),
                        &table(&source, given),
                        limits(),
                        &mut work(),
                    )
                    .unwrap();
                let n = sum(event & given, &masses);
                let z = sum(given, &masses);
                assert_eq!(result.numerator().value(), &n);
                assert_eq!(result.evidence_mass().value(), &z);
                let expected = if z.is_zero() {
                    None
                } else {
                    Some(n.div(&z, &mut work()).unwrap())
                };
                assert_eq!(result.value(), expected.as_ref());
                assert_eq!(result.original().event(), &table(&source, event));
                assert_eq!(result.original().given(), &table(&source, given));
            }
        }
    }
}

#[test]
fn shared_binding_and_conditioning_precede_integration_and_normalization() {
    let source = shared_bias();
    let bound = prior(&source, 1, 1);
    let first = source.coordinate(0, &()).unwrap();
    let second = source.coordinate(1, &()).unwrap();
    let both = first.apply(BoolOp4::AND, &second, &()).unwrap();
    let joint = bound
        .probability(&both, &source.full(), limits(), &mut work())
        .unwrap();
    assert_eq!(joint.value(), Some(&ratio("1", "3")));
    let a = bound
        .probability(&first, &source.full(), limits(), &mut work())
        .unwrap();
    let b = bound
        .probability(&second, &source.full(), limits(), &mut work())
        .unwrap();
    assert_eq!(
        a.value()
            .unwrap()
            .mul(b.value().unwrap(), &mut work())
            .unwrap(),
        ratio("1", "4")
    );
    let conditional = second
        .parameter_probability(&first, limits(), &mut work())
        .unwrap();
    let posterior = bound
        .bind_probability(&conditional, limits(), &mut work())
        .unwrap();
    assert_eq!(posterior.value(), Some(&ratio("2", "3")));
    assert_eq!(posterior.evidence_mass().value(), &ratio("1", "2"));
    // The conditional p has an inherited hole at p=0; it is not the integrand.
    assert_eq!(
        bound
            .integrate(conditional.conditional(), limits(), &mut work())
            .unwrap_err(),
        Error::UndefinedFunction
    );
    assert_eq!(
        conditional
            .value_at(&0u64.into(), limits(), &mut work())
            .unwrap(),
        None
    );
    let asymmetric = BetaSource::new(
        &source,
        ratio("1", "2"),
        ratio("3", "2"),
        limits(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(
        asymmetric
            .probability(&second, &first, limits(), &mut work())
            .unwrap()
            .value(),
        Some(&ratio("1", "2"))
    );
}

#[test]
fn isolated_parameter_events_keep_their_identity_even_when_prior_mass_is_zero() {
    let source = shared_bias();
    let endpoint = source.coordinate(2, &()).unwrap();
    assert!(!endpoint.is_empty());
    let bound = prior(&source, 1, 1);
    let observation = bound
        .probability(&endpoint, &source.full(), limits(), &mut work())
        .unwrap();
    assert_eq!(observation.value(), Some(&ExactRational::zero()));
    assert!(!observation.numerator().exceptions().is_empty());
    assert_eq!(observation.original().event(), &endpoint);
    let impossible = bound
        .probability(&source.full(), &endpoint, limits(), &mut work())
        .unwrap();
    assert!(impossible.is_impossible());
    assert!(!impossible.original().given().is_empty());
    assert_eq!(
        impossible
            .original()
            .evidence_mass()
            .value_at(
                &0u64.into(),
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap(),
        Some(1u64.into())
    );
}

#[test]
fn scalar_binding_checks_polynomial_identity_across_all_open_pieces() {
    let source = shared_bias();
    let bound = prior(&source, 2, 3);
    let factor = c(1)
        .add(&p(), limits().parameters.region.polynomial, &mut work())
        .unwrap();
    let polynomial = guarded(mul(&p(), &factor), factor.clone());
    let at_half = sign(&sub(&mul(&c(2), &p()), &c(1)), PolynomialSigns::ZERO);
    let ordinary = polynomial
        .restrict(
            &at_half.complement(),
            limits().parameters.region,
            &mut work(),
        )
        .unwrap();
    let exception = guarded(c(9), c(1))
        .restrict(&at_half, limits().parameters.region, &mut work())
        .unwrap();
    let input = function(&[ordinary, exception]);
    let integral = bound.integrate(&input, limits(), &mut work()).unwrap();
    assert_eq!(integral.polynomial(), &p());
    assert_eq!(integral.value(), &ratio("2", "5"));
    assert!(
        integral
            .exceptions()
            .equivalent(&at_half, limits().parameters.region, &mut work())
            .unwrap()
    );
    let nonpolynomial = function(&[guarded(c(1), factor)]);
    assert_eq!(
        bound
            .integrate(&nonpolynomial, limits(), &mut work())
            .unwrap_err(),
        Error::UnsupportedBetaIntegrand
    );
    let half = sign(&sub(&mul(&c(2), &p()), &c(1)), PolynomialSigns::NEGATIVE);
    let step = function(&[
        guarded(c(0), c(1))
            .restrict(&half, limits().parameters.region, &mut work())
            .unwrap(),
        guarded(c(1), c(1))
            .restrict(&half.complement(), limits().parameters.region, &mut work())
            .unwrap(),
    ]);
    assert_eq!(
        bound.integrate(&step, limits(), &mut work()).unwrap_err(),
        Error::UnsupportedBetaIntegrand
    );
}

#[test]
fn finite_and_parameter_payoffs_keep_signed_utilities_and_original_evidence() {
    let source = shared_bias();
    let bound = prior(&source, 1, 1);
    let pieces: Vec<_> = [-4, 2, 7, -1]
        .iter()
        .enumerate()
        .map(|(i, v)| FunctionPiece {
            region: table(&source, 1 << i),
            value: ratio(&v.to_string(), "1"),
        })
        .collect();
    let payoff = FiniteFunction::new(&source, &pieces, limits().functions, &mut work()).unwrap();
    let given = source.coordinate(0, &()).unwrap();
    let observed = bound
        .expectation(&payoff, &given, limits(), &mut work())
        .unwrap();
    assert_eq!(observed.value(), Some(&ratio("0", "1")));
    assert_eq!(observed.evidence_mass().value(), &ratio("1", "2"));
    assert!(
        observed
            .original()
            .function()
            .equivalent(&payoff, &())
            .unwrap()
    );
    let family = FamilyFunction::from_parameter(
        &source,
        &function(&[guarded(p(), c(1))]),
        limits(),
        &mut work(),
    )
    .unwrap();
    let observation = bound
        .family_expectation(&family, &given, limits(), &mut work())
        .unwrap();
    assert_eq!(observation.value(), Some(&ratio("2", "3")));
    drop((bound, source, payoff, family));
    assert_eq!(
        observation.original().evidence().to_bytes(&()).unwrap(),
        given.to_bytes(&()).unwrap()
    );
}

#[test]
fn prior_scope_capabilities_and_resources_refuse_without_guessing() {
    let source = shared_bias();
    for bad in [ExactRational::zero(), ratio("-1", "1")] {
        for (a, b) in [(bad.clone(), 1u64.into()), (1u64.into(), bad)] {
            assert_eq!(
                BetaSource::new(&source, a, b, limits(), &mut work()).unwrap_err(),
                Error::InvalidBetaPrior
            );
        }
    }
    let structural = space(202, 1, &[]);
    assert_eq!(
        BetaSource::new(&structural, 1u64.into(), 1u64.into(), limits(), &mut work()).unwrap_err(),
        Error::MissingLaw
    );
    let narrow = ParameterRestriction::new(
        SpaceId([203; 32]),
        &source,
        &sign(&p(), PolynomialSigns::POSITIVE),
        limits(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(
        BetaSource::new(
            narrow.space(),
            1u64.into(),
            1u64.into(),
            limits(),
            &mut work()
        )
        .unwrap_err(),
        Error::UnsupportedBetaDomain
    );
    let bound = prior(&source, 1, 1);
    assert_eq!(
        bound
            .probability(&structural.empty(), &source.empty(), limits(), &mut work())
            .unwrap_err(),
        Error::SpaceMismatch
    );
    let mut tiny = limits();
    tiny.functions.cells = 0;
    assert!(matches!(
        bound.integrate(&function(&[guarded(c(0), c(1))]), tiny, &mut work()),
        Err(Error::Capacity(_))
    ));
    assert_eq!(
        bound
            .probability(
                &source.empty(),
                &source.empty(),
                limits(),
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
            )
            .unwrap_err(),
        Error::Cancelled
    );
}

#[test]
fn sparse_division_retains_remainders_and_checks_discarded_parameters() {
    let den = sub(&p(), &c(1));
    for degree in 0..12 {
        let n = p()
            .pow(degree, limits().parameters.region.polynomial, &mut work())
            .unwrap();
        let (q, r) = n
            .div_rem_univariate(
                &den,
                domain().parameter(),
                limits().parameters.region.polynomial,
                &mut work(),
            )
            .unwrap();
        assert_eq!(r, c(1));
        assert_eq!(
            q.mul(&den, limits().parameters.region.polynomial, &mut work())
                .unwrap()
                .add(&r, limits().parameters.region.polynomial, &mut work())
                .unwrap(),
            n
        );
    }
    let foreign = ExactPolynomial::parameter(ParameterId([77; 32]));
    assert_eq!(
        c(0).div_rem_univariate(
            &foreign,
            domain().parameter(),
            limits().parameters.region.polynomial,
            &mut work()
        )
        .unwrap_err(),
        Error::NotUnivariate
    );
    assert_eq!(
        c(0).div_rem_univariate(
            &c(0),
            domain().parameter(),
            limits().parameters.region.polynomial,
            &mut work()
        )
        .unwrap_err(),
        Error::DivisionByZero
    );
}

struct Cancel;
impl Control for Cancel {
    fn checkpoint(&self) -> Result<()> {
        Err(Error::Cancelled)
    }
}
