use super::super::key::KeyScratch;
use super::*;

#[test]
fn arena_creates_all_node_data_variants() {
    let mut arena = ColtArena::new();
    let range = arena.add_range_node(0, 10);
    let singleton = arena.add_singleton_node(7);
    let offsets = arena.add_offsets_node(&[2, 4, 8]);
    let map = arena.add_map_placeholder_node();

    assert_eq!(
        arena.node(range).map(|node| node.data),
        Some(NodeData::Range { start: 0, len: 10 })
    );
    assert_eq!(
        arena.node(singleton).map(|node| node.data),
        Some(NodeData::Singleton { offset: 7 })
    );
    assert_eq!(
        arena.node(offsets).map(|node| node.data),
        Some(NodeData::Offsets(OffsetRange { start: 0, len: 3 }))
    );
    assert_eq!(
        arena.node(map).map(|node| node.data),
        Some(NodeData::Map(ColtMapId(0)))
    );
}

#[test]
fn arena_node_ids_remain_stable_after_insertions() {
    let mut arena = ColtArena::new();
    let first = arena.add_singleton_node(1);
    let second = arena.add_range_node(0, 2);

    for offset in 10..100 {
        let _ = arena.add_singleton_node(offset);
    }

    assert_eq!(
        arena.node(first).map(|node| node.data),
        Some(NodeData::Singleton { offset: 1 })
    );
    assert_eq!(
        arena.node(second).map(|node| node.data),
        Some(NodeData::Range { start: 0, len: 2 })
    );
}

#[test]
fn arena_offset_ranges_read_back_exact_offsets() {
    let mut arena = ColtArena::new();
    let first = arena.append_offsets(&[1, 3, 5]);
    let second = arena.append_offsets(&[8, 13]);

    assert_eq!(arena.offsets(first), &[1, 3, 5]);
    assert_eq!(arena.offsets(second), &[8, 13]);
}

#[test]
fn arena_iterates_range_singleton_and_pooled_offsets() {
    let mut arena = ColtArena::new();
    let pooled = arena.child_offsets_data(&[5, 8, 13]);

    assert_eq!(
        arena
            .iter_offsets(NodeData::Range { start: 3, len: 4 })
            .collect::<Vec<_>>(),
        vec![3, 4, 5, 6]
    );
    assert_eq!(
        arena
            .iter_offsets(NodeData::Singleton { offset: 42 })
            .collect::<Vec<_>>(),
        vec![42]
    );
    assert_eq!(
        arena.iter_offsets(pooled).collect::<Vec<_>>(),
        vec![5, 8, 13]
    );
}

#[test]
fn arena_duplicate_heavy_children_use_singletons_without_pool_offsets() {
    let mut arena = ColtArena::new();
    let child = arena.child_offsets_data(&[17]);

    assert_eq!(child, NodeData::Singleton { offset: 17 });
    assert_eq!(arena.offset_pool_len(), 0);
    assert_eq!(arena.iter_offsets(child).collect::<Vec<_>>(), vec![17]);
}

#[test]
fn arena_many_offset_child_uses_one_offset_pool_range() {
    let mut arena = ColtArena::new();
    let child = arena.child_offsets_data(&[1, 2, 3, 5, 8]);

    assert_eq!(child, NodeData::Offsets(OffsetRange { start: 0, len: 5 }));
    assert_eq!(arena.offset_pool_len(), 5);
    assert_eq!(
        arena.iter_offsets(child).collect::<Vec<_>>(),
        vec![1, 2, 3, 5, 8]
    );
}

#[test]
fn arena_full_unfiltered_source_uses_implicit_range() {
    let mut arena = ColtArena::new();
    let source = arena.add_full_source_node(4);

    assert_eq!(arena.offset_pool_len(), 0);
    assert_eq!(
        arena.node(source).map(|node| node.data),
        Some(NodeData::Range { start: 0, len: 4 })
    );
    assert_eq!(
        arena
            .node(source)
            .map(|node| arena.iter_offsets(node.data).collect::<Vec<_>>()),
        Some(vec![0, 1, 2, 3])
    );
}

#[test]
fn arena_empty_ranges_and_singletons_are_distinct() {
    let mut arena = ColtArena::new();
    let empty = arena.add_offsets_node(&[]);
    let singleton = arena.add_singleton_node(0);

    assert_eq!(
        arena.node(empty).map(|node| node.data),
        Some(NodeData::Offsets(OffsetRange { start: 0, len: 0 }))
    );
    assert_eq!(
        arena.node(singleton).map(|node| node.data),
        Some(NodeData::Singleton { offset: 0 })
    );
    assert_ne!(
        arena.node(empty).map(|node| node.data),
        arena.node(singleton).map(|node| node.data)
    );
}

#[test]
fn arena_source_handle_is_compact_copy_state() {
    assert!(std::mem::size_of::<ArenaSourceHandle>() <= 24);
    assert!(std::mem::size_of::<ArenaSourceUndo>() <= 32);
}

#[test]
fn arena_source_store_replaces_and_restores_compact_handles() {
    let root = ArenaSourceHandle::new(ColtSourceId(0), ColtNodeId(0), SchemaVarsId(0));
    let child = ArenaSourceHandle::new(ColtSourceId(0), ColtNodeId(1), SchemaVarsId(1));
    let mut store = ArenaSourceStore::with_atom_count(1);
    store.set_initial(0, root);
    let mark = store.undo_mark();

    assert_eq!(store.source_for(0), Some(root));
    assert!(store.replace_source(0, child));
    assert_eq!(store.source_for(0), Some(child));
    store.restore_to(mark);
    assert_eq!(store.source_for(0), Some(root));
}

#[test]
fn arena_source_store_missing_atom_replacement_is_rejected() {
    let child = ArenaSourceHandle::new(ColtSourceId(0), ColtNodeId(1), SchemaVarsId(1));
    let mut store = ArenaSourceStore::with_atom_count(1);

    assert!(!store.replace_source(0, child));
    assert_eq!(store.source_for(0), None);
}

#[test]
fn arena_flat_map_inserts_distinct_keys_and_reuses_duplicates() {
    let mut arena = ColtArena::new();
    let map = arena.add_map_table(4, 16);
    let child_a = arena.add_singleton_node(1);
    let child_b = arena.add_singleton_node(2);
    let key_a = [1; 8];
    let key_b = [2; 8];

    assert_eq!(
        arena.insert_map_entry(map, KeyRef::new(&key_a), child_a),
        child_a
    );
    assert_eq!(
        arena.insert_map_entry(map, KeyRef::new(&key_a), child_b),
        child_a
    );
    assert_eq!(
        arena.insert_map_entry(map, KeyRef::new(&key_b), child_b),
        child_b
    );
    assert_eq!(arena.map_entry_count(map), 2);
    assert_eq!(arena.lookup_map(map, KeyRef::new(&key_a)), Some(child_a));
    assert_eq!(arena.lookup_map(map, KeyRef::new(&key_b)), Some(child_b));
}

#[test]
fn arena_flat_map_borrowed_lookup_is_allocation_bounded() {
    let mut arena = ColtArena::new();
    let map = arena.add_map_table(8, 8);
    let child = arena.add_singleton_node(1);
    let key = [3; 16];
    arena.insert_map_entry(map, KeyRef::new(&key), child);

    let alloc_calls = crate::diagnostics::with_allocation_tracking_for_test(|| {
        let start = crate::diagnostics::allocation_snapshot();
        for _ in 0..1000 {
            assert_eq!(arena.lookup_map(map, KeyRef::new(&key)), Some(child));
        }
        crate::diagnostics::allocation_delta(start, crate::diagnostics::allocation_snapshot())
            .alloc_calls
    });

    assert!(alloc_calls < 500);
}

#[test]
fn arena_flat_map_allocates_less_than_heap_tuple_map_pattern() {
    let flat_calls = crate::diagnostics::with_allocation_tracking_for_test(|| {
        let start = crate::diagnostics::allocation_snapshot();
        let mut arena = ColtArena::new();
        let map = arena.add_map_table(1024, 1024);
        for value in 0..1024u64 {
            let child = arena.add_singleton_node(value as u32);
            arena.insert_map_entry(map, KeyRef::new(&value.to_be_bytes()), child);
        }
        crate::diagnostics::allocation_delta(start, crate::diagnostics::allocation_snapshot())
            .alloc_calls
    });
    let heap_calls = crate::diagnostics::with_allocation_tracking_for_test(|| {
        let start = crate::diagnostics::allocation_snapshot();
        let mut map = std::collections::HashMap::with_capacity(1024);
        for value in 0..1024u64 {
            map.insert(
                crate::tuple::EncodedTuple::from_bytes(value.to_be_bytes().to_vec()),
                value,
            );
        }
        crate::diagnostics::allocation_delta(start, crate::diagnostics::allocation_snapshot())
            .alloc_calls
    });

    assert!(
        flat_calls * 2 < heap_calls,
        "flat map should allocate materially less than heap tuple map: flat={flat_calls} heap={heap_calls}"
    );
}

#[test]
fn arena_force_builds_flat_map_from_range_once() {
    let mut arena = ColtArena::new();
    let root = arena.add_range_node(0, 4);
    let map = arena.force_node_with_key_fn(root, |offset, key| {
        key.clear();
        key.extend_from_slice(&(offset % 2).to_be_bytes());
    });

    assert_eq!(
        arena.node(root).map(|node| node.data),
        Some(NodeData::Map(map))
    );
    assert_eq!(arena.map_entry_count(map), 2);
    let key = 1u32.to_be_bytes();
    let child = arena.lookup_map(map, KeyRef::new(&key));
    assert!(child.is_some());
    let Some(child) = child else { return };
    let child_data = arena.node(child);
    assert!(child_data.is_some());
    let Some(child_data) = child_data else { return };
    assert_eq!(
        arena.iter_offsets(child_data.data).collect::<Vec<_>>(),
        vec![1, 3]
    );

    let mut key_calls = 0usize;
    let again = arena.force_node_with_key_fn(root, |_offset, _key| key_calls += 1);
    assert_eq!(again, map);
    assert_eq!(key_calls, 0);
}

#[test]
fn arena_force_second_level_only_forces_selected_child() {
    let mut arena = ColtArena::new();
    let root = arena.add_range_node(0, 4);
    let root_map = arena.force_node_with_key_fn(root, |offset, key| {
        key.clear();
        key.extend_from_slice(&(offset / 2).to_be_bytes());
    });
    let selected_key = 0u32.to_be_bytes();
    let other_key = 1u32.to_be_bytes();
    let selected = arena.lookup_map(root_map, KeyRef::new(&selected_key));
    let other = arena.lookup_map(root_map, KeyRef::new(&other_key));
    assert!(selected.is_some());
    assert!(other.is_some());
    let Some(selected) = selected else { return };
    let Some(other) = other else { return };

    let selected_map = arena.force_node_with_key_fn(selected, |offset, key| {
        key.clear();
        key.extend_from_slice(&offset.to_be_bytes());
    });

    assert_eq!(
        arena.node(selected).map(|node| node.data),
        Some(NodeData::Map(selected_map))
    );
    assert!(matches!(
        arena.node(other).map(|node| node.data),
        Some(NodeData::Offsets(_))
    ));
}

#[test]
fn arena_force_allocation_is_bounded_below_row_count_for_duplicates() {
    let alloc_calls = crate::diagnostics::with_allocation_tracking_for_test(|| {
        let start = crate::diagnostics::allocation_snapshot();
        let mut arena = ColtArena::new();
        let root = arena.add_range_node(0, 256);
        let map = arena.force_node_with_key_fn(root, |offset, key| {
            key.clear();
            key.extend_from_slice(&(offset / 64).to_be_bytes());
        });
        assert_eq!(arena.map_entry_count(map), 4);
        crate::diagnostics::allocation_delta(start, crate::diagnostics::allocation_snapshot())
            .alloc_calls
    });

    assert!(alloc_calls < 256);
}

#[test]
fn arena_streams_offsets_and_map_keys_without_materializing_all_items() {
    let mut arena = ColtArena::new();
    let offsets = arena.add_offsets_node(&[2, 4, 8]);
    let mut seen_offsets = Vec::new();
    arena
        .try_for_each_item::<()>(offsets, |item| {
            if let ArenaIterItem::Offset(offset) = item {
                seen_offsets.push(offset);
            }
            Ok(())
        })
        .unwrap_or(());
    assert_eq!(seen_offsets, vec![2, 4, 8]);

    let root = arena.add_range_node(0, 2);
    let map = arena.force_node_with_key_fn(root, |offset, key| {
        key.clear();
        key.extend_from_slice(&offset.to_be_bytes());
    });
    let mut key_count = 0;
    arena
        .try_for_each_item::<()>(root, |item| {
            if matches!(item, ArenaIterItem::Key(_)) {
                key_count += 1;
            }
            Ok(())
        })
        .unwrap_or(());
    assert_eq!(arena.map_entry_count(map), 2);
    assert_eq!(key_count, 2);
}

#[test]
fn arena_fill_offset_batch_is_bounded_by_requested_size() {
    let mut arena = ColtArena::new();
    let node = arena.add_range_node(0, 1000);
    let mut cursor = ArenaOffsetCursor::default();

    let alloc_calls = crate::diagnostics::with_allocation_tracking_for_test(|| {
        let start = crate::diagnostics::allocation_snapshot();
        let first = arena.fill_offset_batch(node, &mut cursor, 4);
        assert_eq!(first.offsets, vec![0, 1, 2, 3]);
        assert!(!first.exhausted);
        crate::diagnostics::allocation_delta(start, crate::diagnostics::allocation_snapshot())
            .alloc_calls
    });

    assert!(alloc_calls < 100);
    assert_eq!(cursor.position, 4);
}

#[test]
fn arena_fill_offset_batch_handles_partial_and_empty_sources() {
    let mut arena = ColtArena::new();
    let partial = arena.add_range_node(0, 5);
    let empty = arena.add_offsets_node(&[]);
    let mut cursor = ArenaOffsetCursor::default();

    assert_eq!(
        arena.fill_offset_batch(partial, &mut cursor, 4).offsets,
        vec![0, 1, 2, 3]
    );
    let final_batch = arena.fill_offset_batch(partial, &mut cursor, 4);
    assert_eq!(final_batch.offsets, vec![4]);
    assert!(final_batch.exhausted);

    let empty_batch = arena.fill_offset_batch(empty, &mut ArenaOffsetCursor::default(), 4);
    assert!(empty_batch.offsets.is_empty());
    assert!(empty_batch.exhausted);
}

#[test]
fn arena_probe_uses_dense_slots_and_compact_source_undo() {
    let mut arena = ColtArena::new();
    let root = arena.add_range_node(0, 2);
    let map = arena.force_node_with_key_fn(root, |offset, key| {
        key.clear();
        key.extend_from_slice(&offset.to_be_bytes());
    });
    assert_eq!(arena.map_entry_count(map), 2);
    let root_source = ArenaSourceHandle::new(ColtSourceId(0), root, SchemaVarsId(0));
    let mut store = ArenaSourceStore::with_atom_count(2);
    store.set_initial(1, root_source);

    let mut scratch = KeyScratch::new();
    let key = scratch.set(&1u32.to_be_bytes());
    let child = arena.get_child_source(root_source, key, SchemaVarsId(1));
    assert!(child.is_some());
    let Some(child) = child else { return };
    let mark = store.undo_mark();
    assert!(store.replace_source(1, child));
    assert_eq!(store.slot_count(), 2);
    assert_eq!(store.source_for(1), Some(child));
    store.restore_to(mark);
    assert_eq!(store.source_for(1), Some(root_source));
}

#[test]
fn arena_probe_key_scratch_and_lookup_are_allocation_bounded() {
    let mut arena = ColtArena::new();
    let root = arena.add_range_node(0, 16);
    arena.force_node_with_key_fn(root, |offset, key| {
        key.clear();
        key.extend_from_slice(&offset.to_be_bytes());
    });
    let source = ArenaSourceHandle::new(ColtSourceId(0), root, SchemaVarsId(0));

    let alloc_calls = crate::diagnostics::with_allocation_tracking_for_test(|| {
        let start = crate::diagnostics::allocation_snapshot();
        let mut scratch = KeyScratch::new();
        for value in 0..16u32 {
            let key = scratch.set(&value.to_be_bytes());
            assert!(
                arena
                    .get_child_source(source, key, SchemaVarsId(1))
                    .is_some()
            );
        }
        crate::diagnostics::allocation_delta(start, crate::diagnostics::allocation_snapshot())
            .alloc_calls
    });

    assert!(alloc_calls < 100);
}
