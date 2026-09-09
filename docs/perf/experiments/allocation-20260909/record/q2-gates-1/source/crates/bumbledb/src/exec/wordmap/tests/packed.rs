use super::*;

#[test]
fn payload_lengths_follow_actual_rows_not_hash_slots() {
    for arity in [0, 1, 2, 4, 8, 9] {
        let mut map = WordMap::<u64>::with_capacity_hint(arity, 1_024);
        assert_eq!(map.keys.len(), 0, "unexecuted payload must be empty");
        assert_eq!(map.values.len(), 0);
        for i in 0..129_u64 {
            let key = vec![i; arity];
            map.get_or_insert_with(&key, || i + 1);
            assert_eq!(map.keys.len(), map.len() * arity);
            assert_eq!(map.values.len(), map.len());
        }
        let before: Vec<_> = map.iter().map(|(k, v)| (k.to_vec(), *v)).collect();
        let payload = (map.keys.as_ptr(), map.values.as_ptr());
        let capacity = (map.keys.capacity(), map.values.capacity());
        map.grow();
        assert_eq!((map.keys.as_ptr(), map.values.as_ptr()), payload);
        assert_eq!((map.keys.capacity(), map.values.capacity()), capacity);
        assert_eq!(
            map.iter()
                .map(|(k, v)| (k.to_vec(), *v))
                .collect::<Vec<_>>(),
            before
        );
        for since in [0, 1, before.len(), before.len() + 1, usize::MAX] {
            let iter = map.iter_since(since);
            assert!(iter.clone().eq(iter));
            assert_eq!(
                map.iter_since(since)
                    .map(|(k, v)| (k.to_vec(), *v))
                    .collect::<Vec<_>>(),
                before[since.min(before.len())..]
            );
        }
        map.clear();
        assert!(map.keys.is_empty() && map.values.is_empty());
        assert_eq!((map.keys.as_ptr(), map.values.as_ptr()), payload);
        assert_eq!((map.keys.capacity(), map.values.capacity()), capacity);
    }
}

#[test]
fn packed_payload_reuses_capacity_through_stale_ordinals_and_rollover() {
    for arity in [0, 1, 2, 8, 9] {
        let mut map = WordMap::<u64>::with_capacity_hint(arity, 1_024);
        let count = if arity == 0 { 1 } else { 33 };
        let keys: Vec<_> = (0..count * 2).map(|i| vec![i; arity]).collect();
        for key in &keys[..count as usize] {
            map.get_or_insert_with(key, || 0);
        }
        let payload = (map.keys.as_ptr(), map.values.as_ptr());
        #[cfg(feature = "alloc-counter")]
        let before = crate::alloc_counter::snapshot().window;
        for generation in 0..600_u64 {
            map.clear();
            let offset = if generation % 2 == 0 {
                0
            } else {
                count as usize
            };
            for (i, key) in keys[offset..offset + count as usize]
                .iter()
                .enumerate()
                .rev()
            {
                assert!(!map.contains_key(key));
                let (value, inserted) = map.get_or_insert_with(key, || generation);
                assert!(inserted);
                *value += i as u64;
                let (same, duplicate) = map.get_or_insert_with(key, || panic!("duplicate"));
                assert!(!duplicate);
                assert_eq!(*same, generation + i as u64);
            }
            assert_eq!(map.keys.len(), map.len() * arity);
            assert_eq!(map.values.len(), map.len());
            assert_eq!(map.len(), count as usize);
            assert_eq!((map.keys.as_ptr(), map.values.as_ptr()), payload);
        }
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().window, before);
    }
}

#[test]
fn constructor_panic_after_index_growth_preserves_existing_payload() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    for arity in [1, 2, 8, 9] {
        let mut map = WordMap::<u64>::new(arity);
        for i in 0..2 {
            map.get_or_insert_with(&vec![i; arity], || i + 10);
        }
        let before: Vec<_> = map.iter().map(|(k, v)| (k.to_vec(), *v)).collect();
        let payload = (map.keys.as_ptr(), map.values.as_ptr());
        let capacity = map.capacity();
        let key = vec![2; arity];
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                map.get_or_insert_with(&key, || panic!("refuse construction"));
            }))
            .is_err()
        );
        assert_eq!(map.capacity(), capacity * 2);
        assert_eq!((map.keys.as_ptr(), map.values.as_ptr()), payload);
        assert_eq!(map.keys.len(), map.len() * arity);
        assert_eq!(map.values.len(), map.len());
        assert!(!map.contains_key(&key));
        assert_eq!(
            map.iter()
                .map(|(k, v)| (k.to_vec(), *v))
                .collect::<Vec<_>>(),
            before
        );
        assert!(map.get_or_insert_with(&key, || 99).1);
        assert_eq!(*map.get_or_insert_with(&key, || 0).0, 99);
    }
}
