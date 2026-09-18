//! Information readouts remain ordinary Events in the original legal scope.
use super::carrier::*;
use super::legal::{Domain, Domains};

pub fn mask(width: u32, family: &str, faces: &str) -> u64 {
    assert!(width >= 2);
    let local = match family {
        "suffix-1" => 1,
        "suffix-half" => (1u64 << (width / 2)) - 1,
        "whole" => (1u64 << width) - 1,
        "non-suffix" => 1u64 << (width - 1),
        _ => panic!("unknown readout family"),
    };
    match faces {
        "x" => local,
        "y" => local << width,
        "xy" => local | (local << width),
        _ => panic!("unknown readout faces"),
    }
}

pub fn construct<C: RegionOps>(c: &mut Memo<C>, event: Id, hidden: u64) -> [Id; 4] {
    let possible = c.inner.exists(event, hidden);
    let negated = c.inner.not(event);
    let counterexample = c.inner.exists(negated, hidden);
    let guaranteed = c.inner.not(counterexample);
    let ambiguous = c.op(8, possible, counterexample);
    [event, possible, guaranteed, ambiguous]
}

/// Independent oracle: partition ORIGINAL legal worlds by visible coordinates.
/// No raw decoder aliases and no Boolean-axis reduction are used here.
pub fn reference(support: &[u64], event: &[u64], n: usize, hidden: u64) -> [Vec<u64>; 4] {
    assert!(n.is_power_of_two());
    assert_eq!(hidden >> n.trailing_zeros(), 0);
    let mut positive = vec![false; n];
    let mut negative = vec![false; n];
    for w in 0..n {
        if at(support, w) {
            let key = (w as u64 & !hidden) as usize;
            if at(event, w) {
                positive[key] = true;
            } else {
                negative[key] = true;
            }
        }
    }
    let original = bits(n, |w| at(support, w) && at(event, w));
    let possible = bits(n, |w| {
        at(support, w) && positive[(w as u64 & !hidden) as usize]
    });
    let guaranteed = bits(n, |w| {
        at(support, w) && !negative[(w as u64 & !hidden) as usize]
    });
    let ambiguous = bits(n, |w| {
        let key = (w as u64 & !hidden) as usize;
        at(support, w) && positive[key] && negative[key]
    });
    [original, possible, guaranteed, ambiguous]
}

pub fn verify<C: Carrier>(c: &C, out: &[Id; 4], expected: &[Vec<u64>; 4], hidden: u64) {
    verify_batch(c, out, std::slice::from_ref(expected), hidden);
}

pub fn verify_batch<C: Carrier>(c: &C, out: &[Id], expected: &[[Vec<u64>; 4]], hidden: u64) {
    assert_eq!(out.len(), 4 * expected.len());
    let before = (c.nodes(), c.bytes(), c.memory_stats());
    for (&id, words) in out.iter().zip(expected.iter().flatten()) {
        assert_eq!(c.export(id), *words, "{} readout", C::NAME);
        assert_eq!(
            c.count(id),
            words.iter().map(|w| w.count_ones() as u64).sum()
        );
    }
    assert_eq!((c.nodes(), c.bytes(), c.memory_stats()), before);
    // Validation may allocate. Keep all of it outside the actual retained owner.
    let mut proof = c.clone();
    for (&id, words) in out.iter().zip(expected.iter().flatten()) {
        assert_eq!(proof.import(words), id, "canonical readout reimport");
    }
    for group in out.chunks_exact(4) {
        let [original, possible, guaranteed, ambiguous]: [Id; 4] = group.try_into().unwrap();
        assert_eq!(proof.op(4, guaranteed, original), proof.empty());
        assert_eq!(proof.op(4, original, possible), proof.empty());
        for id in [possible, guaranteed, ambiguous] {
            assert_eq!(proof.exists(id, hidden), id, "readout membership FD");
        }
        assert_eq!(original == possible, original == guaranteed);
    }
}

fn finite<C: Carrier>() {
    let domains = [
        Domains::new(3, 0, vec![Domain::Below(8)]).unwrap(),
        Domains::new(3, 0, vec![Domain::Below(5)]).unwrap(),
        Domains::new(3, 0, vec![Domain::Values(vec![0, 2, 3, 6])]).unwrap(),
        Domains::new(3, 1, vec![Domain::Below(5), Domain::Values(vec![1, 3, 6])]).unwrap(),
        Domains::new(3, 1, vec![Domain::Below(0), Domain::Values(vec![1, 3, 6])]).unwrap(),
    ];
    let mut cases = 0;
    let mut nested_cases = 0;
    for d in &domains {
        let n = 1usize << d.dimensions();
        let support = bits(n, |w| d.contains(w as u64));
        let state = (1u64 << (2 * d.width())) - 1;
        let environment = ((1u64 << d.environment_bits()) - 1) << (3 * d.width());
        let mut masks = vec![0, 1, 3, 7, 4, 1 << d.width(), 1 << (2 * d.width())];
        for family in ["suffix-1", "suffix-half", "whole", "non-suffix"] {
            for faces in ["x", "y", "xy"] {
                masks.push(mask(d.width(), family, faces));
            }
        }
        if environment != 0 {
            masks.extend([environment, environment | 1]);
        }
        masks.sort_unstable();
        masks.dedup();
        for layout in ["bit-major", "face-major"] {
            let owner = C::legal_space(d, d.order(layout)).unwrap();
            assert_eq!(owner.export(owner.full()), support);
            for seed in 0..6 {
                let mut c = Memo::new(owner.clone(), seed % 2 == 0);
                let words = bits(n, |w| {
                    mix((w as u64 & (state | environment)) + seed * 37) % 5 < 2
                });
                let event = c.inner.import(&words);
                for &hidden in &masks {
                    let expected = reference(&support, &words, n, hidden);
                    let out = construct(&mut c, event, hidden);
                    verify(&c.inner, &out, &expected, hidden);
                    cases += 1;
                }
                let small = construct(&mut c, event, 1);
                let large = construct(&mut c, event, 3);
                assert_eq!(c.inner.op(4, small[1], large[1]), c.inner.empty());
                assert_eq!(c.inner.op(4, large[2], small[2]), c.inner.empty());
                assert_eq!(c.inner.exists(small[1], 3), large[1]);
                nested_cases += 1;
            }
        }
    }
    // A coupled support deliberately has no product or suffix certificate.
    let support = bits(64, |w| w & 3 == (w >> 4) & 3);
    let mut c = Memo::new(C::new(&support, 64), true);
    for seed in 0..8 {
        let words = bits(64, |w| mix(w as u64 + seed * 17) % 7 < 3);
        let event = c.inner.import(&words);
        for hidden in [0, 1, 3, 4, 15, 48, 63] {
            let out = construct(&mut c, event, hidden);
            verify(
                &c.inner,
                &out,
                &reference(&support, &words, 64, hidden),
                hidden,
            );
            cases += 1;
        }
    }
    // Marginal possibilities cannot replace a joint witness. Conversely,
    // collectively exhaustive alternatives can be guaranteed after grouping.
    let mut b = Memo::new(C::full_space(3, vec![2, 1, 0]).unwrap(), false);
    let a = b.inner.variable(0);
    let not_a = b.inner.not(a);
    let ra = construct(&mut b, a, 1);
    let rb = construct(&mut b, not_a, 1);
    assert_eq!(b.op(8, ra[1], rb[1]), b.inner.full());
    let joint = b.op(8, a, not_a);
    assert_eq!(construct(&mut b, joint, 1)[1], b.inner.empty());
    assert_eq!(b.op(14, ra[2], rb[2]), b.inner.empty());
    let covered = b.op(14, a, not_a);
    assert_eq!(construct(&mut b, covered, 1)[2], b.inner.full());
    println!(
        "EVENT_LAB {{\"kind\":\"readout_verification\",\"candidate\":\"{}\",\"cases\":{},\"nested_cases\":{},\"boundary_cases\":2,\"passed\":true}}",
        C::NAME,
        cases,
        nested_cases
    );
}

fn wide<C: Carrier>() {
    let width = 20;
    let d = Domains::new(
        width,
        1,
        vec![
            Domain::Below((1 << width) - 3),
            Domain::Below((1 << width) - 7),
        ],
    )
    .unwrap();
    let mut c = Memo::new(C::legal_space(&d, d.order("bit-major")).unwrap(), true);
    let environment = c.inner.variable(3 * width);
    let mut last = c.inner.empty();
    for e in 0..2 {
        let selector = if e == 0 {
            c.inner.not(environment)
        } else {
            environment
        };
        let value =
            super::legal::equal_value(&mut c.inner, 0, width, d.domain(e).cardinality() - 1);
        let part = c.op(8, selector, value);
        last = c.op(14, last, part);
    }
    let paired = c.inner.not(last);
    let low = c.inner.variable(0);
    let expected_low: u64 = (0..2)
        .map(|e| {
            let n = d.domain(e).cardinality();
            assert_eq!(n % 2, 1);
            (n / 2) * n * n
        })
        .sum();
    let expected_pairs: u64 = (0..2)
        .map(|e| {
            let n = d.domain(e).cardinality();
            (n - 1) * n * n
        })
        .sum();
    let out = construct(&mut c, low, 1);
    assert_eq!(out, [low, paired, c.inner.empty(), paired]);
    assert_eq!(c.inner.count(out[0]), expected_low);
    assert_eq!(c.inner.count(out[1]), expected_pairs);
    let not_low = c.inner.not(low);
    let out = construct(&mut c, not_low, 1);
    assert_eq!(out, [not_low, c.inner.full(), last, paired]);
    assert_eq!(c.inner.count(out[0]), d.population() - expected_low);
    assert_eq!(c.inner.count(out[2]), d.population() - expected_pairs);
    println!(
        "EVENT_LAB {{\"kind\":\"readout_symbolic_verification\",\"candidate\":\"{}\",\"coordinates\":61,\"cases\":2,\"passed\":true}}",
        C::NAME
    );
}

pub fn checks() {
    use super::essential::Essential;
    use super::finite::{Dense, Finite};
    use super::packed::Packed;
    use super::retraction::Retraction;
    finite::<Retraction<6, true>>();
    finite::<Retraction<9, true>>();
    finite::<Retraction<6>>();
    finite::<Retraction<9>>();
    finite::<Essential<6>>();
    finite::<Essential<9>>();
    finite::<Packed<8>>();
    finite::<Finite<Dense<true>>>();
    wide::<Retraction<6, true>>();
    wide::<Retraction<9, true>>();
    wide::<Retraction<6>>();
    wide::<Retraction<9>>();
    wide::<Essential<6>>();
    wide::<Essential<9>>();
    wide::<Packed<8>>();
}
