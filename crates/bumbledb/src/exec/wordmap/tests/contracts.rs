use super::*;

/// The hash-quality contract (measured):
/// **false-tag rate — not probe length — is the sensitive quality
/// metric** for tagged tables. A single-multiply fold hash passes
/// probe-length vetting (avg 1.40) while collapsing the 7-bit tag to
/// 19.4% false compares on strided keys (design point 1/128). This
/// test gates WHATEVER hash the module ships, by property: across
/// adversarial key families, the measured false-compare rate per
/// probe must stay ≤ 2/128. The `#[should_panic]` companion proves
/// the gate's teeth: a plausible cheaper hash fails it.
#[test]
fn false_tag_rate_stays_at_the_design_point_on_adversarial_keys() {
    for (name, rate) in adversarial_false_tag_rates(super::hash_words) {
        println!("false-compare rate [{name}]: {rate:.5}");
        assert!(
            rate <= 2.0 / 128.0,
            "family {name}: false-compare rate {rate:.5} above 2/128"
        );
    }
}

/// The red case, visible in review: a single-multiply fold hash —
/// 2× cheaper, passes probe-length vetting — collapses the tag on
/// low-entropy keys. If a future "optimization" swaps the hash and
/// this stops panicking, the swap broke the tag and the gate above
/// will say so.
#[test]
#[should_panic(expected = "above 2/128")]
fn a_single_multiply_hash_fails_the_false_tag_gate() {
    fn foldmul(words: &[u64]) -> u64 {
        let mut h = 0u64;
        for w in words {
            h = (h ^ w).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
        h
    }
    for (name, rate) in adversarial_false_tag_rates(foldmul) {
        assert!(
            rate <= 2.0 / 128.0,
            "family {name}: false-compare rate {rate:.5} above 2/128"
        );
    }
}

/// Measures the false-compare rate (tag matched, key mismatched, per
/// probe) of `hash` across the adversarial key families, by walking
/// a simulated 25%-loaded table exactly as the probe does. The walk
/// is a model, not the shipped window probe: the metric is a hash
/// property, independent of probe mechanics.
fn adversarial_false_tag_rates(hash: fn(&[u64]) -> u64) -> Vec<(&'static str, f64)> {
    let families: Vec<(&'static str, Vec<Vec<u64>>)> = vec![
        ("sequential", (0..16_384u64).map(|i| vec![i]).collect()),
        ("strided-8", (0..16_384u64).map(|i| vec![i * 8]).collect()),
        (
            "strided-4096",
            (0..16_384u64).map(|i| vec![i * 4096]).collect(),
        ),
        (
            "biased-i64-small",
            (0..16_384u64)
                .map(|i| vec![(1u64 << 63) ^ i.wrapping_sub(8_192)])
                .collect(),
        ),
        (
            "fresh-pairs",
            (0..16_384u64).map(|i| vec![i, i / 64]).collect(),
        ),
        ("random-control", {
            let mut rng = 0xDEAD_BEEF_u64;
            (0..16_384)
                .map(|_| {
                    rng = rng
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    vec![rng]
                })
                .collect()
        }),
    ];
    families
        .into_iter()
        .map(|(name, keys)| {
            let arity = keys[0].len();
            // A 25%-loaded model table: capacity = 4 × keys, linear
            // probing, tag = top-7 bits — the shipped geometry.
            let capacity = (keys.len() * 4).next_power_of_two();
            let mask = capacity - 1;
            let mut slots: Vec<Option<usize>> = vec![None; capacity];
            let mut tags: Vec<u8> = vec![0; capacity];
            for (ki, key) in keys.iter().enumerate() {
                let h = hash(key);
                let mut idx = usize::try_from(h).expect("64-bit") & mask;
                loop {
                    if slots[idx].is_none() {
                        slots[idx] = Some(ki);
                        tags[idx] = super::tag(h);
                        break;
                    }
                    idx = (idx + 1) & mask;
                }
            }
            // Probe every key (hits) plus an equal count of misses
            // drawn from the same family shape, counting steps where
            // the tag matched but the key did not.
            let mut probes = 0usize;
            let mut false_compares = 0usize;
            let mut probe = |key: &[u64]| {
                let h = hash(key);
                let wanted = super::tag(h);
                let mut idx = usize::try_from(h).expect("64-bit") & mask;
                probes += 1;
                loop {
                    match slots[idx] {
                        None => break,
                        Some(ki) => {
                            if tags[idx] == wanted {
                                if keys[ki].as_slice() == key {
                                    break;
                                }
                                false_compares += 1;
                            }
                        }
                    }
                    idx = (idx + 1) & mask;
                }
            };
            for key in &keys {
                probe(key);
            }
            for i in 0..keys.len() as u64 {
                // Same shape, disjoint values (offset far past the family).
                let miss: Vec<u64> = keys[usize::try_from(i).expect("small") % keys.len()]
                    .iter()
                    .map(|w| w.wrapping_add(0x0100_0000_0000_0000))
                    .collect();
                debug_assert_eq!(miss.len(), arity);
                probe(&miss);
            }
            #[allow(clippy::cast_precision_loss)]
            let rate = false_compares as f64 / probes as f64;
            (name, rate)
        })
        .collect()
}

/// The const-arity contract: `hash_core::<K>` is
/// hash-IDENTICAL to `hash_words` — same seed, fold order, constants
/// — so the monomorph and dyn arms land keys in the same slots and
/// the false-tag gate covers both.
#[test]
fn hash_core_is_identical_to_hash_words() {
    fn check<const K: usize>(next: &mut impl FnMut() -> u64) {
        for _ in 0..1_000 {
            let key: Vec<u64> = (0..K).map(|_| next()).collect();
            assert_eq!(hash_core::<K>(&key), hash_words(&key), "K={K}");
        }
    }
    let mut rng = 0x0F1E_2D3C_4B5A_6978u64;
    let mut next = move || {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        rng
    };
    check::<1>(&mut next);
    check::<2>(&mut next);
    check::<3>(&mut next);
    check::<4>(&mut next);
    check::<6>(&mut next);
    check::<8>(&mut next);
}

/// Probe-step evidence: average probe steps
/// at the shipped max load stay near 1 (gate: ≤ 1.2).
#[test]
fn probe_steps_stay_near_one_at_max_load() {
    let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 32_768);
    let mut rng = 7u64;
    let mut next = move || {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        rng >> 16
    };
    for _ in 0..32_768 {
        map.insert(&[next(), next()]);
    }
    assert!(
        map.len() * LOAD_DEN <= map.values.len(),
        "the sweep runs at the shipped max load"
    );
    // Measure probes for hits over every key (slot-step model:
    // window loads amortize these steps 8-at-a-time, but the walk
    // length is the portable quality metric).
    let keys: Vec<Vec<u64>> = map.iter().map(|(k, ())| k.to_vec()).collect();
    let mask = map.values.len() - 1;
    let mut steps = 0usize;
    for key in &keys {
        let hash = super::hash_words(key);
        let mut idx = usize::try_from(hash).expect("64-bit") & mask;
        loop {
            steps += 1;
            let c = map.ctrl[idx];
            assert_ne!(c, 0, "key exists");
            if c == super::tag(hash)
                && &map.keys[idx * map.arity..(idx + 1) * map.arity] == key.as_slice()
            {
                break;
            }
            idx = (idx + 1) & mask;
        }
    }
    #[allow(clippy::cast_precision_loss)] // both far below 2^52
    let avg = steps as f64 / keys.len() as f64;
    println!("avg probe steps at the shipped max load: {avg:.3}");
    assert!(avg <= 1.2, "near-one probe steps at 25% load, got {avg}");
}
