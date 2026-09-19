use super::*;
use bumbledb::event::{DensityPiece, LawLimits, SpaceId};

#[test]
fn measured_operands_share_one_budget_and_cancellation_precedes_admission() {
    let control = WorkContext::new();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let raw = Space::new(SpaceId([121; 32]), 1, &control).unwrap();
    let measured = raw
        .with_density(
            &[DensityPiece {
                region: raw.full(),
                density: ExactRational::fraction("1", "2", &mut work).unwrap(),
            }],
            LawLimits::default(),
            &mut work,
        )
        .unwrap();
    let bytes = measured.full().to_bytes(&control).unwrap();
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    event(&bytes, SourceDescriptorLimits::default(), &mut probe).unwrap();
    let mut shared = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    // Each input fits separately; both inputs and contraction must share work.
    assert!(matches!(
        execute(
            Op::Probability,
            &[bytes.clone(), bytes.clone()],
            0,
            &control,
            &mut shared
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let result = execute(
        Op::Probability,
        &[bytes.clone(), bytes.clone()],
        0,
        &control,
        &mut work,
    )
    .unwrap();
    let Output::EventSource(SourceOutput::Observation {
        value: Some(value), ..
    }) = result
    else {
        panic!()
    };
    assert_eq!(
        ExactRational::from_bytes(&value, &mut work).unwrap(),
        ExactRational::one()
    );
    control.cancel();
    assert!(matches!(
        execute(Op::Mass, &[bytes], 0, &control, &mut work),
        Err(Error::Cancelled)
    ));
}
