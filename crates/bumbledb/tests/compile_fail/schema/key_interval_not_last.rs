//! A pointwise key's interval is its last position.
//@ error: the interval `Booking.span` must be the key's last position
//@ line: 9

bumbledb::schema! {
    pub Rooms;
    relation Booking { room: u64, span: interval<u64> }

    Booking(span, room) -> Booking;
}
