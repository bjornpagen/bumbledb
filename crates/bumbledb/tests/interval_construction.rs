//! Safe const construction and the macro's constant-accessor path must
//! preserve the same nonempty interval invariant as runtime construction.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Interval, Theory as _};

bumbledb::schema! {
    pub ConstantIntervals;

    closed relation Window as WindowId {
        unsigned: interval<u64>,
        signed: interval<i64>,
        fixed: interval<u64, 5>,
    } = {
        First { unsigned: 0..10, signed: -7..8, fixed: 0..5 },
        Second { unsigned: 10..20, signed: -100..-1, fixed: 5..10 },
    };
}

#[test]
fn macro_ground_axioms_use_the_checked_const_constructor() {
    const UNSIGNED: Interval<u64> = Window::First.unsigned();
    const SIGNED: Interval<i64> = Window::Second.signed();
    const FIXED: Interval<u64> = Window::First.fixed();

    ConstantIntervals
        .descriptor()
        .validate()
        .expect("the ground axioms have valid bounds");
    assert_eq!(UNSIGNED, Interval::<u64>::new(0, 10).unwrap());
    assert_eq!(SIGNED, Interval::<i64>::new(-100, -1).unwrap());
    assert_eq!(FIXED, Interval::<u64>::fixed(0, 5).unwrap());
}

#[test]
fn downstream_const_calls_refuse_invalid_bounds() {
    const EMPTY_U64: Option<Interval<u64>> = Interval::<u64>::const_new(0, 0);
    const REVERSED_U64: Option<Interval<u64>> = Interval::<u64>::const_new(u64::MAX, 0);
    const EMPTY_I64: Option<Interval<i64>> = Interval::<i64>::const_new(i64::MIN, i64::MIN);
    const REVERSED_I64: Option<Interval<i64>> = Interval::<i64>::const_new(i64::MAX, i64::MIN);

    assert!(EMPTY_U64.is_none());
    assert!(REVERSED_U64.is_none());
    assert!(EMPTY_I64.is_none());
    assert!(REVERSED_I64.is_none());
}
