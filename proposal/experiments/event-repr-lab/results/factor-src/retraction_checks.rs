use super::carrier::*;
use super::legal::{Domain, Domains};
use super::observation::CountPlan;
use super::retraction::Retraction;

fn finite<const K: u32, const PREFIX: bool>() {
    let mut cases = 0;
    for domains in [
        vec![Domain::Below(3)],
        vec![Domain::Values(vec![1, 3])],
        vec![Domain::Values(vec![0, 2]), Domain::Values(vec![1])],
        vec![Domain::Below(0), Domain::Values(vec![1, 2])],
        vec![Domain::Values(vec![1]), Domain::Below(0)],
    ] {
        let d = Domains::new(2, if domains.len() == 1 { 0 } else { 1 }, domains).unwrap();
        let n = 1usize << d.dimensions();
        for layout in ["bit-major", "face-major"] {
            let mut c = Retraction::<K, PREFIX>::legal_space(&d, d.order(layout)).unwrap();
            let support = bits(n, |w| d.contains(w as u64));
            for w in 0..n {
                let decoded = c.decode_world(w as u64);
                assert!(d.contains(decoded));
                if PREFIX {
                    assert_eq!(decoded, greedy_world(&d, w as u64));
                }
                if d.contains(w as u64) {
                    assert_eq!(decoded, w as u64);
                }
                assert_eq!(c.decode_world(decoded), decoded);
            }
            let mut product_cases = 0;
            let second_data = bits(n, |w| mix(w as u64 + 997) % 5 < 3);
            let second = c.import(&second_data);
            let mut plan = CountPlan::new((0..d.dimensions()).map(|v| v as usize % 2).collect());
            c.prepare_spectrum(&mut plan);
            for seed in 0..16 {
                // Includes truly three-face Events, not only relation cylinders.
                let data = bits(n, |w| mix(w as u64 + seed * 29) % 7 < 3);
                let event = c.import(&data);
                let expected: Vec<_> = support.iter().zip(&data).map(|(&s, &a)| s & a).collect();
                assert_eq!(c.export(event), expected);
                assert_eq!(c.import(&expected), event);
                let before = (c.nodes(), c.bytes());
                c.set_word_counts(false);
                let scalar = c.count(event);
                c.set_word_counts(true);
                assert_eq!(c.count(event), scalar, "word/scalar joint contraction");
                assert_eq!(
                    c.count(event),
                    expected.iter().map(|w| w.count_ones() as u64).sum::<u64>()
                );
                assert_eq!(c.spectrum(event, &plan), plan.oracle(&expected, n));
                let not = event ^ 1;
                assert_eq!(c.count(event) + c.count(not), d.population());
                assert_eq!(
                    c.direct_signature(event, not),
                    Some(if event == 0 {
                        2
                    } else if event == 1 {
                        4
                    } else {
                        6
                    })
                );
                assert_eq!(
                    (c.nodes(), c.bytes()),
                    before,
                    "observation must not intern a clipped Event"
                );
                for mask in 0..1u64 << d.dimensions() {
                    let want = bits(n, |w| {
                        at(&support, w)
                            && (0..n).any(|v| {
                                (v & !(mask as usize)) == (w & !(mask as usize)) && at(&expected, v)
                            })
                    });
                    let got = c.exists(event, mask);
                    assert_eq!(c.export(got), want, "projection {mask}");
                    assert_eq!(got, c.import(&want));
                    if PREFIX {
                        let suffixes = mask >> 6 == 0
                            && (0..3).all(|k| {
                                let m = (mask >> (2 * k)) & 3;
                                m & (m + 1) == 0
                            });
                        assert_eq!(c.complete_fibres(mask), suffixes);
                        let product = c.relprod(event, second, mask);
                        let want_product = bits(n, |w| {
                            at(&support, w)
                                && (0..n).any(|v| {
                                    (v & !(mask as usize)) == (w & !(mask as usize))
                                        && at(&expected, v)
                                        && at(&second_data, v)
                                })
                        });
                        assert_eq!(c.export(product), want_product);
                        assert_eq!(product, c.import(&want_product));
                        product_cases += 1;
                    }
                    cases += 1;
                }
            }
            if PREFIX {
                assert!(product_cases > 0);
            }
            let relation = c.import(&bits(n, |w| w % 4 <= (w >> 2) % 4));
            assert_eq!(
                c.physical_axes(relation) & 0x30,
                0,
                "scratch face must physically disappear"
            );
            let identity = c.diagonal(&[(0, 2), (1, 3)]).unwrap();
            assert_eq!(c.physical_axes(identity) & 0x30, 0);
            // Semantic packets must carry original support alongside completed roots.
            let packet = c.packet(
                &(0..d.dimensions() as u64).collect::<Vec<_>>(),
                &[relation, identity],
            );
            let mut target =
                super::essential::Essential::<K>::legal_space(&d, d.order(layout)).unwrap();
            for (&r, &root) in [relation, identity].iter().zip(&packet.roots) {
                let map: Vec<_> = (0..d.dimensions()).map(Some).collect();
                let imported = super::transfer::evaluate_mode(
                    &packet,
                    root,
                    &map,
                    &mut target,
                    super::transfer::TableImport::Words,
                );
                assert_eq!(c.export(r), target.export(imported));
            }
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"retraction_verification\",\"candidate\":\"{}\",\"projection_cases\":{},\"passed\":true}}",
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
    decoder_exhaustive::<3>();
    decoder_exhaustive::<9>();
}

// Independent finite algorithm: inspect legal values, never symbolic formulas.
fn greedy(domain: &Domain, width: u32, input: u64) -> u64 {
    let mut chosen = 0;
    for bit in (0..width).rev() {
        let trial = chosen | (input & (1 << bit));
        let prefix_mask = ((1 << width) - 1) & !((1 << bit) - 1);
        let feasible = (0..1 << width).any(|v| domain.contains(v) && v & prefix_mask == trial);
        chosen = if feasible { trial } else { trial ^ (1 << bit) };
    }
    chosen
}
fn greedy_world(d: &Domains, w: u64) -> u64 {
    let base = 3 * d.width();
    let env = (w >> base) as usize;
    let env = if d.domain(env).minimum().is_some() {
        env
    } else {
        (d.anchor() >> base) as usize
    };
    let face = (1 << d.width()) - 1;
    (0..3).fold((env as u64) << base, |out, k| {
        out | (greedy(d.domain(env), d.width(), (w >> (k * d.width())) & face) << (k * d.width()))
    })
}
fn decoder_exhaustive<const K: u32>() {
    let mut cases = 0;
    for subset in 1..256 {
        let d = Domains::new(
            3,
            0,
            vec![Domain::Values(
                (0..8).filter(|v| subset >> v & 1 != 0).collect(),
            )],
        )
        .unwrap();
        for layout in ["face-major", "bit-major"] {
            let c = Retraction::<K, true>::legal_space(&d, d.order(layout)).unwrap();
            for w in 0..512 {
                assert_eq!(c.decode_world(w), greedy_world(&d, w));
                cases += 1;
            }
        }
    }
    let d = Domains::new(2, 0, vec![Domain::Values(vec![1, 3])]).unwrap();
    let mut c = Retraction::<K, true>::legal_space(&d, d.order("bit-major")).unwrap();
    let high = c.variable(1);
    assert_eq!(
        c.physical_axes(high),
        2,
        "prefix completion should keep high bit literally unchanged"
    );
    assert_eq!(c.variable(0), c.full());
    println!(
        "EVENT_LAB {{\"kind\":\"prefix_decoder_verification\",\"candidate\":\"{}\",\"world_cases\":{},\"passed\":true}}",
        Retraction::<K, true>::NAME,
        cases
    );
}
