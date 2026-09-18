//! Cross-strategy canonical identity, independent legal-world partitions,
//! arbitrary raw substitutions, and conservative certificate fallbacks.
use super::carrier::*;
use super::legal::{Domain, Domains};
use super::retraction::Retraction;

fn check<const K: u32, const PREFIX: bool>(
    mut c: Retraction<K, PREFIX>,
    support: Vec<u64>,
) -> usize {
    let n = 1usize << c.dimensions();
    let second = bits(n, |w| w & 8 != 0 || w & 16 != 0);
    let b = c.import(&second);
    let mut cases = 0;
    for seed in 0..8 {
        let data = bits(n, |w| match seed {
            0 => false,
            1 => true,
            2 => w & 2 != 0,
            3 => w & 4 != 0,
            4 => (w & 2 != 0) ^ (w & 4 != 0),
            5 => (w & 1 != 0) == (w & 4 != 0),
            6 => (w & 2 != 0) ^ (w & 8 != 0) ^ (w & 32 != 0),
            _ => mix(w as u64 + 97) % 7 < 3,
        });
        let a = c.import(&data);
        let joint: Vec<_> = data.iter().zip(&second).map(|(&a, &b)| a & b).collect();
        for hidden in 0..n as u64 {
            let expected = super::readout::reference(&support, &data, n, hidden)[1].clone();
            let product = super::readout::reference(&support, &joint, n, hidden)[1].clone();
            let mut canonical = None;
            for policy in ["joint", "active", "witness"] {
                c.set_projection_gates(policy);
                for completion in ["full", "needed", "local", "local-needed"] {
                    c.set_projection_completion(completion);
                    let got = c.exists(a, hidden);
                    let got_product = c.relprod(a, b, hidden);
                    assert_eq!(c.export(got), expected, "{policy} exists mask {hidden}");
                    assert_eq!(
                        c.export(got_product),
                        product,
                        "{policy} product mask {hidden}"
                    );
                    if let Some(roots) = canonical {
                        assert_eq!((got, got_product), roots);
                    }
                    canonical = Some((got, got_product));
                    cases += 2;
                }
            }
        }
    }
    cases
}

fn finite<const K: u32, const PREFIX: bool>() {
    let mut product_cases = 0;
    for domains in [
        vec![Domain::Below(3)],
        vec![Domain::Values(vec![1, 3])],
        vec![Domain::Values(vec![0, 2]), Domain::Values(vec![1])],
        vec![Domain::Below(0), Domain::Values(vec![1, 2])],
        vec![Domain::Values(vec![1]), Domain::Below(0)],
    ] {
        let d = Domains::new(2, if domains.len() == 1 { 0 } else { 1 }, domains).unwrap();
        let n = 1usize << d.dimensions();
        for order in ["bit-major", "face-major"] {
            let c = Retraction::<K, PREFIX>::legal_space(&d, d.order(order)).unwrap();
            product_cases += check(c, bits(n, |w| d.contains(w as u64)));
        }
    }
    let mut coupled_cases = 0;
    for n in [16usize, 32] {
        let support = bits(n, |w| (w & 1 == (w >> 1) & 1) && w % 7 != 3);
        let c = Retraction::<K, PREFIX>::new(&support, n);
        coupled_cases += check(c, support);
    }
    let d = Domains::new(2, 1, vec![Domain::Below(0), Domain::Values(vec![1, 2])]).unwrap();
    let n = 1usize << d.dimensions();
    let mut substitutions = 0;
    for order in ["bit-major", "face-major"] {
        let mut c = Retraction::<K, PREFIX>::legal_space(&d, d.order(order)).unwrap();
        substitutions += c.check_partial_substitution(&bits(n, |w| mix(w as u64 + 43) % 5 < 2));
    }
    println!(
        concat!(
            "EVENT_LAB {{\"kind\":\"local_completion_verification\",",
            "\"candidate\":\"{}\",\"product_cases\":{},\"coupled_cases\":{},",
            "\"policies\":3,\"strategies\":4,\"substitution_cases\":{},\"passed\":true}}"
        ),
        Retraction::<K, PREFIX>::NAME,
        product_cases,
        coupled_cases,
        substitutions
    );
}

pub fn all() {
    finite::<3, false>();
    finite::<6, false>();
    finite::<9, false>();
    finite::<3, true>();
    finite::<6, true>();
    finite::<9, true>();
}
