use super::*;
use bumbledb::event::{Space, SpaceId};

#[test]
fn cancelled_admission_wins_over_bad_value_or_certificate_without_partial_values() {
    let work = WorkContext::new();
    work.cancel();
    let bad_value = ValueInput::Event(b"BEVT\x01".to_vec());
    assert!(matches!(
        bad_value.admit(&work),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    assert!(matches!(
        ImportInput(b"BEDC".to_vec()).admit(&work),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    let query = query::Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Compute],
        rec: None,
        rules: vec![query::Rule {
            finds: vec![query::FindTerm::Event(query::EventExpr::Map {
                operation: bumbledb::event::MapOp::Image,
                map: ImportInput(b"BEDC".to_vec()),
                input: Box::new(query::EventExpr::Var(bumbledb::VarId(0))),
            })],
            atoms: vec![],
            negated: vec![],
            conditions: vec![],
        }],
    };
    assert!(matches!(
        query.admit(&work),
        Err(RuntimeError::Work(WorkError::Cancelled))
    ));
    let fresh = WorkContext::new();
    assert!(matches!(
        ValueInput::Event(b"BEVT\x01".to_vec()).admit(&fresh),
        Err(RuntimeError::Engine { kind: "event", .. })
    ));
}

#[test]
fn worker_admits_owned_values_and_keeps_import_certificates_after_input_release() {
    use bumbledb::event::{AdmittedDescriptor, CoordinateMap, Descriptor, DescriptorLimits};
    let work = WorkContext::new();
    let space = Space::new(SpaceId([198; 32]), 62, &work).unwrap();
    let event = space.coordinate(61, &work).unwrap();
    let bytes = event.to_bytes(&work).unwrap();
    let expected = bytes.clone();
    let map = CoordinateMap::identity(&space, &work).unwrap();
    let limits = DescriptorLimits::default();
    let descriptor = Descriptor::capture(&AdmittedDescriptor::Map(map), limits, &work)
        .unwrap()
        .to_bytes(limits, &work)
        .unwrap();
    let join = std::thread::spawn(move || {
        let work = WorkContext::new();
        (
            ValueInput::Event(bytes).admit(&work).unwrap(),
            ImportInput(descriptor).admit(&work).unwrap(),
        )
    });
    drop(event);
    drop(space);
    let (Value::Event(event), import) = join.join().unwrap() else {
        panic!("Event")
    };
    assert_eq!(event.to_bytes(&work).unwrap(), expected);
    assert_eq!(event.count(&work).unwrap(), 1 << 61);
    // The import is a real admitted native object, not a copied-byte certificate.
    let query = bumbledb::EventExpr::Map {
        operation: bumbledb::event::MapOp::Image,
        map: import,
        input: Box::new(bumbledb::EventExpr::Var(bumbledb::VarId(0))),
    };
    query.validate_shape().unwrap();
}

#[test]
fn complete_event_faults_move_their_payloads_and_operational_errors_have_no_partial_set() {
    use bumbledb::{EventFaultCategory, EventOperandFault, VarId};
    let source = Space::new(SpaceId([200; 32]), 2, &()).unwrap();
    let value = source.coordinate(0, &()).unwrap();
    let faults = vec![EventOperandFault {
        stage: Some(2),
        rule: 3,
        find: 4,
        operand: 5,
        variable: VarId(6),
        category: EventFaultCategory::SpaceMismatch,
        expected_space: source.full().to_bytes(&()).unwrap().into_boxed_slice(),
        offending_value: value.to_bytes(&()).unwrap().into_boxed_slice(),
    }]
    .into_boxed_slice();
    let address = faults[0].offending_value.as_ptr();
    let error = crate::runtime::session::owned_engine_error(bumbledb::Error::EventFaults(faults));
    drop(value);
    drop(source);
    let RuntimeError::EventFaults(faults) = error else {
        panic!("complete Event fault set")
    };
    assert_eq!(faults[0].offending_value.as_ptr(), address);
    assert_eq!(faults[0].stage, Some(2));
    assert_eq!(faults[0].variable, VarId(6));
    assert!(bumbledb::Event::from_bytes(&faults[0].offending_value, &()).is_ok());
    assert!(matches!(
        crate::runtime::session::owned_engine_error(EventError::Cancelled.into()),
        RuntimeError::Work(WorkError::Cancelled)
    ));
    assert!(matches!(
        crate::runtime::session::owned_engine_error(
            EventError::Capacity(Capacity::Diagnostics).into()
        ),
        RuntimeError::Engine { kind: "event", .. }
    ));
}
