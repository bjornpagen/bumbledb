use super::*;

/// The const-arity pin (measured): the K=4
/// monomorphic insert beats the dyn arm on a
/// 16 MB miss-heavy fill. The headline 1.9× was measured against a
/// dyn reconstruction which still carried the general-length
/// compare ladder; the shipped dyn arm was already
/// dieted (manual word loops, no `bcmp`), so the honest in-tree
/// margin is 1.16–1.25× (16 MB / 2 MB tiers). The pin guards the
/// MECHANISM — monomorph strictly beats dyn — at a ≥ 10% floor that
/// survives tier noise. Both arms probe OPAQUE runtime slices (flat
/// buffer, black-boxed arity) — the shipped sink shape — so the
/// compiler cannot const-prop the key width into either arm from the
/// test itself; the monomorph arm's width knowledge comes only from
/// the internal dispatch. Ignored: a microbenchmark, run explicitly
/// for the Result section.
#[test]
#[ignore = "microbench pin: run explicitly with --ignored"]
fn const_arity_k4_insert_beats_the_dyn_arm() {
    // 128k arity-4 keys: capacity (128k×3).next_pow2 = 512k slots,
    // 512k × 32 B keys = 16 MiB — the DRAM-tier miss-heavy fill.
    const N: usize = std::hint::black_box(128) * 1024;
    let arity = std::hint::black_box(4usize);
    let flat: Vec<u64> = {
        let mut rng = 0x5DEE_CE66_D42F_1A2Bu64;
        (0..N * arity)
            .map(|_| {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                rng
            })
            .collect()
    };
    let fill_core = |flat: &[u64]| {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(arity, N);
        let start = std::time::Instant::now();
        for i in 0..N {
            map.insert(&flat[i * arity..(i + 1) * arity]);
        }
        let elapsed = start.elapsed();
        assert_eq!(map.len(), N);
        elapsed
    };
    let fill_dyn = |flat: &[u64]| {
        let mut map: WordMap<()> = WordMap::with_capacity_hint(arity, N);
        let start = std::time::Instant::now();
        for i in 0..N {
            let key = &flat[i * arity..(i + 1) * arity];
            let _ = map.entry_dyn(key, hash_words(key), || ());
        }
        let elapsed = start.elapsed();
        assert_eq!(map.len(), N);
        elapsed
    };
    // Interleaved min-of-5 (min-of-N
    // absorbs DVFS and residency noise without a proxy dependency).
    let mut core_best = std::time::Duration::MAX;
    let mut dyn_best = std::time::Duration::MAX;
    for _ in 0..5 {
        core_best = core_best.min(fill_core(&flat));
        dyn_best = dyn_best.min(fill_dyn(&flat));
    }
    let core_ns = u64::try_from(core_best.as_nanos()).expect("fits");
    let dyn_ns = u64::try_from(dyn_best.as_nanos()).expect("fits");
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )] // both far below 2^52
    let ratio = dyn_ns as f64 / core_ns as f64;
    println!("const-arity K=4 fill: core {core_ns} ns, dyn {dyn_ns} ns, ratio {ratio:.2}");
    assert!(
        dyn_ns * 10 >= core_ns * 11,
        "K=4 monomorph must beat the dyn arm by ≥ 10%: core {core_ns} ns vs dyn {dyn_ns} ns"
    );
}
