use super::carrier::*;
use super::observation::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

fn ratio(a: i64, b: i64) -> BigRational {
    BigRational::new(a.into(), b.into())
}

// Independent sequential Pólya urn oracle, rather than the rising-factorial table.
fn urn_world(plan: &CountPlan, priors: &[(u32, u32)], w: usize) -> BigRational {
    let mut states = priors.to_vec();
    let mut probability = BigRational::one();
    for (bit, &group) in plan.group_of.iter().enumerate() {
        let (a, b) = &mut states[group];
        let heads = w >> bit & 1 != 0;
        probability *= ratio(if heads { *a } else { *b } as i64, (*a + *b) as i64);
        if heads {
            *a += 1;
        } else {
            *b += 1;
        }
    }
    probability
}

pub fn verify<C: Carrier>() {
    for nbits in [3, 8, 10] {
        let n = 1usize << nbits;
        for restricted in [false, true] {
            let support = bits(n, |w| {
                !restricted || (w % 7 != 0 && (w ^ (w >> 3)) & 1 != 0)
            });
            for order in [(0..nbits).rev().collect(), (0..nbits).collect()] {
                let mut c = C::with_order(&support, n, order);
                let bank: Vec<_> = (0..6)
                    .map(|seed| bits(n, |w| mix((w + seed * 151) as u64) % 5 < 3))
                    .collect();
                let ids: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
                for group_count in [1, 2, 3] {
                    let mut plan =
                        CountPlan::new((0..nbits as usize).map(|i| i % group_count).collect());
                    c.prepare_spectrum(&mut plan);
                    let full = c.spectrum(c.full(), &plan);
                    assert_eq!(
                        full,
                        plan.oracle(&support, n),
                        "{} support spectrum",
                        C::NAME
                    );
                    let empty = c.spectrum(c.empty(), &plan);
                    assert!(empty.iter().all(|&v| v == 0));
                    for (i, &a) in ids.iter().enumerate() {
                        let actual = c.spectrum(a, &plan);
                        let expected: Vec<_> =
                            bank[i].iter().zip(&support).map(|(a, s)| a & s).collect();
                        assert_eq!(actual, plan.oracle(&expected, n), "{} spectrum", C::NAME);
                        assert_eq!(actual.iter().sum::<u64>(), c.count(a));
                        let neg = c.not(a);
                        assert_eq!(c.spectrum(neg, &plan), plan.subtract(full.clone(), &actual));
                        let transformed = c.exists(a, 5);
                        assert_eq!(
                            c.spectrum(transformed, &plan),
                            plan.oracle(&c.export(transformed), n)
                        );
                    }
                    if nbits == 3 {
                        let priors: Vec<_> = (0..group_count).map(|g| (2 + g as u32, 3)).collect();
                        let law = ExactLaw::beta(&plan, &priors);
                        for &a in &ids {
                            let b = c.export(a);
                            let expected = (0..n)
                                .filter(|&w| at(&b, w))
                                .fold(BigRational::zero(), |v, w| v + urn_world(&plan, &priors, w));
                            assert_eq!(law.probability(&c.spectrum(a, &plan)), expected);
                        }
                    }
                }
            }
        }
    }
    // Shared draws versus distinct parameter allocations; actual event identity unchanged.
    let mut c = C::new(&[15], 4);
    let a = c.import(&[10]);
    let b = c.import(&[12]);
    let both = c.op(8, a, b);
    for (groups, expected_joint, expected_conditional) in [
        (vec![0, 0], ratio(1, 5), ratio(1, 2)),
        (vec![0, 1], ratio(4, 25), ratio(2, 5)),
    ] {
        let mut plan = CountPlan::new(groups);
        c.prepare_spectrum(&mut plan);
        let law = ExactLaw::beta(&plan, &vec![(2, 3); plan.degrees.len()]);
        let ha = c.spectrum(a, &plan);
        let hb = c.spectrum(b, &plan);
        let hab = c.spectrum(both, &plan);
        assert_eq!(law.probability(&ha), ratio(2, 5));
        assert_eq!(law.probability(&hb), ratio(2, 5));
        assert_eq!(law.probability(&hab), expected_joint);
        assert_eq!(law.conditional(&hab, &ha), Some(expected_conditional));
        assert_eq!(
            law.probability(&c.spectrum(c.full(), &plan)),
            BigRational::one()
        );
        let zero = ExactLaw::point(&plan, &vec![(0, 1); plan.degrees.len()]);
        assert_eq!(zero.probability(&ha), BigRational::zero());
        assert_eq!(zero.conditional(&hab, &ha), None);
        assert_ne!(a, c.empty());
    }
}

pub fn high<C: Carrier>(mut c: C, all_heads: Id, one_head: Id) {
    let mut plan = CountPlan::new(vec![0; 40]);
    c.prepare_spectrum(&mut plan);
    let law = ExactLaw::beta(&plan, &[(1, 1)]);
    let one = c.spectrum(one_head, &plan);
    let all = c.spectrum(all_heads, &plan);
    assert_eq!(
        law.probability(&one),
        ratio(1, 2),
        "{} discardability",
        C::NAME
    );
    assert_eq!(
        law.probability(&all),
        ratio(1, 41),
        "{} shared 40 draws",
        C::NAME
    );
    let not_all = c.not(all_heads);
    assert_eq!(law.probability(&c.spectrum(not_all, &plan)), ratio(40, 41));
    let fixed = ExactLaw::point(&plan, &[(1, 2)]);
    assert_eq!(
        fixed.probability(&all),
        BigRational::new(BigInt::one(), BigInt::from(1u64 << 40))
    );
}
