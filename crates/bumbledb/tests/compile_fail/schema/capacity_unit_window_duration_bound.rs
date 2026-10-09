//! A count window cannot be bounded by a duration.
//@ error: mixes dimensions
//@ line: 12

bumbledb::schema! {
    pub Rooms;

    relation Room    { id: u64, span: interval<u64> }
    relation Booking { room: u64, booked: interval<u64> }

    Room(id) -> Room;
    Room(id) <={0..Duration(span)} Booking(room);
}
