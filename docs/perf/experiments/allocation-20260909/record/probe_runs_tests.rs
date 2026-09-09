//! Prepared independently of the frozen-source diagnostic build. Move into
//! exec/run/tests only after that diagnostic completes; not yet compiled.
use super::*;

fn probe_view(width: usize) -> crate::image::view::View {
    let rows: Vec<Vec<u64>> = [(0, 50), (1, 14), (2, 1), (3, 130)]
        .into_iter()
        .flat_map(|(group, count)| {
            (0..count).map(move |row| {
                std::iter::once(group)
                    .chain((0..width).map(|word| probe_word(row / 2, word)))
                    .chain(std::iter::once(row))
                    .collect()
            })
        })
        .collect();
    let mut image = crate::image::TransientImage::default();
    let image = image.refill(
        &vec![bumbledb_theory::schema::ValueType::U64; width + 2],
        rows.len(),
        &crate::image::test_generation(),
        rows.iter().map(Vec::as_slice),
    );
    apply(&image, &[], &[], Vec::new(), image.generation().text_eq()).unwrap()
}

fn probe_word(value: u64, word: usize) -> u64 {
    value
        .wrapping_mul(31)
        .rotate_left(u32::try_from(word * 7).unwrap())
}

fn sibling_runs<const K: usize>(width: usize) {
    let schema = schema(1);
    let normalized = normalized(vec![occurrence(0, 0, &[(0, 0)])], vec![]);
    let plan = planned(&normalized, &schema, &[0]);
    let sentinel = Cursor::Row(u32::MAX);
    let view = probe_view(width);
    let new_colt = || {
        Colt::new(
            view.clone(),
            &[],
            vec![vec![0], (1..=width).collect(), vec![width + 1]],
        )
    };
    for len in [0, 1, 7, 64] {
        for alternating in [false, true] {
            for children in [false, true] {
                for carried in [false, true] {
                    let mut colt = new_colt();
                    let mut reference = new_colt();
                    let cursors: Vec<_> = (0..4)
                        .map(|group| {
                            let cursor = colt.get(Colt::root(), 0, &[group]).unwrap();
                            assert_eq!(reference.get(Colt::root(), 0, &[group]), Some(cursor));
                            cursor
                        })
                        .chain([Cursor::Row(0), Cursor::Row(1)])
                        .collect();
                    let mut scratch = NodeScratch {
                        survivors: (0..len)
                            .map(|k| u32::try_from(((k * 17) % len) * 2).unwrap())
                            .collect(),
                        parents: vec![0; len * 2 + 1],
                        pending_cursors: vec![sentinel; len * 2],
                        children: vec![vec![sentinel; if children { len * 2 + 1 } else { 0 }]],
                        mask: vec![9; len + 3],
                        ..NodeScratch::default()
                    };
                    for (k, &element) in scratch.survivors.iter().enumerate() {
                        scratch.parents[element as usize] = u32::try_from(k).unwrap();
                        let group = if alternating { k } else { k / 7 };
                        scratch.pending_cursors[k * 2 + 1] = cursors[group % cursors.len()];
                        let value = [0, 1, 3, 18, 999][k % 5];
                        let key: Vec<_> = (0..width).map(|word| probe_word(value, word)).collect();
                        scratch.hashes.push(crate::exec::colt::hash_key(&key));
                        scratch.probe_keys.extend(key);
                    }
                    let mut expected_mask = scratch.mask.clone();
                    let mut expected_children = scratch.children[0].clone();
                    for (k, &element) in scratch.survivors.iter().enumerate() {
                        let cursor = if carried {
                            scratch.pending_cursors[k * 2 + 1]
                        } else {
                            cursors[0]
                        };
                        let hit = reference
                            .get_prehashed(
                                cursor,
                                1,
                                &scratch.probe_keys[k * width..(k + 1) * width],
                                scratch.hashes[k],
                            )
                            .unwrap();
                        expected_mask[k] = u8::from(hit.is_some());
                        if children && let Some(child) = hit {
                            expected_children[element as usize] = child;
                        }
                    }
                    let survivors = scratch.survivors.clone();
                    let mut executor = Executor::new(&plan);
                    // Cold runs force different maps while previous maps remain
                    // readable. The second pass must reuse all retained owners.
                    for warm in [false, true] {
                        scratch.mask.fill(9);
                        scratch.children[0].fill(sentinel);
                        #[cfg(feature = "alloc-counter")]
                        let before = crate::alloc_counter::snapshot().window;
                        executor.probe_sibling_batch::<K, _>(
                            &mut scratch,
                            &mut colt,
                            0,
                            0,
                            1,
                            carried.then_some(1),
                            2,
                            cursors[0],
                            width,
                            &mut NoopCounters,
                        );
                        #[cfg(feature = "alloc-counter")]
                        if warm {
                            assert_eq!(crate::alloc_counter::snapshot().window, before);
                        }
                        assert_eq!(scratch.mask, expected_mask, "width={width} warm={warm}");
                        assert_eq!(scratch.children[0], expected_children);
                        assert_eq!(scratch.survivors, survivors);
                        assert!(matches!(executor.drive_state, DriveState::Running));
                    }
                }
            }
        }
    }
}

#[test]
fn store_free_sibling_runs_match_scalar_probes_and_reuse_pools() {
    for width in [0, 1, 2, 3, 4, 5, 8] {
        sibling_runs::<0>(width);
    }
    sibling_runs::<1>(1);
    sibling_runs::<2>(2);
    sibling_runs::<3>(3);
    sibling_runs::<4>(4);
}
