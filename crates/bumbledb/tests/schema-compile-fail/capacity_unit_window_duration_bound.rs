//! A unit (count) window against a `Duration` bound — dimension
//! mixing, refused (ruled 2026-07-24, C18): a count of facts bounded
//! by a span of time. The legal direction — a Duration weight under a
//! literal or Duration ceiling — is pinned in `tests/schema_macro.rs`.
//@ error: a unit (count) window against the Duration bound
//@ error: dimension error (ruled 2026-07-24, C18)

bumbledb::schema! {
    pub Rooms;

    relation Room    { id: u64, span: interval<u64> }
    relation Booking { room: u64, booked: interval<u64> }

    Room(id) -> Room;
    Room(id) <={0..Duration(span)} Booking(room);
}
