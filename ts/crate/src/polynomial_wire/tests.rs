use super::*;

#[test]
fn polynomial_operands_and_results_share_arithmetic_and_keep_cancellation() {
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let value = ExactPolynomial::parameter(ParameterId([3; 32]));
    let bytes = value
        .to_bytes(PolynomialLimits::default(), &mut work)
        .unwrap();
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    ExactPolynomial::from_bytes(&bytes, PolynomialLimits::default(), &mut probe).unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    assert!(matches!(
        execute(
            Op::Multiply,
            &[bytes.clone(), bytes.clone()],
            0,
            &control,
            &mut bounded
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let Output::Bytes(result) = execute(
        Op::Pow,
        std::slice::from_ref(&bytes),
        2,
        &control,
        &mut work,
    )
    .unwrap() else {
        panic!()
    };
    let square =
        ExactPolynomial::from_bytes(&result.bytes, PolynomialLimits::default(), &mut work).unwrap();
    assert_eq!(
        square.terms()[0].powers.as_ref(),
        &[(ParameterId([3; 32]), 2)]
    );
    control.cancel();
    assert!(matches!(
        execute(Op::Describe, &[bytes], 0, &control, &mut work),
        Err(Error::Cancelled)
    ));
}

#[test]
fn polynomial_metadata_and_unused_operands_cannot_escape_admission() {
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let zero = ExactRational::zero().to_bytes(&mut work).unwrap();
    assert!(matches!(
        execute(
            Op::New,
            &[zero.clone(), vec![0; 35]],
            0,
            &control,
            &mut work
        ),
        Err(Error::InvalidPolynomial)
    ));
    assert!(matches!(
        execute(Op::New, &[zero, vec![0; 36 * 257]], 0, &control, &mut work),
        Err(Error::Capacity(Capacity::PolynomialFactors))
    ));
    let zero = ExactPolynomial::zero()
        .to_bytes(PolynomialLimits::default(), &mut work)
        .unwrap();
    // The unused replacement must be decoded even when the source is zero.
    assert!(
        execute(
            Op::Substitute,
            &[zero.clone(), vec![0; 32], b"BEPL\x01".to_vec()],
            0,
            &control,
            &mut work
        )
        .is_err()
    );
    assert!(matches!(
        execute(Op::New, &[zero], 0, &control, &mut work),
        Err(Error::InvalidEncoding)
    ));
}
