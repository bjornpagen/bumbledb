use super::*;

/// splitmix64's finalizer: the fresh-data generator for the probe
/// twin bench (fresh keys per repetition — the TAGE discipline,
/// `m2max.predict.tage-memorizes-benchmarks`).
fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The arity-4 stored row for index `i` — distinct in word 0, so any
/// `i ≥ n_rows` is a guaranteed miss key.
fn row4(i: u64) -> [u64; 4] {
    [
        i,
        mix(i),
        i.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        i.rotate_left(23) ^ 0xA5A5_5A5A_DEAD_BEEF,
    ]
}

/// P(a, b, c, d u64) — the arity-4 probe fixture.
fn schema4() -> Schema {
    let field = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "P".into(),
            fields: vec![field("a"), field("b"), field("c"), field("d")],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Builds an image over `n` committed [`row4`] rows.
fn view4_of(dir: &TempDir, schema: &Schema, n: u64) -> Arc<crate::image::RelationImage> {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    let mut bytes = Vec::new();
    for i in 0..n {
        let w = row4(i);
        bytes.clear();
        encode_fact(
            &[
                ValueRef::U64(w[0]),
                ValueRef::U64(w[1]),
                ValueRef::U64(w[2]),
                ValueRef::U64(w[3]),
            ],
            schema.relation(R).layout(),
            &mut bytes,
        );
        delta.insert(&view, R, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    crate::image::build(&txn, schema, R).expect("build")
}

/// Identity, pinned in a general register: an empty asm template whose
/// only effect is the operand constraint — the compiler cannot trace
/// the value through it, so the XOR-difference OR-reduction in
/// [`flag_free_probe4`] stays an `eor`/`orr` tree ending in one `cbz`
/// instead of being re-fused into the serial `cmp` + `ccmp` flag chain
/// (the `AArch64` backend's or-of-xor combine — LLVM substitutes, the
/// machine code is the arm). Zero instructions, zero memory, flags
/// preserved. `hint::black_box` pins the same shape at the price of a
/// stack round trip per candidate.
#[inline(always)]
#[cfg_attr(
    target_arch = "aarch64",
    expect(
        unsafe_code,
        reason = "the localized unsafe operation has a documented safety invariant"
    )
)]
fn opaque(diff: u64) -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let mut pinned = diff;
        // SAFETY: an empty template — no instructions execute, no memory
        // or flags are touched (`nomem`, `nostack`, `preserves_flags`).
        unsafe {
            core::arch::asm!(
                "/* {0} */",
                inout(reg) pinned,
                options(nomem, nostack, preserves_flags)
            );
        }
        pinned
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        diff
    }
}

/// The REFUTED flag-free twin (T3), preserved as the falsifier's arm:
/// the candidate compare XOR-differences the 4 stored words, OR-reduces
/// and exits on one `cbz` — zero cmp/ccmp µops in the candidate compare
/// (disassembly-checked on this test binary) where the shipped walk
/// carries the serial `cmp` + `ccmp`×3 chain — and the key-word reads
/// are unchecked (safety: the map's bucket range is
/// `bucket_start .. bucket_start + nbuckets * stride`, sized exactly so
/// at every mint and only appended after; `b ≤ nbm`, `slot < 8`,
/// `i < 4 = arity`). Everything else is the shipped walk, line for
/// line. The `m2max.core.flag-strand-mlp` prediction — 1.2–1.7× at
/// DRAM-tier displaced probes — measured 0.95–1.03 instead: the probe
/// batch's cross-element independence already saturates the miss
/// lanes, so unparking the flag triad buys nothing here.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn flag_free_probe4(colt: &Colt, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
    const A: usize = 4;
    let key: &[u64; A] = key.first_chunk().expect("key is arity-wide");
    let nbm = m.nbuckets - 1;
    let wanted = ctrl_tag(hash);
    let (groups, _) = colt.ctrl.as_chunks::<8>();
    let group_base = m.ctrl_start / 8;
    let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
    loop {
        let cw = u64::from_le_bytes(groups[group_base + b]);
        let mut matches = eq_byte_mask(cw, wanted);
        while matches != 0 {
            let slot = (matches.trailing_zeros() as usize) >> 3;
            let base = m.bucket_start + b * (8 * A + 8);
            let mut diff = 0u64;
            #[expect(
                clippy::needless_range_loop,
                reason = "the explicit constant range is the intended unroll shape"
            )]
            for i in 0..A {
                debug_assert!(base + i * 8 + slot < colt.buckets.len());
                // SAFETY: in the map's bucket range — see the fn doc.
                let stored = unsafe { *colt.buckets.get_unchecked(base + i * 8 + slot) };
                diff |= stored ^ key[i];
            }
            if opaque(diff) == 0 {
                return (true, b * 8 + slot);
            }
            matches &= matches - 1;
        }
        let empties = zero_byte_mask(cw);
        if empties != 0 {
            let slot = (empties.trailing_zeros() as usize) >> 3;
            return (false, b * 8 + slot);
        }
        b = (b + 1) & nbm;
    }
}

/// One timed pass of the shipped (cmp+ccmp) probe. `inline(never)`:
/// a clean symbol for the test binary's disassembly check, and no
/// cross-arm optimization.
#[inline(never)]
fn shipped_pass(colt: &Colt, m: &Map, keys: &[u64], hashes: &[u64]) -> u64 {
    let mut hits = 0u64;
    for (j, h) in hashes.iter().enumerate() {
        let (found, _) = colt.probe_hashed(m, &keys[j * 4..j * 4 + 4], *h);
        hits += u64::from(found);
    }
    hits
}

/// One timed pass of the flag-free twin ([`flag_free_probe4`]).
#[inline(never)]
fn flag_free_pass(colt: &Colt, m: &Map, keys: &[u64], hashes: &[u64]) -> u64 {
    let mut hits = 0u64;
    for (j, h) in hashes.iter().enumerate() {
        let (found, _) = flag_free_probe4(colt, m, &keys[j * 4..j * 4 + 4], *h);
        hits += u64::from(found);
    }
    hits
}

/// Streams a foreign buffer between probe passes — residency is a
/// property of phase interleaving, not footprint
/// (`m2max.mem.residency-is-interleaving`), so the displaced regime is
/// constructed by realistic foreign traffic, never by cache flushing.
#[inline(never)]
fn stream_foreign(buf: &[u64]) -> u64 {
    let mut acc = 0u64;
    for &w in buf {
        acc = acc.wrapping_add(w);
    }
    acc
}

/// Fresh arity-4 probe keys + hashes for one repetition: `hit_pct`%
/// drawn from the stored rows, the rest guaranteed-absent (word 0
/// past the insert domain). Hashes precomputed — phase 1 is ALU work,
/// the timed pass is phase 2's load chain.
fn gen_probe_keys(
    rep: u64,
    hit_pct: u64,
    n_rows: u64,
    probes: usize,
    keys: &mut Vec<u64>,
    hashes: &mut Vec<u64>,
) {
    keys.clear();
    hashes.clear();
    for j in 0..probes as u64 {
        let r = mix(rep.wrapping_mul(0x1000_0000).wrapping_add(j));
        let row = if r % 100 < hit_pct {
            row4(mix(r) % n_rows)
        } else {
            row4(n_rows + mix(r) % n_rows)
        };
        keys.extend_from_slice(&row);
        hashes.push(hash_words(&row));
    }
}

/// The flag-free probe-compare twin bench (T3) — the falsifier that
/// REFUTED it. The prediction (`m2max.core.flag-strand-mlp`): the
/// shipped candidate compare's serial cmp+ccmp×3 chain parks flag µops
/// in the 3-port triad's scheduler behind every displaced-bucket miss,
/// so a flag-free eor/orr/cbz compare should buy 1.2–1.7× at DRAM-tier
/// displaced probes and ±0 at L2. Measured (interleaved same-session
/// A/B, fresh keys per repetition, arm order alternating, arity-4
/// 42 MB map displaced by 96 MB of foreign traffic between passes per
/// `m2max.mem.residency-is-interleaving`): shipped/twin medians
/// 0.95–1.03 at the DRAM tier across hit rates 10/50/90 and two
/// sessions — a wash-to-small-loss where 1.2–1.7 was predicted — and
/// 1.02–1.08 at L2 (a small inversion of the predicted ±0, largest at
/// 90% hits where the compare actually runs). The gravestone: the probe batch's
/// cross-element independence is already saturating the miss lanes —
/// each probe is an independent 2-deep chain (ctrl word, then up to 4
/// spread key words), so the out-of-order window supplies the lanes and the
/// parked flag µops never bind; unparking them just trades 4 triad
/// µops for 7 wider-tree ALU µops. Prints per-regime/per-hit-rate
/// ratio distributions (>1 = twin wins); asserts only arm agreement —
/// the timing verdict belongs to the measured falsifier run under
/// `scripts/measure.sh`, not to CI ambient.
#[test]
#[ignore = "microbench pin: run explicitly with --ignored"]
fn flag_free_compare_twin_at_displaced_and_resident_probes() {
    const PROBES: usize = 100_000;
    const REPS: u64 = 20;
    let schema = schema4();

    // DRAM-tier displaced: 400k keys → 131072 buckets × 320 B ≈ 42 MB
    // of bucket slab (+1 MB ctrl). L2-resident: 20k keys ≈ 2.7 MB.
    let regimes: &[(&str, u64, bool)] = &[
        ("displaced-dram", 400_000, true),
        ("l2-resident", 20_000, false),
    ];
    // 96 MB of foreign traffic — past the SLC, streamed between passes.
    let foreign: Vec<u64> = (0..12_000_000u64).map(mix).collect();

    for &(regime, n_rows, displace) in regimes {
        let dir = TempDir::new("colt-flagfree-twin");
        let view = view4_of(&dir, &schema, n_rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0, 1, 2, 3]]);
        let root = Colt::root();
        colt.ensure_forced(root, 0);
        let m = colt.maps[0];
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )] // far below 2^52
        let slab_mb = (m.nbuckets * m.stride() * 8) as f64 / 1e6;

        let mut keys = Vec::new();
        let mut hashes = Vec::new();
        for &hit_pct in &[10u64, 50, 90] {
            let mut ratios = Vec::new();
            for rep in 0..REPS {
                gen_probe_keys(rep, hit_pct, n_rows, PROBES, &mut keys, &mut hashes);
                let mut ns = [0f64; 2]; // [shipped, flag-free twin]
                // Arm order alternates per rep (drift cancellation).
                for arm_slot in 0..2 {
                    let shipped_arm = (rep % 2 == 0) == (arm_slot == 0);
                    if displace {
                        std::hint::black_box(stream_foreign(&foreign));
                    }
                    let start = std::time::Instant::now();
                    let hits = if shipped_arm {
                        shipped_pass(&colt, &m, &keys, &hashes)
                    } else {
                        flag_free_pass(&colt, &m, &keys, &hashes)
                    };
                    let nanos = start.elapsed().as_nanos();
                    std::hint::black_box(hits);
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "reporting accepts lossy integer-to-float conversion"
                    )]
                    {
                        ns[usize::from(!shipped_arm)] = nanos as f64 / PROBES as f64;
                    }
                }
                ratios.push(ns[0] / ns[1]); // shipped / twin: >1 = twin wins
            }
            ratios.sort_by(f64::total_cmp);
            let median = ratios[ratios.len() / 2];
            println!(
                "flag-free twin [{regime} {slab_mb:.0} MB, hit {hit_pct}%]: \
                 shipped/twin median {median:.3}, \
                 min {:.3}, max {:.3}",
                ratios[0],
                ratios[ratios.len() - 1]
            );
        }

        // Arm agreement: identical (found, slot) on a fresh key set.
        gen_probe_keys(REPS, 50, n_rows, PROBES, &mut keys, &mut hashes);
        for (j, h) in hashes.iter().enumerate() {
            let key = &keys[j * 4..j * 4 + 4];
            assert_eq!(
                colt.probe_hashed(&m, key, *h),
                flag_free_probe4(&colt, &m, key, *h),
                "arm disagreement at probe {j}"
            );
        }
    }
}

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
