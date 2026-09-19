use super::*;
use bumbledb::event::{CoordinateMap, Event, Space, SpaceId};
use bumbledb::work::WorkError;

#[test]
fn descriptor_operations_reconstruct_and_own_their_results() {
    let work = WorkContext::new();
    let space = Space::new(SpaceId([73; 32]), 1, &work).unwrap();
    let map = CoordinateMap::coordinates(&space, &space, &[0], &work).unwrap();
    let original = encoded(&AdmittedDescriptor::Map(map), &work).unwrap();
    let Output::EventDescriptor(DescriptorOutput::Description(description)) =
        execute(Op::Describe, Input::Bytes(original.clone()), &work).unwrap()
    else {
        panic!()
    };
    let Output::Bytes(bytes) = execute(Op::Admit, Input::Data(description), &work).unwrap() else {
        panic!()
    };
    assert_eq!(bytes.bytes, original);
    assert!(matches!(
        execute(Op::Inspect, Input::Bytes(bytes.bytes), &work),
        Ok(Output::EventDescriptor(DescriptorOutput::Inspection(_)))
    ));
    assert!(matches!(
        execute(Op::Inspect, Input::Bytes(vec![0]), &work),
        Err(RuntimeError::Engine { kind: "event", .. })
    ));
    let foreign = Space::new(SpaceId([74; 32]), 1, &work).unwrap();
    let mut invalid =
        Descriptor::from_bytes(&original, DescriptorLimits::default(), &work).unwrap();
    let Descriptor::Map(ref mut map) = invalid else {
        panic!()
    };
    map.readouts[0] = foreign
        .coordinate(0, &work)
        .unwrap()
        .to_bytes(&work)
        .unwrap();
    assert!(matches!(
        execute(Op::Admit, Input::Data(invalid), &work),
        Err(RuntimeError::Engine { kind: "event", .. })
    ));
    // A proper region is not an implicit request to replace its source by full.
    let mut invalid =
        Descriptor::from_bytes(&original, DescriptorLimits::default(), &work).unwrap();
    let Descriptor::Map(ref mut map) = invalid else {
        panic!()
    };
    map.source = space.coordinate(0, &work).unwrap().to_bytes(&work).unwrap();
    assert!(matches!(
        execute(Op::Admit, Input::Data(invalid), &work),
        Err(RuntimeError::Engine { kind: "event", .. })
    ));
    drop(space);
    let data = Descriptor::from_bytes(&original, DescriptorLimits::default(), &work).unwrap();
    let Descriptor::Map(map) = data else { panic!() };
    assert!(Event::from_bytes(&map.source, &work).unwrap().is_full());
    work.cancel();
    for op in [Op::Admit, Op::Describe, Op::Inspect] {
        assert!(matches!(
            execute(op, Input::Bytes(original.clone()), &work),
            Err(RuntimeError::Work(WorkError::Cancelled))
        ));
    }
}
