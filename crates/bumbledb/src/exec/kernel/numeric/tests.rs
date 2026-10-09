use super::*;

#[test]
fn canonical_zero_and_nan_and_no_reassociation_or_fusion() {
    let one = F64::from(1.0);
    assert_eq!(
        F64Math::divide(one, F64::ZERO.negated()).unwrap(),
        F64::INFINITY,
        "negative zero canonicalizes before it can flip the infinity"
    );
    assert_eq!(
        F64Math::subtract(F64::INFINITY, F64::INFINITY).unwrap(),
        F64::NAN
    );
    assert_eq!(F64Math::divide(F64::ZERO, F64::ZERO).unwrap(), F64::NAN);
    assert_eq!(
        F64Math::multiply(F64::INFINITY, F64::ZERO).unwrap(),
        F64::NAN
    );
    assert_eq!(
        F64Math::multiply(F64::from(-1.0), F64::ZERO).unwrap(),
        F64::ZERO
    );

    // 2^53 + 1 rounds to 2^53 but 1 - 2^53 is exact: each node rounds alone.
    let large = F64::from(9_007_199_254_740_992.0);
    let sequential = F64Math::add(F64Math::add(large, one).unwrap(), large.negated()).unwrap();
    let regrouped = F64Math::add(large, F64Math::add(one, large.negated()).unwrap()).unwrap();
    assert_eq!(sequential, F64::ZERO);
    assert_eq!(regrouped, one);
    assert_eq!(
        F64Math::sum([large, one, large.negated()]).unwrap(),
        Some(one)
    );

    // (1 + 2^-52)(1 - 2^-52) rounds to 1; a fused multiply-add would give -2^-104.
    let product = F64Math::multiply(
        F64::from_bits(0x3ff0_0000_0000_0001),
        F64::from_bits(0x3fef_ffff_ffff_fffe),
    )
    .unwrap();
    assert_eq!(F64Math::subtract(product, one).unwrap(), F64::ZERO);
}

#[test]
fn gradual_underflow_and_ties_to_even() {
    let one = F64::from(1.0);
    assert_eq!(
        F64Math::multiply(F64::MIN_POSITIVE_SUBNORMAL, one)
            .unwrap()
            .to_bits(),
        1
    );
    assert_eq!(
        F64Math::divide(F64::from_bits(3), F64::from(2.0))
            .unwrap()
            .to_bits(),
        2
    );
    assert_eq!(
        F64Math::add(one, F64::from_bits(0x3ca0_0000_0000_0000)).unwrap(),
        one
    );
}
