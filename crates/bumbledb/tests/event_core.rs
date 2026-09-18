use bumbledb::{
    Event, WorkContext,
    event::{BoolOp4, Error, Space, SpaceId},
};

#[test]
fn native_work_context_cancels_event_computation() {
    let work = WorkContext::new();
    let space = Space::new(SpaceId([8; 32]), 12, &work).unwrap();
    let retained: Event = space.coordinate(5, &work).unwrap();
    work.cancel();
    assert_eq!(
        retained.apply(BoolOp4::AND, &space.empty(), &work),
        Err(Error::Cancelled)
    );
    drop(space);
    assert_eq!(retained.count(&WorkContext::new()).unwrap(), 2048);
}
