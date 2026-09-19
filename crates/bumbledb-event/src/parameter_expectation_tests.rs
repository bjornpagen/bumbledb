#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{
    c, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn payoff(source: &Space, values: [i64; 4]) -> FiniteFunction {
    let pieces: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, value)| FunctionPiece {
            region: source.table(3, &[1 << i], &()).unwrap(),
            value: ratio(&value.to_string(), "1"),
        })
        .collect();
    FiniteFunction::new(source, &pieces, limits().functions, &mut work()).unwrap()
}

fn value(function: &ParameterFunction, numerator: i64, denominator: i64) -> Option<ExactRational> {
    function
        .value_at(
            &ratio(&numerator.to_string(), &denominator.to_string()),
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap()
}

fn equivalent(a: &ParameterFunction, b: &ParameterFunction) -> bool {
    a.equivalent(
        b,
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}

#[test]
fn signed_family_expectations_match_independent_outcome_sums() {
    let source = shared_bias();
    for values in [[-4, 2, 7, -1], [0, 0, 0, 0], [3, 3, 3, 3]] {
        let function = payoff(&source, values);
        for mask in 0..16 {
            let evidence = source.table(3, &[mask], &()).unwrap();
            let observation = function
                .parameter_expectation(&evidence, limits(), &mut work())
                .unwrap();
            assert!(observation.function().equivalent(&function, &()).unwrap());
            assert_eq!(observation.evidence(), &evidence);
            assert_eq!(observation.is_impossible(), mask == 0);
            for n in 0..=8 {
                let masses = [(8 - n) * (8 - n), n * (8 - n), n * (8 - n), n * n];
                let mut numerator = 0;
                let mut denominator = 0;
                for (index, (mass, payoff)) in masses.into_iter().zip(values).enumerate() {
                    if mask & (1 << index) != 0 {
                        numerator += mass * payoff;
                        denominator += mass;
                    }
                }
                assert_eq!(
                    value(observation.numerator(), n, 8),
                    Some(ratio(&numerator.to_string(), "64"))
                );
                assert_eq!(
                    value(observation.evidence_mass(), n, 8),
                    Some(ratio(&denominator.to_string(), "64"))
                );
                assert_eq!(
                    observation
                        .value_at(&ratio(&n.to_string(), "8"), limits(), &mut work())
                        .unwrap(),
                    (denominator != 0)
                        .then(|| ratio(&numerator.to_string(), &denominator.to_string()))
                );
            }
        }
    }
}

#[test]
fn family_expectation_is_linear_and_indicator_agrees_with_probability() {
    let source = shared_bias();
    let first = source.coordinate(0, &()).unwrap();
    let evidence = first
        .apply(BoolOp4::XOR, &source.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let a = payoff(&source, [-4, 2, 7, -1]);
    let b = payoff(&source, [3, 5, -9, 2]);
    let left = a
        .add(&b, limits().functions, &mut work())
        .unwrap()
        .parameter_expectation(&evidence, limits(), &mut work())
        .unwrap();
    let a = a
        .parameter_expectation(&evidence, limits(), &mut work())
        .unwrap();
    let b = b
        .parameter_expectation(&evidence, limits(), &mut work())
        .unwrap();
    let right = a
        .conditional()
        .add(
            b.conditional(),
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    assert!(equivalent(left.conditional(), &right));
    let indicator = payoff(&source, [0, 1, 0, 1])
        .parameter_expectation(&evidence, limits(), &mut work())
        .unwrap();
    let probability = first
        .parameter_probability(&evidence, limits(), &mut work())
        .unwrap();
    assert!(equivalent(indicator.numerator(), probability.numerator()));
    assert!(equivalent(
        indicator.conditional(),
        probability.conditional()
    ));
    let zero = payoff(&source, [0; 4])
        .parameter_expectation(&evidence, limits(), &mut work())
        .unwrap();
    assert!(!zero.is_impossible());
    assert_eq!(value(zero.numerator(), 0, 1), Some(0u64.into()));
    assert_eq!(value(zero.conditional(), 0, 1), None);
    assert_eq!(value(zero.conditional(), 1, 2), Some(0u64.into()));
    assert_eq!(value(zero.conditional(), 1, 1), None);
}

#[test]
fn family_payoff_keeps_irrational_domains_and_exact_sign_regions() {
    let source = shared_bias();
    let roots = sign(
        &sub(&mul(&c(2), &mul(&p(), &p())), &c(1)),
        PolynomialSigns::ZERO,
    );
    let restriction =
        ParameterRestriction::new(SpaceId([172; 32]), &source, &roots, limits(), &mut work())
            .unwrap();
    let function = payoff(restriction.space(), [-2, 4, -2, 4]);
    let observation = function
        .parameter_expectation(&restriction.space().full(), limits(), &mut work())
        .unwrap();
    let positive = observation
        .conditional()
        .where_sign(
            PolynomialSigns::POSITIVE,
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    assert!(
        positive
            .equivalent(
                restriction.space().parameter_domain().unwrap().region(),
                limits().parameters.region,
                &mut work()
            )
            .unwrap()
    );
    assert!(matches!(
        positive
            .witness(limits().parameters.region, &mut work())
            .unwrap(),
        Some(RealWitness::Algebraic(_))
    ));
    assert_eq!(value(observation.conditional(), 1, 2), None);
}

#[test]
fn family_expectation_retains_decoded_inputs_and_possible_zero_mass_evidence() {
    let source = shared_bias();
    let first = source.coordinate(0, &()).unwrap();
    let zero_parameter = source.coordinate(2, &()).unwrap();
    let evidence = first.apply(BoolOp4::AND, &zero_parameter, &()).unwrap();
    assert!(!evidence.is_empty());
    let decoded = Event::from_bytes(&evidence.to_bytes(&()).unwrap(), &()).unwrap();
    let function = payoff(&source, [-4, 2, 7, -1]);
    let observation = function
        .parameter_expectation(&decoded, limits(), &mut work())
        .unwrap();
    drop((source, first, zero_parameter, evidence, decoded, function));
    assert!(observation.is_impossible());
    assert!(!observation.evidence().is_empty());
    assert_eq!(value(observation.evidence_mass(), 0, 1), Some(0u64.into()));
    assert_eq!(value(observation.conditional(), 0, 1), None);
    assert_eq!(observation.function().at(5, &()).unwrap(), 2u64.into());
}

#[test]
fn expected_payoff_yields_an_exact_event_of_profitable_parameter_cases() {
    let source = shared_bias();
    let observation = payoff(&source, [-2, 4, -2, 4])
        .parameter_expectation(&source.full(), limits(), &mut work())
        .unwrap();
    let polynomial = sub(&mul(&c(6), &p()), &c(2));
    let expected = ParameterFunction::new(
        source.parameter_domain().unwrap().clone(),
        &[GuardedRationalFunction::new(
            source.parameter_domain().unwrap().clone(),
            polynomial.clone(),
            c(1),
            limits().parameters.region,
            &mut work(),
        )
        .unwrap()],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    assert!(equivalent(observation.conditional(), &expected));
    let profitable = observation
        .conditional()
        .where_sign(
            PolynomialSigns::POSITIVE,
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    let refinement = ParameterRefinement::new(
        SpaceId([175; 32]),
        &source,
        std::slice::from_ref(&profitable),
        limits(),
        &mut work(),
    )
    .unwrap();
    let event = refinement
        .refined()
        .parameter_event(&profitable, limits(), &mut work())
        .unwrap();
    let mass = event.parameter_mass(limits(), &mut work()).unwrap();
    assert_eq!(value(&mass, 1, 3), Some(0u64.into()));
    assert_eq!(value(&mass, 1, 2), Some(1u64.into()));
    assert!(!event.is_empty());
    assert!(!event.complement().is_empty());
}

#[test]
fn family_expectation_checks_zero_payoff_contexts_and_operation_limits() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let source = shared_bias();
    let function = payoff(&source, [0; 4]);
    let unmeasured = space(173, 2, &[]);
    let unmeasured_zero = payoff(&unmeasured, [0; 4]);
    assert!(matches!(
        unmeasured_zero.parameter_expectation(&unmeasured.full(), limits(), &mut work()),
        Err(Error::MissingLaw)
    ));
    assert!(matches!(
        function.parameter_expectation(&unmeasured.empty(), limits(), &mut work()),
        Err(Error::SpaceMismatch)
    ));
    let finite = Space::new(SpaceId([174; 32]), 2, &()).unwrap();
    assert!(matches!(
        payoff(&finite, [0; 4]).parameter_expectation(&finite.empty(), limits(), &mut work()),
        Err(Error::MissingParameter)
    ));
    assert!(matches!(
        function.parameter_expectation(
            &source.empty(),
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        function.parameter_expectation(
            &source.empty(),
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
    let mut small = limits();
    small.functions.steps = 0;
    assert!(matches!(
        function.parameter_expectation(&source.empty(), small, &mut work()),
        Err(Error::Capacity(Capacity::FunctionSteps))
    ));
    small = limits();
    small.functions.cells = 1;
    assert!(matches!(
        payoff(&source, [1, 2, 3, 4]).parameter_expectation(&source.empty(), small, &mut work()),
        Err(Error::Capacity(Capacity::FunctionCells))
    ));
}
