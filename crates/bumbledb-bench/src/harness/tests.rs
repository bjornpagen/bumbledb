use super::*;

#[test]
fn stats_match_hand_computed_nearest_rank() {
    let mut one = vec![7];
    let s = stats(&mut one);
    assert_eq!((s.min, s.p50, s.p99, s.max, s.mean_ns), (7, 7, 7, 7, 7));

    let mut two = vec![20, 10];
    let s = stats(&mut two);

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
}

#[test]
fn batched_measurement_divides_time_and_sums_all_work() {
    let proto = Protocol {
        warmups: 2,
        samples: 4,
    };
    let mut calls = 0u64;
    let m = measure_batched(proto, 8, || {
        calls += 1;
        Ok(3)
    })
    .expect("measures");
    assert_eq!(
        calls,
        2 + 4 * 8,
        "warmups run once each; samples run batch times"
    );
    assert_eq!(m.work, 4 * 8 * 3, "work sums every batched call");
}

/// The touch runs before every sample (warmups included), and generations
/// strictly increase across samples on a real store.
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
    let db = bumbledb::Db::create(&dir, crate::schema::Ledger, crate::harness::bench_work())
        .expect("create")
        .expect("accepted");
    let generations = RefCell::new(Vec::new());
    measure_cold(proto, org_touch(&db), || {
        let generation = db
            .generation(crate::harness::bench_work())
            .map_err(|e| format!("{e:?}"))?;
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
