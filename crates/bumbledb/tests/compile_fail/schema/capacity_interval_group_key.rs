//! A capacity groups by scalar fields; an interval enters as a weight.
//@ error: the interval `Shift.span` is a capacity group key
//@ line: 11

bumbledb::schema! {
    pub Roster;
    relation Slot  { span: interval<u64>, seats: u64 }
    relation Shift { span: interval<u64> }

    Slot(span) -> Slot;
    Slot(span) <={0..seats} Shift(span);
}
