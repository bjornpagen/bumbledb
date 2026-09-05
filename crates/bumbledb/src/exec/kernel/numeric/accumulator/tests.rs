use super::*;

fn accumulator(values: &[F64]) -> ExactF64Accumulator {
    let mut result = ExactF64Accumulator::default();
    for &value in values {
        result.push(value).expect("small fixture");
    }
    result
}

fn permute(values: &mut [F64], at: usize, expected: &ExactF64Accumulator, count: &mut usize) {
    if at == values.len() {
        assert_eq!(&accumulator(values), expected);
        *count += 1;
        return;
    }
    for index in at..values.len() {
        values.swap(at, index);
        permute(values, at + 1, expected, count);
        values.swap(at, index);
    }
}

#[test]
fn all_permutations_and_disjoint_partitions_have_one_exact_state() {
    let values = [
        0x4341_c379_37e0_8000,
        0x3ff0_0000_0000_0000,
        0xc341_c379_37e0_8000,
        1,
        0x8000_0000_0000_0001,
        0x4008_0000_0000_0000,
    ]
    .map(F64::from_bits);
    let expected = accumulator(&values);
    assert_eq!(expected.sum(), Some(F64::from_bits(0x4010_0000_0000_0000)));
    assert_eq!(expected.mean(), Some(F64::from_bits(0x3fe5_5555_5555_5555)));
    let mut count = 0;
    permute(&mut values.clone(), 0, &expected, &mut count);
    assert_eq!(count, 720);
    for mask in 0..1 << values.len() {
        let mut left = ExactF64Accumulator::default();
        let mut right = ExactF64Accumulator::default();
        for (index, &value) in values.iter().enumerate() {
            if mask & (1 << index) == 0 {
                left.push(value)
            } else {
                right.push(value)
            }
            .unwrap();
        }
        let mut opposite = right.clone();
        opposite.merge(&left).unwrap();
        left.merge(&right).unwrap();
        assert_eq!(left, expected);
        assert_eq!(opposite, expected);
    }
}

#[test]
fn merge_table_is_associative_commutative_but_not_idempotent() {
    let choices = [
        None,
        Some(F64::ZERO),
        Some(F64::from_bits(1)),
        Some(F64::from_bits(0xbff0_0000_0000_0000)),
        Some(F64::INFINITY),
        Some(F64::NEG_INFINITY),
        Some(F64::NAN),
    ];
    let states: Vec<_> = choices
        .into_iter()
        .map(|value| accumulator(&value.into_iter().collect::<Vec<_>>()))
        .collect();
    for a in &states {
        for b in &states {
            let mut ab = a.clone();
            ab.merge(b).unwrap();
            let mut ba = b.clone();
            ba.merge(a).unwrap();
            assert_eq!(ab, ba);
            for c in &states {
                let mut ab_c = ab.clone();
                ab_c.merge(c).unwrap();
                let mut bc = b.clone();
                bc.merge(c).unwrap();
                let mut a_bc = a.clone();
                a_bc.merge(&bc).unwrap();
                assert_eq!(ab_c, a_bc);
            }
        }
    }
    let one = accumulator(&[F64::from_bits(0x3ff0_0000_0000_0000)]);
    let mut doubled = one.clone();
    doubled.merge(&one).unwrap();
    assert_ne!(doubled, one);
    assert_eq!(doubled.sum(), Some(F64::from_bits(0x4000_0000_0000_0000)));
    assert_eq!(doubled.mean(), one.mean());
}

#[test]
fn u64_max_count_bound_and_error_are_numerical_state_independent() {
    for value in [
        F64::ZERO,
        F64::from_bits(0x7fef_ffff_ffff_ffff),
        F64::from_bits(0xffef_ffff_ffff_ffff),
        F64::INFINITY,
        F64::NEG_INFINITY,
        F64::NAN,
    ] {
        let mut power = accumulator(&[value]);
        let mut full = ExactF64Accumulator::default();
        for bit in 0..64 {
            full.merge(&power).unwrap();
            if bit != 63 {
                let copy = power.clone();
                power.merge(&copy).unwrap();
            }
        }
        let State::NonEmpty { count, total } = &full.state else {
            panic!("nonempty");
        };
        assert_eq!(count.get(), u64::MAX);
        let mut repeated = ExactF64Accumulator::default();
        repeated.push_repeated(value, u64::MAX).unwrap();
        assert_eq!(repeated, full, "scaled constant input equals disjoint exact merges");
        repeated.push_repeated(F64::NAN, 0).unwrap();
        assert_eq!(repeated, full, "zero multiplicity contributes no numerical state");
        if let Total::Finite(finite) = total {
            // 2162 is an upper bound independent of the storage's 2176 bits.
            assert_eq!(finite.limbs[33] >> 50, 0);
        }
        assert_eq!(full.mean(), Some(value));
        let previous = full.clone();
        for next in [F64::ZERO, F64::NAN, F64::INFINITY, F64::NEG_INFINITY] {
            assert_eq!(full.push(next), Err(FloatCardinalityOverflow));
            assert_eq!(full, previous);
        }
        full.merge(&ExactF64Accumulator::default()).unwrap();
        assert_eq!(full, previous);
        assert_eq!(full.merge(&full.clone()), Err(FloatCardinalityOverflow));
        assert_eq!(full, previous);
    }
}

#[test]
fn exact_mean_does_not_round_sum_first_or_lose_subnormal_ties() {
    let max = F64::from_bits(0x7fef_ffff_ffff_ffff);
    let two_max = accumulator(&[max, max]);
    assert_eq!(two_max.sum(), Some(F64::INFINITY));
    assert_eq!(two_max.mean(), Some(max));
    for (values, mean) in [
        ([1, 0], 0),
        ([3, 0], 2),
        ([5, 0], 2),
        ([7, 0], 4),
        ([0x8000_0000_0000_0001, 0], 0),
        ([0x8000_0000_0000_0003, 0], 0x8000_0000_0000_0002),
    ] {
        assert_eq!(
            accumulator(&values.map(F64::from_bits)).mean(),
            Some(F64::from_bits(mean))
        );
    }
}

#[test]
fn deterministic_random_merge_trees_match_unpartitioned_exact_states() {
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }
    let mut random = 0x17d6_eacd_9748_2021;
    for _ in 0..128 {
        let values: Vec<_> = (0..63).map(|_| F64::from_bits(next(&mut random))).collect();
        let expected = accumulator(&values);
        let mut states: Vec<_> = values.iter().map(|v| accumulator(&[*v])).collect();
        while states.len() > 1 {
            let index = usize::try_from(next(&mut random) % states.len() as u64).unwrap();
            let right = states.swap_remove(index);
            let index = usize::try_from(next(&mut random) % states.len() as u64).unwrap();
            states[index].merge(&right).unwrap();
        }
        assert_eq!(states[0], expected);
    }
}
