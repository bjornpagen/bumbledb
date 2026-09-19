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
