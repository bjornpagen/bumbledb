//@ error: evaluation panicked: nonempty interval ground axiom

const INVALID: bumbledb::Interval<i64> =
    bumbledb::Interval::<i64>::const_new(9, -1).expect("nonempty interval ground axiom");

fn main() {
    let _ = INVALID;
}
