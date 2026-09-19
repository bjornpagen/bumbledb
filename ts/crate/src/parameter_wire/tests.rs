use super::*;

fn run(
    name: &str,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    execute(Op::parse(name).unwrap(), inputs, 0, control, work)
}
fn take_bytes(output: Output) -> Vec<u8> {
    let Output::Bytes(value) = output else {
        panic!("bytes expected")
    };
    value.bytes
}

#[test]
fn nested_parameter_transport_spends_one_arithmetic_budget_and_keeps_holes() {
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let id = ParameterId([9; 32]);
    let ambient = ParameterRegion::full(id)
        .to_bytes(limits().parameters.parameters, &mut work)
        .unwrap();
    let p = ExactPolynomial::parameter(id)
        .to_bytes(limits().parameters.parameters.region.polynomial, &mut work)
        .unwrap();
    let bytes = take_bytes(
        run(
            "function.ratio",
            &[ambient, p.clone(), p],
            &control,
            &mut work,
        )
        .unwrap(),
    );
    let zero = ExactRational::zero().to_bytes(&mut work).unwrap();
    assert!(matches!(
        run("function.at", &[bytes.clone(), zero], &control, &mut work),
        Ok(Output::Parameter(ParameterOutput::Value(None)))
    ));

    // Budget equal to one whole nested BESC import cannot pay for two operands,
    // the arithmetic and output together. No child decoder resets it.
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    bumbledb::event::SourceDescriptor::import(&bytes, limits(), &mut probe).unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    assert!(matches!(
        run(
            "function.multiply",
            &[bytes.clone(), bytes.clone()],
            &control,
            &mut bounded
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    control.cancel();
    assert!(matches!(
        run("function.describe", &[bytes], &control, &mut work),
        Err(Error::Cancelled)
    ));
}

#[test]
fn parameter_roles_and_unused_pieces_are_checked_before_empty_shortcuts() {
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let id = ParameterId([1; 32]);
    let full = ParameterRegion::full(id)
        .to_bytes(limits().parameters.parameters, &mut work)
        .unwrap();
    let empty = ParameterRegion::empty(id)
        .to_bytes(limits().parameters.parameters, &mut work)
        .unwrap();
    assert!(
        run(
            "region.domain",
            std::slice::from_ref(&empty),
            &control,
            &mut work
        )
        .is_err()
    );
    let foreign = ParameterRegion::empty(ParameterId([2; 32]))
        .to_bytes(limits().parameters.parameters, &mut work)
        .unwrap();
    assert!(
        run(
            "region.apply",
            &[empty.clone(), foreign],
            &control,
            &mut work
        )
        .is_err()
    );
    let zero = ExactPolynomial::zero()
        .to_bytes(limits().parameters.parameters.region.polynomial, &mut work)
        .unwrap();
    assert!(
        run(
            "function.pieces",
            &[full.clone(), zero.clone(), b"BEPL\x01".to_vec(), empty],
            &control,
            &mut work
        )
        .is_err()
    );
    assert!(
        run(
            "function.pieces",
            &[full.clone(), zero],
            &control,
            &mut work
        )
        .is_err()
    );
    // An unrelated but admitted BESC v1 finite function must not acquire the
    // parameter-function role from its shared numeric tag zero.
    let space =
        bumbledb::event::Space::new(bumbledb::event::SpaceId([6; 32]), 0, &control).unwrap();
    let finite = bumbledb::event::FiniteFunction::constant(
        &space,
        ExactRational::zero(),
        bumbledb::event::FunctionLimits::default(),
        &mut work,
    )
    .unwrap();
    let descriptor = bumbledb::event::SourceDescriptor::capture(
        &bumbledb::event::AdmittedSourceDescriptor::Function(finite),
        limits(),
        &mut work,
    )
    .unwrap()
    .to_bytes(limits(), &control)
    .unwrap();
    assert!(matches!(
        run("function.validate", &[descriptor], &control, &mut work),
        Err(Error::RoleMismatch)
    ));
    assert!(matches!(
        execute(
            Op::parse("region.sign").unwrap(),
            &[vec![1; 32], b"BEPL\x01".to_vec()],
            8,
            &control,
            &mut work
        ),
        Err(Error::InvalidEncoding)
    ));
}

#[test]
fn parameter_structured_payloads_refuse_combined_output_overflow() {
    let mut budget = output::Budget::default();
    budget.blob(vec![0; MAX_EVENT_BYTES - 1]).unwrap();
    budget.blob(vec![1]).unwrap();
    assert!(matches!(
        budget.blob(vec![2]),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
}

fn prior_inputs(control: &WorkContext, work: &mut ExactArithmetic<'_>) -> [Vec<u8>; 3] {
    use bumbledb::event::{BoolOp4, FamilyFunction, GuardedRationalFunction, Space, SpaceId};
    let name = ParameterId([77; 32]);
    let p = ExactPolynomial::parameter(name);
    let tail = ExactPolynomial::one()
        .sub(&p, limits().parameters.parameters.region.polynomial, work)
        .unwrap();
    let low = ParameterRegion::from_polynomial(
        name,
        &p,
        PolynomialSigns::NON_NEGATIVE,
        limits().parameters.parameters.region,
        work,
    )
    .unwrap();
    let high = ParameterRegion::from_polynomial(
        name,
        &tail,
        PolynomialSigns::NON_NEGATIVE,
        limits().parameters.parameters.region,
        work,
    )
    .unwrap();
    let domain = ParameterDomain::new(
        low.apply(
            BoolOp4::AND,
            &high,
            limits().parameters.parameters.region,
            work,
        )
        .unwrap(),
    )
    .unwrap();
    let source = Space::new(SpaceId([78; 32]), 0, control)
        .unwrap()
        .with_parameters(domain.clone(), &[], limits().parameters, work)
        .unwrap();
    let one = GuardedRationalFunction::new(
        domain,
        ExactPolynomial::one(),
        ExactPolynomial::one(),
        limits().parameters.parameters.region,
        work,
    )
    .unwrap();
    let source = FamilyFunction::constant(&source, one, limits().parameters, work)
        .unwrap()
        .designate(limits().parameters, work)
        .unwrap();
    let full = source.full().to_bytes(control).unwrap();
    let shape = ExactRational::one().to_bytes(work).unwrap();
    let beta = take_bytes(
        run(
            "prior.new",
            &[full.clone(), shape.clone(), shape],
            control,
            work,
        )
        .unwrap(),
    );
    [beta, full.clone(), full]
}

#[test]
fn prior_observations_share_admission_contraction_and_output_work() {
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let inputs = prior_inputs(&control, &mut work);
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    bumbledb::event::SourceDescriptor::import(&inputs[0], limits(), &mut probe).unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    assert!(matches!(
        run("prior.probability", &inputs, &control, &mut bounded),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let result = run("prior.probability", &inputs, &control, &mut work).unwrap();
    let Output::Parameter(ParameterOutput::Prior(details)) = result else {
        panic!("prior details")
    };
    let prior::Details::Observation {
        original, value, ..
    } = *details
    else {
        panic!("prior observation")
    };
    assert_eq!(
        rational(value.as_ref().unwrap(), &mut work).unwrap(),
        ExactRational::one()
    );
    assert!(matches!(
        *original,
        ParameterOutput::Observation {
            kind: "probability",
            ..
        }
    ));
    assert!(!Op::parse("prior.probability").unwrap().valid(2, 0));
    assert!(!Op::parse("prior.new").unwrap().valid(3, 1));
    assert!(Op::parse("prior.implicit").is_none());
    control.cancel();
    assert!(matches!(
        run("prior.probability", &inputs, &control, &mut work),
        Err(Error::Cancelled)
    ));
}

#[test]
fn family_observations_share_nested_admission_work_and_keep_owned_inputs() {
    use bumbledb::event::{
        FamilyFunction, GuardedRationalFunction, ParameterDomain, Space, SpaceId,
    };
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let domain = ParameterDomain::new(ParameterRegion::full(ParameterId([27; 32]))).unwrap();
    let source = Space::new(SpaceId([28; 32]), 1, &control)
        .unwrap()
        .with_parameters(domain.clone(), &[], limits().parameters, &mut work)
        .unwrap();
    let half = GuardedRationalFunction::new(
        domain,
        ExactPolynomial::constant(ExactRational::fraction("1", "2", &mut work).unwrap()),
        ExactPolynomial::one(),
        limits().parameters.parameters.region,
        &mut work,
    )
    .unwrap();
    let source = FamilyFunction::constant(&source, half, limits().parameters, &mut work)
        .unwrap()
        .designate(limits().parameters, &mut work)
        .unwrap();
    let event = source
        .coordinate(0, &control)
        .unwrap()
        .to_bytes(&control)
        .unwrap();
    let given = source.full().to_bytes(&control).unwrap();
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    super::source::event(&event, &mut probe).unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    assert!(matches!(
        run(
            "source.probability",
            &[event.clone(), given.clone()],
            &control,
            &mut bounded
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let Output::Parameter(ParameterOutput::Observation {
        kind,
        input,
        given: retained,
        numerator,
        mass,
        value,
        defined,
    }) = run(
        "source.probability",
        &[event.clone(), given.clone()],
        &control,
        &mut work,
    )
    .unwrap()
    else {
        panic!("observation expected")
    };
    assert_eq!(kind, "probability");
    assert_eq!(input, event);
    assert_eq!(retained, given);
    assert!(region(&defined, &mut work).unwrap().is_full());
    drop(source);
    for (encoded, expected) in [(numerator, "1/2"), (mass, "1"), (value, "1/2")] {
        let value = function::function(&encoded, &mut work)
            .unwrap()
            .value_at(
                &ExactRational::zero(),
                limits().parameters.parameters.region,
                limits().parameters.functions,
                &mut work,
            )
            .unwrap()
            .unwrap();
        assert_eq!(value.to_string(), expected);
    }
    control.cancel();
    assert!(matches!(
        run("source.probability", &[event, given], &control, &mut work),
        Err(Error::Cancelled)
    ));
}

#[test]
fn family_revision_inspection_replays_under_one_budget_and_retains_impossible_identity() {
    use bumbledb::event::{FamilyFunction, GuardedRationalFunction, Space, SpaceId};
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let domain = ParameterDomain::new(ParameterRegion::full(ParameterId([43; 32]))).unwrap();
    let source = Space::new(SpaceId([44; 32]), 1, &control)
        .unwrap()
        .with_parameters(domain.clone(), &[], limits().parameters, &mut work)
        .unwrap();
    let density = GuardedRationalFunction::new(
        domain,
        ExactPolynomial::constant(ExactRational::fraction("1", "2", &mut work).unwrap()),
        ExactPolynomial::one(),
        limits().parameters.parameters.region,
        &mut work,
    )
    .unwrap();
    let prior = FamilyFunction::constant(&source, density, limits().parameters, &mut work)
        .unwrap()
        .designate(limits().parameters, &mut work)
        .unwrap();
    let inputs = vec![
        vec![45; 32],
        prior.full().to_bytes(&control).unwrap(),
        prior.empty().to_bytes(&control).unwrap(),
    ];
    let Output::Bytes(value) = run("revision.condition", &inputs, &control, &mut work).unwrap()
    else {
        panic!("revision");
    };
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    bumbledb::event::SourceDescriptor::import(&value.bytes, limits(), &mut probe).unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    assert!(matches!(
        run(
            "revision.inspect",
            std::slice::from_ref(&value.bytes),
            &control,
            &mut bounded
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    drop(prior);
    let Output::Parameter(ParameterOutput::Dynamics(dynamics::Details::Revision {
        identity,
        prior,
        defined,
        outcome,
        ..
    })) = run(
        "revision.inspect",
        std::slice::from_ref(&value.bytes),
        &control,
        &mut work,
    )
    .unwrap()
    else {
        panic!("inspection");
    };
    assert_eq!(identity, [45; 32]);
    assert_eq!(prior, inputs[1]);
    assert!(outcome.is_none());
    assert!(region(&defined, &mut work).unwrap().is_empty());
    assert!(matches!(
        run(
            "kernel.validate",
            std::slice::from_ref(&value.bytes),
            &control,
            &mut work
        ),
        Err(Error::RoleMismatch)
    ));
    assert!(matches!(
        run(
            "restriction.validate",
            std::slice::from_ref(&value.bytes),
            &control,
            &mut work
        ),
        Err(Error::RoleMismatch)
    ));
    control.cancel();
    assert!(matches!(
        run("revision.inspect", &[value.bytes], &control, &mut work),
        Err(Error::Cancelled)
    ));
}
