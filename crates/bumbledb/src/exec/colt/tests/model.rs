use super::*;

#[test]
fn packed_children_preserve_the_full_payload_without_a_second_cursor_type() {
    for payload in [0, 1, 255, 65_535, 1 << 31, u32::MAX - 1, u32::MAX] {
        for cursor in [Cursor::Row(payload), Cursor::Node(NodeRef(payload))] {
            let word = pack_child(cursor);
            assert_eq!(unpack_child(word), cursor);
            assert_eq!(word & u64::from(u32::MAX), u64::from(payload));
            assert_eq!(word & !(CHILD_NODE_TAG | u64::from(u32::MAX)), 0);
            assert_eq!(
                word & CHILD_NODE_TAG != 0,
                matches!(cursor, Cursor::Node(_))
            );
        }
    }
}

fn wide_iteration_image(width: usize) -> Arc<crate::image::RelationImage> {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: (0..=width)
                .map(|column| FieldDescriptor {
                    name: format!("c{column}").into(),
                    value_type: ValueType::U64,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    // Triples promote children into nodes; the final row stays a singleton.
    // Nonempty keys force map growth and span multiple lookahead/batch windows.
    let facts = (0..514u64)
        .map(|row| {
            (0..width)
                .map(|column| Value::U64((row / 3).rotate_left(u32::try_from(column * 9).unwrap())))
                .chain(std::iter::once(Value::U64(row)))
                .collect()
        })
        .collect();
    TestSource::new(&schema, &[(R, facts)])
        .image_with_cache(R)
        .1
}

fn drain_guarded_batches(
    colt: &mut Colt,
    cursor: Cursor,
    level: usize,
    size: usize,
) -> Vec<(Vec<u64>, Cursor)> {
    let width = colt.arity(level);
    let sentinel = Cursor::Row(u32::MAX);
    let mut keys = vec![u64::MAX; width * size + 5];
    let mut keys_only = keys.clone();
    let mut children = vec![sentinel; size + 3];
    let empty_keys = colt
        .iter_keys_batch(cursor, level, BatchToken::default(), &mut keys_only, 0)
        .expect("zero-sized key batch");
    let (empty, mut token) = colt
        .iter_batch(
            cursor,
            level,
            BatchToken::default(),
            &mut keys,
            &mut children,
            0,
        )
        .expect("zero-sized batch");
    assert_eq!(empty_keys, (empty, token));
    assert_eq!(empty, 0);
    assert_eq!(keys_only, keys);
    assert!(keys.iter().all(|&word| word == u64::MAX));
    assert!(children.iter().all(|&child| child == sentinel));
    let mut observed = Vec::new();
    loop {
        keys.fill(u64::MAX);
        keys_only.fill(u64::MAX);
        children.fill(sentinel);
        let key_batch = colt
            .iter_keys_batch(cursor, level, token, &mut keys_only, size)
            .expect("key batch");
        let (n, next) = colt
            .iter_batch(cursor, level, token, &mut keys, &mut children, size)
            .expect("key and child batch");
        assert_eq!(key_batch, (n, next), "identical count and resume token");
        assert_eq!(keys_only, keys, "identical keys and untouched guard words");
        assert!(n <= size);
        assert!(keys[n * width..].iter().all(|&word| word == u64::MAX));
        assert!(children[n..].iter().all(|&child| child == sentinel));
        for (index, &child) in children[..n].iter().enumerate() {
            let key = keys[index * width..(index + 1) * width].to_vec();
            observed.push((key, child));
        }
        if n == 0 {
            assert_eq!(
                colt.iter_keys_batch(cursor, level, next, &mut keys_only, size)
                    .unwrap(),
                (0, next),
                "exhaustion remains stable"
            );
            break;
        }
        token = next;
    }
    observed
}

#[test]
fn fixed_and_wide_map_iteration_preserves_keys_children_and_batch_boundaries() {
    for width in [0, 1, 2, 3, 4, 5, 8] {
        let image = wide_iteration_image(width);
        let levels = vec![(0..width).collect::<Vec<_>>(), vec![width]];
        let mut colt = Colt::new(all(&image), &[], levels.clone());
        colt.ensure_forced(Colt::root(), 0).expect("force root");
        let baseline = drain(&mut colt, Colt::root(), 0);
        let mut model: HashMap<Vec<u64>, Vec<u64>> = HashMap::new();
        for (position, &value) in image.column_words(width).iter().enumerate() {
            let key = (0..width)
                .map(|column| image.column_words(column)[position])
                .collect();
            model.entry(key).or_default().push(value);
        }
        assert_eq!(baseline.len(), model.len());
        let mut actual = HashMap::new();
        for (key, child) in &baseline {
            assert_eq!(colt.get(Colt::root(), 0, key), Some(*child));
            let mut values: Vec<_> = drain(&mut colt, *child, 1)
                .into_iter()
                .map(|(row, _)| row[0])
                .collect();
            values.sort_unstable();
            assert!(actual.insert(key.clone(), values).is_none(), "key repeated");
        }
        for values in model.values_mut() {
            values.sort_unstable();
        }
        assert_eq!(actual, model);
        if width > 0 {
            assert!(
                baseline
                    .iter()
                    .any(|(_, child)| matches!(child, Cursor::Row(_)))
            );
            assert!(
                baseline
                    .iter()
                    .any(|(_, child)| matches!(child, Cursor::Node(_)))
            );
            assert!(
                colt.forced_capacity(Colt::root()).unwrap()
                    > super::super::force::force_nbuckets(514) * 8
            );
        }
        let mut cloned = Colt::new(all(&image), &[], levels);
        drop(
            cloned
                .clone_bound_from(&colt, Vec::new())
                .expect("clone forced pools"),
        );
        for size in [1, 7, 8, 9, 127, 128, 129, 1024] {
            assert_eq!(
                drain_guarded_batches(&mut colt, Colt::root(), 0, size),
                baseline,
                "width {width}, batch {size}"
            );
            assert_eq!(
                drain_guarded_batches(&mut cloned, Colt::root(), 0, size),
                baseline,
                "cloned width {width}, batch {size}"
            );
        }
    }
}

#[test]
fn batch_width_probes_match_dynamic_through_selection_force_and_pinned_rows() {
    fn check<const K: usize>(width: usize) {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: (0..width + 2)
                    .map(|column| FieldDescriptor {
                        name: format!("c{column}").into(),
                        value_type: ValueType::U64,
                    })
                    .collect(),
            }],
            statements: vec![],
        }
        .validate()
        .expect("valid fixture");
        let tuple = |value: u64| -> Vec<u64> {
            (0..width)
                .map(|column| {
                    value
                        .wrapping_mul(31)
                        .rotate_left(u32::try_from(column).unwrap())
                })
                .collect()
        };
        let facts: Vec<Vec<Value>> = (0..8u64)
            .flat_map(|group| {
                let tuple = &tuple;
                (0..8u64).map(move |row| {
                    std::iter::once(Value::U64(group))
                        .chain(tuple(row / 2).into_iter().map(Value::U64))
                        .chain(std::iter::once(Value::U64(row)))
                        .collect()
                })
            })
            .collect();
        let source = TestSource::new(&schema, &[(R, facts)]);
        let (_cache, image) = source.image_with_cache(R);
        let columns: Vec<usize> = (1..=width).collect();
        let mut fixed = Colt::new(all(&image), &scalars(&[0]), vec![columns.clone()]);
        let mut dynamic = Colt::new(all(&image), &scalars(&[0]), vec![columns]);
        for group in [0, 3, 1, 7, 3, 0] {
            let selected = fixed.select(&[vec![group]]).unwrap().unwrap();
            assert_eq!(dynamic.select(&[vec![group]]).unwrap(), Some(selected));
            for value in [0, 1, 99, 2, 3, 0] {
                let key = tuple(value);
                let hash = hash_key(&key);
                let got = fixed
                    .get_prehashed_width::<K>(selected, 0, &key, hash)
                    .unwrap();
                let expected = dynamic.get_prehashed(selected, 0, &key, hash).unwrap();
                assert_eq!(got, expected);
                assert_eq!(fixed.ctrl, dynamic.ctrl);
                assert_eq!(fixed.buckets, dynamic.buckets);
                assert_eq!(fixed.dense, dynamic.dense);
                assert_eq!(fixed.chunk_positions, dynamic.chunk_positions);
                assert_eq!(fixed.watermark(), dynamic.watermark());
            }
        }
        let key: Vec<_> = (1..=width)
            .map(|column| image.column_words(column)[0])
            .collect();
        for key in [key, tuple(99)] {
            let hash = hash_key(&key);
            assert_eq!(
                fixed
                    .get_prehashed_width::<K>(Cursor::Row(0), 0, &key, hash)
                    .unwrap(),
                dynamic
                    .get_prehashed(Cursor::Row(0), 0, &key, hash)
                    .unwrap(),
            );
        }
    }

    check::<0>(0);
    check::<1>(1);
    check::<2>(2);
    check::<3>(3);
    check::<4>(4);
    check::<0>(5);
}

#[test]
fn bucket_probes_match_the_model_under_adversarial_keys() {
    let schema = schema();

    let mut rows: Vec<(u64, u64)> = Vec::new();
    for i in 0..400u64 {
        let key = (i % 97) << 8;
        rows.push((key, i));
    }
    rows.sort_unstable();
    rows.dedup();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    colt.ensure_forced(root, 0).expect("force");

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

    for i in 0..97u64 {
        let absent = (i << 8) | 1;
        assert!(colt.get(root, 0, &[absent]).is_none(), "key {absent}");
    }
}

#[test]
fn hoisted_gathers_match_the_per_position_reference() {
    let mut rows: Vec<(u64, u64, bool)> = (0..200u64)
        .map(|i| (if i % 3 == 0 { 0 } else { i % 7 }, i * 31 % 191, i % 2 == 0))
        .collect();
    rows.sort_unstable();
    rows.dedup();
    let words: Vec<[u64; 3]> = rows.iter().map(|&(k, v, b)| [k, v, u64::from(b)]).collect();
    let mut transient = crate::image::TransientImage::default();
    let image = transient.refill(
        &[ValueType::U64, ValueType::U64, ValueType::Bool],
        words.len(),
        &crate::image::test_generation(),
        words.iter().map(|row| &row[..]),
    );

    let k_col: Vec<u64> = image.column_words(0).to_vec();
    let v_col: Vec<u64> = image.column_words(1).to_vec();
    let b_col: Vec<u64> = image
        .column_bytes(2)
        .iter()
        .map(|&b| u64::from(b))
        .collect();
    let n_rows = k_col.len();
    assert_eq!(n_rows, rows.len());

    for &size in &[1usize, 3, 8, 64, 128] {
        let mut colt = Colt::new(all(&image), &[], vec![vec![0, 2]]);
        let got = drain_guarded_batches(&mut colt, Colt::root(), 0, size);
        let expected: Vec<(Vec<u64>, Cursor)> = (0..n_rows)
            .map(|pos| {
                (
                    vec![k_col[pos], b_col[pos]],
                    Cursor::Row(u32::try_from(pos).expect("small")),
                )
            })
            .collect();
        assert_eq!(got, expected, "identity root, batch {size}");

        let positions: Vec<_> = (0..n_rows)
            .step_by(3)
            .map(|pos| u32::try_from(pos).unwrap())
            .collect();
        colt.reset(View::Bound(BoundView::Survivors {
            image: Arc::clone(&image),
            positions,
        }));
        assert_eq!(
            drain_guarded_batches(&mut colt, Colt::root(), 0, size),
            expected.iter().step_by(3).cloned().collect::<Vec<_>>(),
            "survivor root, batch {size}"
        );
        assert_eq!(
            drain_guarded_batches(&mut colt, Cursor::Row(0), 0, size),
            expected[..1],
            "pinned row, batch {size}"
        );
        colt.reset(View::Bound(BoundView::Survivors {
            image: Arc::clone(&image),
            positions: Vec::new(),
        }));
        assert!(drain_guarded_batches(&mut colt, Colt::root(), 0, size).is_empty());

        let mut gate = Colt::new(all(&image), &[], vec![vec![]]);
        let raw = drain_guarded_batches(&mut gate, Colt::root(), 0, size);
        assert_eq!(
            raw.len(),
            n_rows,
            "zero-width positions retain multiplicity"
        );
        assert!(raw.iter().all(|(key, _)| key.is_empty()));
        gate.force_distinct_iteration(Colt::root(), 0).unwrap();
        assert_eq!(
            drain_guarded_batches(&mut gate, Colt::root(), 0, size).len(),
            1,
            "only an explicit distinct force removes projected duplicates"
        );

        let mut colt = Colt::new(all(&image), &[], vec![vec![0], vec![1, 2]]);
        for key in 0..7u64 {
            let Some(child) = colt.get(Colt::root(), 0, &[key]) else {
                continue;
            };
            let got = drain_guarded_batches(&mut colt, child, 1, size);
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
    let schema = schema();

    let mut rows: Vec<(u64, u64)> = (0..2_000u64).map(|i| (i % 17, i)).collect();
    rows.extend((100..110u64).map(|k| (k, k * 1000)));
    let view = view_of(&schema, &rows);
    let mut oracle: HashMap<u64, Vec<u64>> = HashMap::new();
    for (k, v) in &rows {
        oracle.entry(*k).or_default().push(*v);
    }

    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();

    let entries = drain(&mut colt, root, 0);
    assert_eq!(entries.len(), oracle.len());
    assert!(matches!(
        colt.key_count(root),
        KeyCount::Exact(n) if n == oracle.len() as u64
    ));
    for (key, child) in entries {
        let expected = &oracle[&key[0]];

        let got = colt.get(root, 0, &key).expect("iterated key resolves");
        assert_eq!(got, child);

        let mut values: Vec<u64> = drain(&mut colt, child, 1)
            .into_iter()
            .map(|(k, _)| k[0])
            .collect();
        values.sort_unstable();
        let mut want = expected.clone();
        want.sort_unstable();
        assert_eq!(values, want, "key {}", key[0]);
    }

    assert_eq!(colt.get(root, 0, &[9999]), None);
}
