use super::*;

/// PRD 07 (docs/perf/): the ctrl-gated bucket probe is behavior-
/// identical to a model across adversarial keys (equal low bits —
/// same slot, different tags), probe hits AND misses, singleton
/// upgrades, and growth across the 75% boundary.
#[test]
fn bucket_probes_match_the_model_under_adversarial_keys() {
    let dir = TempDir::new("colt-bucket-model");
    let schema = schema();
    // Keys collide mod any small capacity (equal low 8 bits) and
    // repeat (singleton -> chunk upgrades); enough distinct keys to
    // force several rehash doubles from the /8 initial guess.
    let mut rows: Vec<(u64, u64)> = Vec::new();
    for i in 0..400u64 {
        let key = (i % 97) << 8;
        rows.push((key, i));
    }
    rows.sort_unstable();
    rows.dedup();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    colt.ensure_forced(root, 0);

    // Model: key -> positions (image order).
    let k_col: Vec<u64> = view.column_words(0).to_vec();
    let mut model: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (pos, k) in k_col.iter().enumerate() {
        model
            .entry(*k)
            .or_default()
            .push(u32::try_from(pos).expect("small"));
    }
    for (key, positions) in &model {
        let child = colt.get(root, 0, &[*key]).expect("present key probes");
        let got: Vec<u32> = drain(&mut colt, child, 1)
            .into_iter()
            .map(|(_, c)| match c {
                Cursor::Row(p) => p,
                Cursor::Node(_) => unreachable!("suffix children pin rows"),
            })
            .collect();
        assert_eq!(&got, positions, "key {key}");
    }
    // Misses: same low bits as present keys, absent values.
    for i in 0..97u64 {
        let absent = (i << 8) | 1;
        assert!(colt.get(root, 0, &[absent]).is_none(), "key {absent}");
    }
}

/// PRD 04 (docs/perf/): the column-hoisted unchecked gathers are
/// bit-identical to a first-principles per-position reference, across
/// word and byte columns, the identity (all-rows) root, chunked
/// children, and resume-token splits at every batch size.
#[test]
#[allow(clippy::too_many_lines)] // one fixture, five batch sizes, two node shapes
fn hoisted_gathers_match_the_per_position_reference() {
    let dir = TempDir::new("colt-hoisted-gather");
    // R(k u64, v u64, b bool): a byte-backed column beside the words.
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "k".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "b".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
            ],
            constraints: vec![],
        }],
    }
    .validate()
    .expect("valid fixture");

    // Skewed keys force multi-chunk children (k=0 holds >64 rows).
    let mut rows: Vec<(u64, u64, bool)> = (0..200u64)
        .map(|i| (if i % 3 == 0 { 0 } else { i % 7 }, i * 31 % 191, i % 2 == 0))
        .collect();
    rows.sort_unstable();
    rows.dedup();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let txn0 = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (k, v, b) in &rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(*k), ValueRef::U64(*v), ValueRef::Bool(*b)],
            schema.relation(R).layout(),
            &mut bytes,
        );
        delta.insert(&txn0, R, &bytes).expect("insert");
    }
    drop(txn0);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let image = crate::image::build(&txn, &schema, R).expect("build");
    // The reference reads the image columns per position — the exact
    // access the hoisted gather replaces; no assumption about the
    // image's position order.
    let k_col: Vec<u64> = image.column_words(0).to_vec();
    let v_col: Vec<u64> = image.column_words(1).to_vec();
    let b_col: Vec<u64> = image
        .column_bytes(2)
        .iter()
        .map(|&b| u64::from(b))
        .collect();
    let n_rows = k_col.len();
    assert_eq!(n_rows, rows.len());

    let drain_at = |colt: &mut Colt, cursor: Cursor, level: usize, size: usize| {
        let arity = colt.arity(level);
        let mut keys = vec![0u64; size * arity.max(1)];
        let mut children = vec![Cursor::Row(0); size];
        let mut token = BatchToken::default();
        let mut out = Vec::new();
        loop {
            let (n, next) =
                colt.iter_batch(cursor, level, token, &mut keys, &mut children, size);
            if n == 0 {
                break;
            }
            for i in 0..n {
                out.push((keys[i * arity..(i + 1) * arity].to_vec(), children[i]));
            }
            token = next;
        }
        out
    };

    for &size in &[1usize, 3, 8, 64, 128] {
        // Identity root suffix over (k, b): word + byte columns.
        let mut colt = Colt::new(apply(&image, &[], &[], Vec::new()), &[], vec![vec![0, 2]]);
        let got = drain_at(&mut colt, Colt::root(), 0, size);
        let expected: Vec<(Vec<u64>, Cursor)> = (0..n_rows)
            .map(|pos| {
                (
                    vec![k_col[pos], b_col[pos]],
                    Cursor::Row(u32::try_from(pos).expect("small")),
                )
            })
            .collect();
        assert_eq!(got, expected, "identity root, batch {size}");

        // Chunked child suffix over (v, b) under each key.
        let mut colt = Colt::new(
            apply(&image, &[], &[], Vec::new()),
            &[],
            vec![vec![0], vec![1, 2]],
        );
        for key in 0..7u64 {
            let Some(child) = colt.get(Colt::root(), 0, &[key]) else {
                continue;
            };
            let got = drain_at(&mut colt, child, 1, size);
            let expected: Vec<(Vec<u64>, Cursor)> = (0..n_rows)
                .filter(|&pos| k_col[pos] == key)
                .map(|pos| {
                    (
                        vec![v_col[pos], b_col[pos]],
                        Cursor::Row(u32::try_from(pos).expect("small")),
                    )
                })
                .collect();
            assert_eq!(got, expected, "key {key} suffix, batch {size}");
        }
    }
}

#[test]
fn get_and_iter_agree_with_a_naive_oracle() {
    let dir = TempDir::new("colt-oracle");
    let schema = schema();
    // Duplicate-heavy: keys follow i % 17, some singleton keys on top.
    let mut rows: Vec<(u64, u64)> = (0..2_000u64).map(|i| (i % 17, i)).collect();
    rows.extend((100..110u64).map(|k| (k, k * 1000)));
    let view = view_of(&dir, &schema, &rows);
    let mut oracle: HashMap<u64, Vec<u64>> = HashMap::new();
    for (k, v) in &rows {
        oracle.entry(*k).or_default().push(*v);
    }

    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    // Root iteration (non-suffix -> forces): keys match the oracle's.
    let entries = drain(&mut colt, root, 0);
    assert_eq!(entries.len(), oracle.len());
    assert!(matches!(
        colt.key_count(root),
        KeyCount::Exact(n) if n == oracle.len() as u64
    ));
    for (key, child) in entries {
        let expected = &oracle[&key[0]];
        // get() agrees with the iterated child.
        let got = colt.get(root, 0, &key).expect("iterated key resolves");
        assert_eq!(got, child);
        // Level-1 values match the oracle multiset.
        let mut values: Vec<u64> = drain(&mut colt, child, 1)
            .into_iter()
            .map(|(k, _)| k[0])
            .collect();
        values.sort_unstable();
        let mut want = expected.clone();
        want.sort_unstable();
        assert_eq!(values, want, "key {}", key[0]);
    }
    // Missing keys miss.
    assert_eq!(colt.get(root, 0, &[9999]), None);
}
