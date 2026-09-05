use super::*;

fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn row4(i: u64) -> [u64; 4] {
    [
        i,
        mix(i),
        i.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        i.rotate_left(23) ^ 0xA5A5_5A5A_DEAD_BEEF,
    ]
}

fn schema4() -> Schema {
    let field = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
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

fn view4_of(schema: &Schema, n: u64) -> Arc<crate::image::RelationImage> {
    let facts: Vec<Vec<crate::ir::Value>> = (0..n)
        .map(|i| {
            let w = row4(i);
            vec![
                crate::ir::Value::U64(w[0]),
                crate::ir::Value::U64(w[1]),
                crate::ir::Value::U64(w[2]),
                crate::ir::Value::U64(w[3]),
            ]
        })
        .collect();
    let source = crate::image::testsupport::TestSource::new(schema, &[(R, facts)]);
    let (_cache, image) = source.image_with_cache(R);
    image
}

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

/// The REFUTED flag-free twin (T3), preserved as the falsifier's arm: the
/// candidate compare XOR-differences the 4 stored words, OR-reduces and exits
/// on one `cbz` — zero cmp/ccmp µops in the candidate compare
/// (disassembly-checked on this test binary) where the shipped walk carries the
/// serial `cmp` + `ccmp`×3 chain — and the key-word reads are unchecked
/// (safety: the map's bucket range is `bucket_start..bucket_end`).
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

#[inline(never)]
fn shipped_pass(colt: &Colt, m: &Map, keys: &[u64], hashes: &[u64]) -> u64 {
    let mut hits = 0u64;
    for (j, h) in hashes.iter().enumerate() {
        let (found, _) = colt.probe_hashed(m, &keys[j * 4..j * 4 + 4], *h);
        hits += u64::from(found);
    }
    hits
}

#[inline(never)]
fn flag_free_pass(colt: &Colt, m: &Map, keys: &[u64], hashes: &[u64]) -> u64 {
    let mut hits = 0u64;
    for (j, h) in hashes.iter().enumerate() {
        let (found, _) = flag_free_probe4(colt, m, &keys[j * 4..j * 4 + 4], *h);
        hits += u64::from(found);
    }
    hits
}

#[inline(never)]
fn stream_foreign(buf: &[u64]) -> u64 {
    let mut acc = 0u64;
    for &w in buf {
        acc = acc.wrapping_add(w);
    }
    acc
}

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

#[test]
#[ignore = "microbench pin: run explicitly with --ignored"]
fn flag_free_compare_twin_at_displaced_and_resident_probes() {
    const PROBES: usize = 100_000;
    const REPS: u64 = 20;
    let schema = schema4();

    // DRAM-tier displaced: 400k keys → 131072 buckets × 320 B ≈ 42 MB

    let regimes: &[(&str, u64, bool)] = &[
        ("displaced-dram", 400_000, true),
        ("l2-resident", 20_000, false),
    ];

    let foreign: Vec<u64> = (0..12_000_000u64).map(mix).collect();

    for &(regime, n_rows, displace) in regimes {
        let view = view4_of(&schema, n_rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0, 1, 2, 3]]);
        let root = Colt::root();
        colt.ensure_forced(root, 0).expect("force");
        let m = colt.maps[0];
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let slab_mb = (m.nbuckets * m.stride() * 8) as f64 / 1e6;

        let mut keys = Vec::new();
        let mut hashes = Vec::new();
        for &hit_pct in &[10u64, 50, 90] {
            let mut ratios = Vec::new();
            for rep in 0..REPS {
                gen_probe_keys(rep, hit_pct, n_rows, PROBES, &mut keys, &mut hashes);
                let mut ns = [0f64; 2];

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
                ratios.push(ns[0] / ns[1]);
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

#[test]
#[ignore = "microbench pin: run explicitly with --ignored"]
fn bucketized_force_stays_at_parity_with_the_linear_build() {
    fn linear_build(keys: &[u64]) -> (Vec<u8>, Vec<u64>) {
        let mut capacity = ((keys.len() / 8).max(16)).next_power_of_two();
        let mut ctrl = vec![0u8; capacity];
        let mut rows = vec![0u64; capacity * 2];
        let mut len = 0usize;
        let mut dense: Vec<u32> = Vec::with_capacity(keys.len());
        for (pos, &k) in keys.iter().enumerate() {
            if (len + 1) * 4 >= capacity * 3 {
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
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }
        std::hint::black_box(len);
        (ctrl, rows)
    }

    let schema = schema();
    let n = std::hint::black_box(100_000u64);
    let rows: Vec<(u64, u64)> = (0..n)
        .map(|i| (i.wrapping_mul(0x9E37_79B9_7F4A_7C15), i))
        .collect();
    let view = view_of(&schema, &rows);
    let decoded: Vec<u64> = view.column_words(0).to_vec();

    let mut bucket_best = std::time::Duration::MAX;
    let mut linear_best = std::time::Duration::MAX;
    for _ in 0..5 {
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let root = Colt::root();
        let start = std::time::Instant::now();
        colt.ensure_forced(root, 0).expect("force");
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
    )]
    let ratio = linear_ns as f64 / bucket_ns as f64;
    println!("force build: bucket {bucket_ns} ns, linear-ref {linear_ns} ns, ratio {ratio:.2}");
    assert!(
        linear_ns * 10 >= bucket_ns * 9,
        "bucketized build must stay within 1.11× of the linear reference: {bucket_ns} vs {linear_ns} ns"
    );
}

fn force_and_iterate(colt: &mut Colt) -> u64 {
    let root = Colt::root();
    colt.ensure_forced(root, 0).expect("force");
    let mut keys = [0u64; 64];
    let mut children = [Cursor::Row(0); 64];
    let mut kids: Vec<Cursor> = Vec::new();
    let mut token = BatchToken::default();
    loop {
        let (n, next) = colt
            .iter_batch(root, 0, token, &mut keys, &mut children, 64)
            .expect("iter");
        if n == 0 {
            break;
        }
        kids.extend_from_slice(&children[..n]);
        token = next;
    }
    let mut sum = 0u64;
    for &child in &kids {
        let mut token = BatchToken::default();
        loop {
            let (n, next) = colt
                .iter_batch(child, 1, token, &mut keys, &mut children, 64)
                .expect("iter");
            if n == 0 {
                break;
            }
            for &w in &keys[..n] {
                sum = sum.wrapping_add(w);
            }
            token = next;
        }
    }
    sum
}

#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn chunk_geometry_force_iterate_ab() {
    use crate::image::TransientImage;
    use bumbledb_theory::schema::ValueType;
    let n: u64 = 1 << 18;
    for &fanout in &[2u64, 4, 8, 64] {
        let words: Vec<[u64; 2]> = (0..n).map(|i| [i / fanout, i]).collect();
        let mut slot = TransientImage::default();
        let image = slot.refill(
            &[ValueType::U64, ValueType::U64],
            words.len(),
            words.iter().map(|row| &row[..]),
        );
        let mut graded_best = std::time::Duration::MAX;
        let mut flat_best = std::time::Duration::MAX;
        let mut footprints = (0usize, 0usize);
        let mut sums = (0u64, 0u64);
        for _ in 0..5 {
            for (arm, cap) in [(0usize, 8u8), (1, 64)] {
                let mut colt = Colt::new(all(&image), &[], vec![vec![0], vec![1]]);
                colt.set_first_chunk_cap(cap);
                let start = std::time::Instant::now();
                let sum = force_and_iterate(&mut colt);
                let elapsed = start.elapsed();
                std::hint::black_box(sum);
                if arm == 0 {
                    graded_best = graded_best.min(elapsed);
                    footprints.0 = colt.chunk_footprint_bytes();
                    sums.0 = sum;
                } else {
                    flat_best = flat_best.min(elapsed);
                    footprints.1 = colt.chunk_footprint_bytes();
                    sums.1 = sum;
                }
            }
        }
        assert_eq!(sums.0, sums.1, "the geometries agree bit for bit");
        let (g_ns, f_ns) = (graded_best.as_nanos() as f64, flat_best.as_nanos() as f64);
        println!(
            "fanout {fanout}: graded {g_ns:.0} ns / flat {f_ns:.0} ns (ratio {:.2}), \
             footprint graded {} B / flat {} B (ratio {:.2})",
            g_ns / f_ns,
            footprints.0,
            footprints.1,
            footprints.0 as f64 / footprints.1 as f64,
        );
    }
}
