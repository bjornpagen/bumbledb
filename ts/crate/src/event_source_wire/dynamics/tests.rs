use super::*;
use bumbledb::event::{ArithmeticLimits, CoordinateMap, FiniteFunction, Space, SpaceId};

#[test]
fn channel_import_and_closure_share_the_callers_arithmetic() {
    let control = WorkContext::new();
    let limits = SourceDescriptorLimits::default();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let base = Space::new(SpaceId([161; 32]), 0, &control).unwrap();
    let prior = FiniteFunction::constant(&base, ExactRational::one(), limits.functions, &mut work)
        .unwrap()
        .designate(limits.laws, &mut work)
        .unwrap();
    let extension = Space::new(SpaceId([162; 32]), 1, &control).unwrap();
    let parent = CoordinateMap::coordinates(&extension, &base, &[], &control).unwrap();
    let half = ExactRational::fraction("1", "2", &mut work).unwrap();
    let density = FiniteFunction::constant(&extension, half, limits.functions, &mut work).unwrap();
    let channel = FiniteKernel::new(&parent, &density, limits.functions, &mut work).unwrap();
    let bytes = SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Kernel(channel),
        limits,
        &mut work,
    )
    .unwrap()
    .to_bytes(limits, &control)
    .unwrap();
    let mut probe = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    kernel(&bytes, &mut probe).unwrap();
    let mut shared = ExactArithmetic::new(
        ArithmeticLimits {
            operations: probe.operations(),
            ..ArithmeticLimits::default()
        },
        &control,
    );
    let inputs = [bytes, prior.full().to_bytes(&control).unwrap()];
    assert!(matches!(
        execute(Op::KernelClose, &inputs, &control, &mut shared),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let output = execute(Op::KernelClose, &inputs, &control, &mut work).unwrap();
    let Output::EventSource(SourceOutput::Dynamics(Details::Extension { space, .. })) = output
    else {
        panic!()
    };
    let full = event(&space, limits, &mut work).unwrap();
    assert_eq!(full.mass(&mut work).unwrap(), ExactRational::one());
}

#[test]
fn revision_inspection_bounds_the_combined_receipt_not_only_each_leaf() {
    let control = WorkContext::new();
    let limits = SourceDescriptorLimits::default();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    let base = Space::new(SpaceId([163; 32]), 0, &control).unwrap();
    let prior = FiniteFunction::constant(&base, ExactRational::one(), limits.functions, &mut work)
        .unwrap()
        .designate(limits.laws, &mut work)
        .unwrap();
    // Every scalar/Event is small and every cell is a legitimate indexed cell.
    // The complete inspection nevertheless exceeds the output roster bound.
    let mut cells = vec![prior.empty(); 1400];
    cells[1399] = prior.full();
    let mut targets = vec![ExactRational::zero(); 1400];
    targets[1399] = ExactRational::one();
    let partition = EventPartition::on(&prior.full(), &cells, limits.partitions, &control).unwrap();
    let revision = prior
        .jeffrey(
            &partition,
            &targets,
            limits.functions,
            limits.laws,
            &mut work,
        )
        .unwrap();
    assert!(revision.revised().is_some());
    assert!(matches!(
        inspect(&revision, &control, &mut work),
        Err(Error::Capacity(Capacity::DescriptorItems))
    ));
}

#[test]
fn dynamics_dispatch_refuses_wrong_arities_and_cancelled_work_before_admission() {
    let control = WorkContext::new();
    control.cancel();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
    for (name, arity) in [
        ("kernel.new", 2),
        ("kernel.validate", 1),
        ("kernel.describe", 1),
        ("kernel.close", 2),
        ("kernel.factorsThrough", 2),
        ("revision.condition", 2),
        ("revision.likelihood", 2),
        ("revision.jeffrey", 3),
        ("revision.validate", 1),
        ("revision.inspect", 1),
    ] {
        let op = super::super::Op::parse(name).unwrap();
        assert!(op.arity(arity));
        assert!(!op.arity(arity + 1));
        assert!(!op.arity(0));
        assert!(matches!(
            super::super::execute(op, &[], 0, &control, &mut work),
            Err(Error::Cancelled)
        ));
    }
}
