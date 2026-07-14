use super::*;

/// The build-cost pin (measured): the 22%-cheaper build belonged to
/// a ctrl-word-IN-bucket layout (one line per insert); the shipped
/// spec keeps ctrl in a separate slab (the probe-side choice), so an
/// insert touches ctrl + key + child lines and the build measured
/// PARITY at the DRAM-tier 100k shape (ratio 1.00) and ~1.5× slower
/// at an L2-resident 20k shape. The pin protects DRAM-tier parity — the
/// force-heavy ledger families gate the rest. Biased AGAINST the
/// shipped side: the reference consumes pre-decoded keys while
/// `force()` pays its own column decode. Ignored: a microbenchmark,
/// run explicitly.
#[test]
#[ignore = "microbench pin: run explicitly with --ignored"]
fn bucketized_force_stays_at_parity_with_the_linear_build() {
    /// The prior build, reconstructed: linear probe over a ctrl
    /// byte slab + row-major `(key, child)` rows, first-empty
    /// insert, rehash-double at ITS OWN 3/4-load trigger
    /// (`(len + 1) * 4 >= capacity * 3`; the shipped bucket-of-8
    /// map's is the 0.4 max load) — near-distinct keys, so the
    /// duplicate/chunk machinery never fires and is elided.
    fn linear_build(keys: &[u64]) -> (Vec<u8>, Vec<u64>) {
        let mut capacity = ((keys.len() / 8).max(16)).next_power_of_two();
        let mut ctrl = vec![0u8; capacity];
        let mut rows = vec![0u64; capacity * 2];
        let mut len = 0usize;
        let mut dense: Vec<u32> = Vec::with_capacity(keys.len());
        for (pos, &k) in keys.iter().enumerate() {
            if (len + 1) * 4 >= capacity * 3 {
                // Rehash-double, insertion order preserved.
                let new_capacity = capacity * 2;
                let mut new_ctrl = vec![0u8; new_capacity];
                let mut new_rows = vec![0u64; new_capacity * 2];
                let mask = new_capacity - 1;
                for d in &mut dense {
                    let old = *d as usize;
                    let key = rows[2 * old];
                    let h = hash_words(&[key]);
                    let mut idx = usize::try_from(h).expect("64-bit") & mask;
                    while new_ctrl[idx] != 0 {
                        idx = (idx + 1) & mask;
                    }
                    new_ctrl[idx] = ctrl_tag(h);
                    new_rows[2 * idx] = key;
                    new_rows[2 * idx + 1] = rows[2 * old + 1];
                    *d = u32::try_from(idx).expect("fits");
                }
                capacity = new_capacity;
                ctrl = new_ctrl;
                rows = new_rows;
            }
            let h = hash_words(&[k]);
            let mask = capacity - 1;
            let wanted = ctrl_tag(h);
            let mut idx = usize::try_from(h).expect("64-bit") & mask;
            loop {
                let c = ctrl[idx];
                if c == 0 {
                    ctrl[idx] = wanted;
                    rows[2 * idx] = k;
                    rows[2 * idx + 1] = pos as u64;
                    dense.push(u32::try_from(idx).expect("fits"));
                    len += 1;
                    break;
                }
                if c == wanted && rows[2 * idx] == k {
                    break; // duplicate: absorbed (near-distinct corpus)
                }
                idx = (idx + 1) & mask;
            }
        }
        std::hint::black_box(len);
        (ctrl, rows)
    }

    let dir = TempDir::new("colt-build-pin");
    let schema = schema();
    let n = std::hint::black_box(100_000u64);
    let rows: Vec<(u64, u64)> = (0..n)
        .map(|i| (i.wrapping_mul(0x9E37_79B9_7F4A_7C15), i))
        .collect();
    let view = view_of(&dir, &schema, &rows);
    let decoded: Vec<u64> = view.column_words(0).to_vec();

    let mut bucket_best = std::time::Duration::MAX;
    let mut linear_best = std::time::Duration::MAX;
    for _ in 0..5 {
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let root = Colt::root();
        let start = std::time::Instant::now();
        colt.ensure_forced(root, 0);
        bucket_best = bucket_best.min(start.elapsed());
        assert!(matches!(colt.key_count(root), KeyCount::Exact(_)));

        let start = std::time::Instant::now();
        let built = linear_build(&decoded);
        linear_best = linear_best.min(start.elapsed());
        std::hint::black_box(&built);
    }
    let bucket_ns = u64::try_from(bucket_best.as_nanos()).expect("fits");
    let linear_ns = u64::try_from(linear_best.as_nanos()).expect("fits");
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )] // both far below 2^52
    let ratio = linear_ns as f64 / bucket_ns as f64;
    println!("force build: bucket {bucket_ns} ns, linear-ref {linear_ns} ns, ratio {ratio:.2}");
    assert!(
        linear_ns * 10 >= bucket_ns * 9,
        "bucketized build must stay within 1.11× of the linear reference: {bucket_ns} vs {linear_ns} ns"
    );
}
