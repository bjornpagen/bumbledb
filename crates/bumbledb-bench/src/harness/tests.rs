use super::*;

#[test]
fn stats_match_hand_computed_nearest_rank() {
    let mut one = vec![7];
    let s = stats(&mut one);
    assert_eq!((s.min, s.p50, s.p99, s.max, s.mean_ns), (7, 7, 7, 7, 7));

    let mut two = vec![20, 10];
    let s = stats(&mut two);
    // n = 2: p50 rank = ceil(1) - 1 = 0 -> 10; p90/p95/p99 -> 20.
    assert_eq!(s.min, 10);
    assert_eq!(s.p50, 10);
    assert_eq!(s.p90, 20);
    assert_eq!(s.p95, 20);
    assert_eq!(s.p99, 20);
    assert_eq!(s.max, 20);
    assert_eq!(s.mean_ns, 15);

    let mut hundred: Vec<u64> = (1..=100).rev().collect();
    let s = stats(&mut hundred);
    assert_eq!(s.min, 1);
    assert_eq!(s.p50, 50);
    assert_eq!(s.p90, 90);
    assert_eq!(s.p95, 95);
    assert_eq!(s.p99, 99);
    assert_eq!(s.max, 100);
    assert_eq!(s.mean_ns, 50, "integer mean of 50.5");
}

#[test]
fn rotation_is_deterministic_round_robin() {
    let mut rotation = Rotation::new(vec![
        vec![Value::U64(0)],
        vec![Value::U64(1)],
        vec![Value::U64(2)],
    ]);
    let order: Vec<Value> = (0..7).map(|_| rotation.next_set()[0].clone()).collect();
    let expected: Vec<Value> = [0, 1, 2, 0, 1, 2, 0].map(Value::U64).into();
    assert_eq!(order, expected);
}

#[test]
fn measure_calls_exactly_warmups_plus_samples_and_sums_work() {
    let proto = Protocol {
        warmups: 3,
        samples: 5,
    };
    let mut calls = 0u64;
    let m = measure(proto, || {
        calls += 1;
        Ok(2)
    })
    .expect("measures");
    assert_eq!(calls, 8, "3 warmups + 5 samples");
    assert_eq!(m.work, 10, "work sums the samples only");
    assert!(m.trace.is_none());
}

/// The per-rep normalization unmasks contamination the block
/// bracket misses (docs/silicon2/00): a sample that ran at 2.0 GHz
/// reads 1.75x slow raw; normalized to the cohort's 3.5 GHz it
/// rejoins the population — and a GENUINELY slow sample stays slow.
#[test]
fn normalization_corrects_slow_clock_samples_and_keeps_real_ones() {
    // Five samples: four clean at 100 ns/3.5 GHz, one clock-slowed
    // (same work, 2.0 GHz -> 175 ns raw). Raw p50 is fine but raw
    // MIN-based protocols would also pass — the failure mode is when
    // slow-clock samples DOMINATE: three of five contaminated.
    let samples = [100u64, 175, 175, 175, 100];
    let ghz = [3.5f64, 2.0, 2.0, 2.0, 3.5];
    // Raw p50 = 175 (contaminated); normalized p50 = 100.
    let mut raw = samples.to_vec();
    assert_eq!(stats(&mut raw).p50, 175);
    assert_eq!(normalized_p50(&samples, &ghz), 100);
    // A genuinely slow sample at full clock stays slow.
    let samples = [100u64, 300, 100, 100, 100];
    let ghz = [3.5f64; 5];
    assert_eq!(normalized_p50(&samples, &ghz), 100);
    let mut raw = samples.to_vec();
    assert_eq!(stats(&mut raw).p50, 100);
    let samples = [300u64, 300, 300, 100, 100];
    assert_eq!(normalized_p50(&samples, &ghz), 300, "real slowness survives");
}

/// End-to-end: the per-rep mode populates `p50_norm`. Ignored:
/// timing-adjacent (runs ~200 us of proxy per sample).
#[test]
#[ignore = "per-rep proxy e2e (docs/silicon2/00 gate); run manually"]
fn per_rep_proxy_mode_populates_the_normalized_p50() {
    let proto = Protocol {
        warmups: 1,
        samples: 8,
    };
    let m = measure_batched(
        proto,
        Modes {
            alloc_window: false,
            trace: false,
            proxy_per_rep: true,
        },
        1,
        || Ok(std::hint::black_box((0..10_000u64).sum::<u64>())),
    )
    .expect("measures");
    let norm = m.p50_norm.expect("per-rep mode populates p50_norm");
    // Normalization rescales toward the best clock: never above raw.
    assert!(norm <= m.stats.p50 + m.stats.p50 / 10);
    let off = measure_batched(proto, Modes::default(), 1, || Ok(1)).expect("measures");
    assert!(off.p50_norm.is_none(), "off by default");
}

#[test]
fn batched_measurement_divides_time_and_sums_all_work() {
    let proto = Protocol {
        warmups: 2,
        samples: 4,
    };
    let mut calls = 0u64;
    let m = measure_batched(proto, Modes::default(), 8, || {
        calls += 1;
        Ok(3)
    })
    .expect("measures");
    assert_eq!(calls, 2 + 4 * 8, "warmups run once each; samples run batch times");
    assert_eq!(m.work, 4 * 8 * 3, "work sums every batched call");
}

#[test]
fn the_modes_are_mutually_exclusive() {
    let err = measure_batched(
        Protocol::COLD,
        Modes {
            alloc_window: true,
            trace: true,
            proxy_per_rep: false,
        },
        1,
        || Ok(0),
    )
    .expect_err("must refuse");
    assert!(err.contains("mutually exclusive"), "{err}");
}

#[test]
fn trace_mode_adds_one_post_measurement_sample() {
    let proto = Protocol {
        warmups: 1,
        samples: 2,
    };
    let mut calls = 0u64;
    let m = measure_batched(
        proto,
        Modes {
            alloc_window: false,
            trace: true,
            proxy_per_rep: false,
        },
        1,
        || {
            calls += 1;
            Ok(1)
        },
    )
    .expect("measures");
    assert_eq!(calls, 4, "warmup + samples + the traced sample");
    assert_eq!(m.work, 2, "the traced sample's work is not summed");
    let (traced_work, _) = m.trace.expect("traced");
    assert_eq!(traced_work, 1);
}

#[cfg(feature = "obs")]
#[test]
fn the_alloc_window_returns_a_snapshot() {
    let proto = Protocol {
        warmups: 2,
        samples: 4,
    };
    let m = measure_batched(
        proto,
        Modes {
            alloc_window: true,
            trace: false,
            proxy_per_rep: false,
        },
        1,
        || Ok(std::hint::black_box(vec![1u8; 4096]).len() as u64),
    )
    .expect("measures");
    let alloc = m.alloc.expect("windowed");
    assert!(alloc.allocs >= 4, "each sample allocates");
    assert!(alloc.alloc_bytes >= 4 * 4096);
}

#[cfg(not(feature = "obs"))]
#[test]
fn the_alloc_window_refuses_without_the_feature() {
    let err = measure_batched(
        Protocol::COLD,
        Modes {
            alloc_window: true,
            trace: false,
            proxy_per_rep: false,
        },
        1,
        || Ok(0),
    )
    .expect_err("must refuse");
    assert!(err.contains("obs feature"), "{err}");
}

/// The touch runs before every sample (warmups included), and
/// generations strictly increase across samples on a real store.
#[test]
fn cold_touches_before_every_sample_and_bumps_generations() {
    use std::cell::RefCell;
    let script = RefCell::new(String::new());
    let proto = Protocol {
        warmups: 2,
        samples: 3,
    };
    let m = measure_cold(
        proto,
        || {
            script.borrow_mut().push('t');
            Ok(())
        },
        || {
            script.borrow_mut().push('f');
            Ok(1)
        },
    )
    .expect("measures");
    assert_eq!(script.borrow().as_str(), "tftftftftf");
    assert_eq!(m.work, 3, "samples only");

    let dir = std::env::temp_dir().join("bumbledb-bench-harness-cold");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, crate::schema::schema()).expect("create");
    let generations = RefCell::new(Vec::new());
    measure_cold(proto, tag_touch(&db), || {
        let generation = db.generation().map_err(|e| format!("{e:?}"))?;
        generations.borrow_mut().push(generation);
        Ok(1)
    })
    .expect("measures");
    let generations = generations.into_inner();
    assert_eq!(generations.len(), 5);
    assert!(
        generations.windows(2).all(|w| w[0] < w[1]),
        "every touch bumps the generation: {generations:?}"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
