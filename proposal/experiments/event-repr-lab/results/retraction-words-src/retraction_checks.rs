use super::carrier::*;
use super::legal::{Domain, Domains};
use super::observation::CountPlan;
use super::retraction::Retraction;

fn finite<const K: u32>() {
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
            let mut c = Retraction::<K>::legal_space(&d, d.order(layout)).unwrap();
            let support = bits(n, |w| d.contains(w as u64));
            for w in 0..n {
                let decoded = c.decode_world(w as u64);
                assert!(d.contains(decoded));
                if d.contains(w as u64) {
                    assert_eq!(decoded, w as u64);
                }
                assert_eq!(c.decode_world(decoded), decoded);
            }
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
                    cases += 1;
                }
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
        Retraction::<K>::NAME,
        cases
    );
}
pub fn all() {
    finite::<3>();
    finite::<6>();
    finite::<9>();
}
