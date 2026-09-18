//! Cardinality contraction through certified product fibres. These checks also
//! guard the fallback for arbitrary support and the original empty environments.
use super::carrier::*;
use super::legal::{Domain, Domains};
use super::retraction::Retraction;

fn count_modes<const K: u32, const PREFIX: bool>(
    c: &mut Retraction<K, PREFIX>,
    event: Id,
    expected: u64,
) {
    let before = (c.nodes(), c.bytes(), c.memory_stats());
    for factor in [false, true] {
        c.set_factor_counts(factor);
        for words in [false, true] {
            c.set_word_counts(words);
            assert_eq!(
                c.count(event),
                expected,
                "{} factor={factor} words={words}",
                Retraction::<K, PREFIX>::NAME
            );
            assert_eq!(c.count(event ^ 1), c.count(c.full()) - expected);
            assert_eq!(
                (c.nodes(), c.bytes(), c.memory_stats()),
                before,
                "count must remain read-only"
            );
        }
    }
}
fn finite<const K: u32, const PREFIX: bool>() {
    let configurations = [
        Domains::new(2, 0, vec![Domain::Below(4)]).unwrap(),
        Domains::new(2, 0, vec![Domain::Below(3)]).unwrap(),
        Domains::new(2, 0, vec![Domain::Values(vec![1, 3])]).unwrap(),
        Domains::new(2, 1, vec![Domain::Below(3), Domain::Values(vec![1])]).unwrap(),
        Domains::new(2, 1, vec![Domain::Below(0), Domain::Values(vec![1, 2])]).unwrap(),
        Domains::new(2, 1, vec![Domain::Values(vec![1]), Domain::Below(0)]).unwrap(),
        Domains::new(
            0,
            2,
            vec![
                Domain::Below(0),
                Domain::Below(1),
                Domain::Below(1),
                Domain::Below(1),
            ],
        )
        .unwrap(),
    ];
    let mut cases = 0;
    for d in &configurations {
        let n = 1usize << d.dimensions();
        let face_bits = (1u64 << d.width()) - 1;
        let state_bits = (1u64 << (3 * d.width())) - 1;
        let environment_bits = ((1u64 << d.dimensions()) - 1) & !state_bits;
        for layout in ["face-major", "bit-major"] {
            let mut c = Retraction::<K, PREFIX>::legal_space(d, d.order(layout)).unwrap();
            for faces in 0..8 {
                let axes = (0..3).fold(environment_bits, |m, f| {
                    m | if faces >> f & 1 != 0 {
                        face_bits << (f * d.width())
                    } else {
                        0
                    }
                });
                for seed in 0..16 {
                    let words = bits(n, |w| mix((w as u64 & axes) + seed * 37) % 7 < 3);
                    let expected = (0..n)
                        .filter(|&w| d.contains(w as u64) && at(&words, w))
                        .count() as u64;
                    let event = c.import(&words);
                    assert_eq!(c.physical_axes(event) & !axes, 0);
                    count_modes(&mut c, event, expected);
                    cases += 1;
                }
            }
        }
    }
    // A copied face is coupled support, so there is no product capability.
    let support = bits(64, |w| w & 3 == (w >> 4) & 3);
    let mut c = Retraction::<K, PREFIX>::new(&support, 64);
    for seed in 0..32 {
        let words = bits(64, |w| mix((w & 15) as u64 + seed * 29) % 5 < 3);
        let event = c.import(&words);
        let expected = (support[0] & words[0]).count_ones() as u64;
        count_modes(&mut c, event, expected);
        cases += 1;
    }
    println!(
        "EVENT_LAB {{\"kind\":\"factor_count_verification\",\"candidate\":\"{}\",\"cases\":{},\"count_modes\":4,\"passed\":true}}",
        Retraction::<K, PREFIX>::NAME,
        cases
    );
}
fn wide<const K: u32, const PREFIX: bool>() {
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
    let mut c = Retraction::<K, PREFIX>::legal_space(&d, d.order("bit-major")).unwrap();
    let mut cases = 0;
    for selected in 0u32..8 {
        let mut event = c.full();
        for face in 0..3 {
            if selected >> face & 1 != 0 {
                let bit = c.variable(face * width);
                event = c.op(8, event, bit);
            }
        }
        for environment_one in [false, true] {
            let event = if environment_one {
                let e = c.variable(3 * width);
                c.op(8, event, e)
            } else {
                event
            };
            let expected: u64 = (0..2)
                .filter(|&e| !environment_one || e == 1)
                .map(|e| {
                    let n = d.domain(e).cardinality();
                    (n / 2).pow(selected.count_ones()) * n.pow(3 - selected.count_ones())
                })
                .sum();
            count_modes(&mut c, event, expected);
            cases += 1;
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"factor_count_symbolic\",\"candidate\":\"{}\",\"coordinates\":61,\"cases\":{},\"passed\":true}}",
        Retraction::<K, PREFIX>::NAME,
        cases
    );
}
pub fn all() {
    finite::<3, false>();
    finite::<6, false>();
    finite::<9, false>();
    finite::<3, true>();
    finite::<6, true>();
    finite::<9, true>();
    wide::<6, false>();
    wide::<9, false>();
    wide::<6, true>();
    wide::<9, true>();
}
