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
        assert_eq!(full.count(), u64::MAX);
        let mut repeated = ExactF64Accumulator::default();
        repeated.push_repeated(value, u64::MAX).unwrap();
        assert_eq!(
            repeated, full,
            "scaled constant input equals disjoint exact merges"
        );
        repeated.push_repeated(F64::NAN, 0).unwrap();
        assert_eq!(
            repeated, full,
            "zero multiplicity contributes no numerical state"
        );
        if let Some(magnitude) = full.magnitude() {
            assert_eq!(magnitude.limbs[33] >> 50, 0, "totals stay below 2^2162");
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
#[cfg_attr(miri, ignore)]
fn deterministic_random_merge_trees_match_unpartitioned_exact_states() {
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }
    let mut random = 0x17d6_eacd_9748_2021;
    for _ in 0..crate::exec::sweep(128) {
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

mod oracle {
    //! The superaccumulator against the 34-limb oracle accumulator:
    //! identical exact totals, counts and sum/mean bits.
    use super::super::limbs::LimbAccumulator;
    use super::*;

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn value(&mut self) -> F64 {
            match self.next() % 8 {
                0 => F64::from_bits(self.next() % 64),
                1 => F64::from_bits(0x7fe0_0000_0000_0000 | (self.next() >> 12)),
                2 => F64::from_bits(0xffe0_0000_0000_0000 | (self.next() >> 12)),
                3 => F64::from(f64::from(u32::try_from(self.next() >> 40).unwrap()) - 8_000_000.0),
                4 if self.next().is_multiple_of(16) => {
                    [F64::INFINITY, F64::NEG_INFINITY, F64::NAN][(self.next() % 3) as usize]
                }
                _ => F64::from_bits(
                    (self.next() & !0x7ff0_0000_0000_0000) | ((self.next() % 0x7ff) << 52),
                ),
            }
        }
    }

    fn assert_same(got: &ExactF64Accumulator, want: &LimbAccumulator, context: &str) {
        assert_eq!(got.count(), want.count(), "count {context}");
        assert_eq!(
            got.magnitude().map(|m| (m.negative, m.limbs)),
            want.magnitude(),
            "exact total {context}"
        );
        assert_eq!(
            got.sum().map(F64::to_bits),
            want.sum().map(F64::to_bits),
            "sum {context}"
        );
        assert_eq!(
            got.mean().map(F64::to_bits),
            want.mean().map(F64::to_bits),
            "mean {context}"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn pushes_merges_repeats_and_key_batches_match_the_limb_accumulator() {
        let mut rng = Rng(0x05ee_d0ff_10a7);
        for round in 0..crate::exec::sweep(400) {
            let len = [0, 1, 2, 3, 7, 64, 1023, 1024, 1025, 3000][round % 10];
            let values: Vec<F64> = (0..len).map(|_| rng.value()).collect();
            let mut want = LimbAccumulator::default();
            for &value in &values {
                want.push(value).unwrap();
            }

            let mut pushed = ExactF64Accumulator::default();
            for &value in &values {
                pushed.push(value).unwrap();
            }
            assert_same(&pushed, &want, "push");

            let mut batched = ExactF64Accumulator::default();
            let cut = values.len() / 3;
            for part in [&values[..cut], &values[cut..]] {
                batched
                    .push_keys(part.iter().map(|v| v.to_order_key()))
                    .unwrap();
            }
            assert_same(&batched, &want, "push_keys");

            let mut parts: Vec<ExactF64Accumulator> = values
                .chunks(7)
                .map(|chunk| {
                    let mut part = ExactF64Accumulator::default();
                    for &value in chunk {
                        part.push(value).unwrap();
                    }
                    part
                })
                .collect();
            while parts.len() > 1 {
                let i = usize::try_from(rng.next() % parts.len() as u64).unwrap();
                let right = parts.swap_remove(i);
                let j = usize::try_from(rng.next() % parts.len() as u64).unwrap();
                parts[j].merge(&right).unwrap();
            }
            let merged = parts.pop().unwrap_or_default();
            assert_same(&merged, &want, "merge tree");

            let value = rng.value();
            let count = match round % 4 {
                0 => rng.next() % 5,
                1 => rng.next(),
                2 => u64::MAX - u64::try_from(len).unwrap(),
                _ => 1 << (rng.next() % 64),
            };
            let mut repeated = pushed.clone();
            let mut want_repeated = LimbAccumulator::default();
            want_repeated.merge(&want).unwrap();
            if want_repeated.push_repeated(value, count).is_ok() {
                repeated.push_repeated(value, count).unwrap();
                assert_same(&repeated, &want_repeated, "push_repeated");
            }
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn long_same_sign_runs_cross_many_carry_propagations() {
        for value in [
            F64::from_bits(0x7fef_ffff_ffff_ffff),
            F64::from_bits(0xffef_ffff_ffff_ffff),
            F64::from_bits(0x000f_ffff_ffff_ffff),
            F64::from_bits(0x4330_0000_0000_0001),
        ] {
            let mut got = ExactF64Accumulator::default();
            let mut want = LimbAccumulator::default();
            for _ in 0..10_000 {
                got.push(value).unwrap();
                want.push(value).unwrap();
            }
            assert_same(&got, &want, "run");
            let keys = vec![value.to_order_key(); 10_000];
            got.push_keys(keys.iter().copied()).unwrap();
            want.push_repeated(value, 10_000).unwrap();
            assert_same(&got, &want, "run batch");
        }
    }

    #[test]
    fn push_keys_counts_overflow_without_changing_the_state() {
        let mut full = ExactF64Accumulator::default();
        full.push_repeated(F64::from(1.0), u64::MAX).unwrap();
        let before = full.clone();
        assert_eq!(
            full.push_keys([F64::from(2.0).to_order_key()].into_iter()),
            Err(FloatCardinalityOverflow)
        );
        assert_eq!(full, before);
        assert_eq!(full.push_keys(std::iter::empty()), Ok(()));
    }
}
