//! Scratch correctness across representations, atomic publication, real
//! allocation ownership, cancellation and early-stoppable borrowed visits.

use super::*;
use crate::storage::store::StoreError;
use crate::work::{WorkContext, WorkError};

fn work() -> WorkContext {
    WorkContext::new()
}

fn assert_cancelled(error: &Error) {
    assert!(
        matches!(error, Error::Store(store) if matches!(
            store.as_ref(), StoreError::Work(WorkError::Cancelled)
        )),
        "expected cancellation, got {error:?}"
    );
}

fn key(word: u64) -> [u8; 8] {
    word.to_be_bytes()
}

/// `last_at_or_before` answers the same predecessor question in RAM and on
/// the spilled tier: exact key ≤ bound, none when everything is above.
#[test]
fn predecessor_queries_agree_across_tiers() {
    let work = work();
    let mut ram = ScratchRelation::new(&work);
    let mut disk = ScratchRelation::new(&work);
    disk.force_spill().expect("forced spill");
    for scratch in [&mut ram, &mut disk] {
        for word in [10u64, 20, 30] {
            scratch.put(&key(word), &word.to_be_bytes()).expect("put");
        }
    }
    let mut found_key = Vec::new();
    let mut found_value = Vec::new();
    for (bound, expected) in [
        (5u64, None),
        (10, Some(10u64)),
        (25, Some(20)),
        (30, Some(30)),
        (u64::MAX, Some(30)),
    ] {
        for (tier, scratch) in [("ram", &mut ram), ("disk", &mut disk)] {
            let hit = scratch
                .last_at_or_before(&key(bound), &mut found_key, &mut found_value)
                .expect("query");
            match expected {
                None => assert!(!hit, "{tier}: nothing at or below {bound}"),
                Some(entry) => {
                    assert!(hit, "{tier}: predecessor of {bound}");
                    assert_eq!(found_key, key(entry), "{tier}: exact key");
                    assert_eq!(found_value, entry.to_be_bytes(), "{tier}: exact value");
                }
            }
        }
    }
}

/// Oversized bucketed keys never answer a predecessor probe — the query's
/// exactness contract is inline keys only, and buckets sort after them.
#[test]
fn predecessor_queries_ignore_bucketed_keys() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work);
    scratch.force_spill().expect("explicit disk staging");
    let long = vec![0xAA; MAX_INLINE_KEY + 100];
    scratch.insert_if_absent(&long, b"big").expect("insert");
    scratch.put(&key(10), b"small").expect("put");
    let mut found_key = Vec::new();
    let mut found_value = Vec::new();
    assert!(
        scratch
            .last_at_or_before(&key(u64::MAX), &mut found_key, &mut found_value)
            .expect("query"),
        "the inline key answers"
    );
    assert_eq!(found_key, key(10), "never a bucketed physical key");
}

#[test]
fn cancelled_insert_preserves_existing_entries() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        scratch.put(b"first", &[7; 2000]).unwrap();
        context.cancel();
        let failure = scratch.put(b"second", &[9; 8000]).unwrap_err();
        assert_cancelled(&failure);
        assert_eq!(scratch.len(), 1);
        // Test-only inspection under a fresh operation, after cancellation.
        scratch.work = work();
        let mut actual = Vec::new();
        assert!(scratch.get(b"first", &mut actual).unwrap());
        assert_eq!(actual, [7; 2000]);
        assert!(!scratch.get(b"second", &mut actual).unwrap());
        scratch.put(b"tiny", b"").unwrap();
        assert_eq!(scratch.len(), 2);
    }
}

#[test]
#[cfg(feature = "alloc-counter")]
fn ram_overwrites_release_old_payload_instead_of_retaining_history() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.put(b"slot", &[7; 8192]).unwrap();
    let before = crate::alloc_counter::snapshot();
    for index in 0..10_000u32 {
        scratch.put(b"slot", &index.to_be_bytes()).unwrap();
    }
    let after = crate::alloc_counter::snapshot();
    assert!(
        after.absolute.live_bytes + 8188 <= before.absolute.live_bytes,
        "one small value replaces the large value, not a history of overwrites"
    );
    assert_eq!(scratch.len(), 1);
    scratch
        .lookup(ScratchMapId::Default, b"slot", |probe| {
            assert_eq!(probe, ScratchProbe::Hit(9999u32.to_be_bytes().as_slice()));
            Ok(())
        })
        .unwrap();
}

#[test]
#[cfg(feature = "alloc-counter")]
fn disposal_releases_all_ram_maps_and_owned_payloads() {
    let context = work();
    let before = crate::alloc_counter::snapshot();
    let mut scratch = ScratchRelation::new(&context);
    for map in ScratchMapId::ALL {
        for word in 0..64u64 {
            scratch.put_map(map, &key(word), &[7; 1024]).unwrap();
        }
    }
    let populated = crate::alloc_counter::snapshot();
    assert!(populated.absolute.live_bytes > before.absolute.live_bytes + 64 * 1024);
    drop(scratch);
    let after = crate::alloc_counter::snapshot();
    assert_eq!(after.absolute.live_bytes, before.absolute.live_bytes);
    assert_eq!(
        after.window.alloc_bytes - before.window.alloc_bytes,
        after.window.dealloc_bytes - before.window.dealloc_bytes
    );
}

/// The single-transaction spill batch preserves every entry, value and
/// verdict — including oversized bucketed keys crossing the tier change.
#[test]
fn spill_batch_is_exact_including_oversized_keys() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work);
    let long_a = vec![0xCD; MAX_INLINE_KEY + 50];
    let mut long_b = long_a.clone();
    long_b.push(9);
    scratch.insert_if_absent(&long_a, b"a").expect("insert");
    scratch.insert_if_absent(&long_b, b"b").expect("insert");
    for word in 0..256u64 {
        scratch.put(&key(word), &word.to_be_bytes()).expect("put");
    }
    scratch
        .force_spill()
        .expect("explicit populated transition");
    assert!(scratch.spilled());
    assert_eq!(scratch.len(), 258);
    assert!(!scratch.insert_if_absent(&long_a, b"a").expect("dup"));
    let mut out = Vec::new();
    assert!(scratch.get(&long_b, &mut out).expect("get"));
    assert_eq!(out, b"b");
    assert!(scratch.get(&key(200), &mut out).expect("get"));
    assert_eq!(out, 200u64.to_be_bytes());
}

#[test]
fn map_full_retry_publishes_one_transaction_and_one_entry() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.force_spill().unwrap();
    scratch.put(b"seed", b"original").unwrap();
    let committed = scratch.committed_txn_for_test().unwrap();
    scratch.inject_map_full_before_commit(1);
    scratch.put(b"retry-key", &[7; 8192]).unwrap();
    assert_eq!(scratch.committed_txn_for_test(), Some(committed + 1));
    assert_eq!(scratch.len(), 2);
    let mut out = Vec::new();
    assert!(scratch.get(b"retry-key", &mut out).unwrap());
    assert_eq!(out, [7; 8192]);
}

#[test]
fn disk_overwrites_keep_one_entry_and_the_final_value() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.force_spill().unwrap();
    for index in 0..1024u32 {
        scratch
            .put(b"slot", &index.to_be_bytes().repeat(32))
            .unwrap();
    }
    assert_eq!(scratch.len(), 1);
    let mut actual = Vec::new();
    assert!(scratch.get(b"slot", &mut actual).unwrap());
    assert_eq!(actual, 1023u32.to_be_bytes().repeat(32));
}

fn replace_disk_values(staged: bool) {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.force_spill().unwrap();
    scratch.put(b"seed", b"original").unwrap();
    for value in [vec![1; 8192], vec![2; 3], vec![3; 4096]] {
        if staged {
            let mut batch = ScratchWriteBatch::new();
            batch.put(ScratchMapId::Default, b"second", &value).unwrap();
            batch
                .put(ScratchMapId::OrderLog, b"second", b"index")
                .unwrap();
            batch.commit(&mut scratch).unwrap();
        } else {
            scratch.put(b"second", &value).unwrap();
        }
        assert_eq!(scratch.len(), 2);
        let mut actual = Vec::new();
        assert!(scratch.get(b"second", &mut actual).unwrap());
        assert_eq!(actual, value);
    }
}

#[test]
fn disk_direct_writes_support_large_small_large_replacement() {
    replace_disk_values(false);
}

#[test]
fn disk_staged_writes_support_large_small_large_replacement() {
    replace_disk_values(true);
}

#[test]
fn ram_upserts_preserve_exact_current_values() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    let mut model = std::collections::BTreeMap::new();
    for size in [0, 128, 3, 8192, 0, 64] {
        for key in [b"a".as_slice(), b"another-key".as_slice()] {
            let value = vec![u8::try_from(size % 256).unwrap(); size];
            scratch.put(key, &value).unwrap();
            model.insert(key.to_vec(), value);
            assert_eq!(scratch.len(), model.len() as u64);
            let mut seen = 0;
            scratch
                .for_each(&mut |key, value| {
                    assert_eq!(model.get(key).unwrap(), value);
                    seen += 1;
                    Ok(true)
                })
                .unwrap();
            assert_eq!(seen, model.len());
        }
    }
    assert!(!scratch.spilled());
}

#[test]
fn failed_spill_preserves_ram_rows_and_can_retry() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.put(b"seed", &[7; 1024]).unwrap();
    super::inject_setup_fail_after_exclusive_dir();
    assert!(scratch.force_spill().is_err());
    assert!(!scratch.spilled());
    assert_eq!(scratch.len(), 1);
    let mut actual = Vec::new();
    assert!(scratch.get(b"seed", &mut actual).unwrap());
    assert_eq!(actual, [7; 1024]);
    scratch.force_spill().unwrap();
    assert!(scratch.spilled());
    assert!(scratch.get(b"seed", &mut actual).unwrap());
    assert_eq!(actual, [7; 1024]);
}

#[test]
fn cancelled_spill_does_not_move_or_drop_ram_owners() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.put(b"slot", &[7; 1024]).unwrap();
    context.cancel();
    assert_cancelled(&scratch.force_spill().unwrap_err());
    assert!(!scratch.spilled());
    assert_eq!(scratch.len(), 1);
    scratch.work = work();
    let mut actual = Vec::new();
    assert!(scratch.get(b"slot", &mut actual).unwrap());
    assert_eq!(actual, [7; 1024]);
}

#[test]
fn wide_keys_remain_exact_after_cancelled_mutation() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.force_spill().unwrap();
    let mut a = vec![0xAB; MAX_INLINE_KEY + 80];
    let mut b = a.clone();
    b.push(1);
    a.push(2);
    scratch.put_exact(ScratchExactKey::new(&a), b"a").unwrap();
    scratch.put_exact(ScratchExactKey::new(&b), b"b").unwrap();
    context.cancel();
    assert_cancelled(&scratch.put(&a, b"replacement").unwrap_err());
    assert_eq!(scratch.len(), 2);
    scratch.work = work();
    let mut out = Vec::new();
    assert!(scratch.get(&a, &mut out).unwrap());
    assert_eq!(out, b"a");
    assert!(scratch.get(&b, &mut out).unwrap());
    assert_eq!(out, b"b");
}

/// D03: exclusive setup does not adopt or delete an unowned directory.
#[test]
fn d03_failed_setup_does_not_touch_unowned_directories() {
    let context = WorkContext::new();
    let foreign = std::env::temp_dir().join(format!(
        "bumbledb-scratch-foreign-{}-{:x}",
        std::process::id(),
        super::fastrand_seed()
    ));
    std::fs::create_dir(&foreign).expect("foreign dir");
    std::fs::write(foreign.join("owned-by-other"), b"keep").expect("marker");
    let err = ScratchRelation::setup_at(&context, &foreign).expect_err("collision");
    assert!(
        foreign.exists(),
        "preexisting directory must survive the refused setup: {err:?}"
    );
    assert_eq!(
        std::fs::read(foreign.join("owned-by-other")).expect("marker"),
        b"keep"
    );
    let _ = std::fs::remove_dir_all(&foreign);
}

/// D03: a failed init after exclusive create removes only the owned dir.
#[test]
fn d03_repeated_failed_setup_cleans_only_owned_identity() {
    let context = WorkContext::new();
    let neighbor = std::env::temp_dir().join(format!(
        "bumbledb-scratch-neighbor-{}-{:x}",
        std::process::id(),
        super::fastrand_seed()
    ));
    std::fs::create_dir(&neighbor).expect("neighbor");
    let owned = std::env::temp_dir().join(format!(
        "bumbledb-scratch-owned-{}-{:x}",
        std::process::id(),
        super::fastrand_seed()
    ));
    super::inject_setup_fail_after_exclusive_dir();
    ScratchRelation::setup_at(&context, &owned).expect_err("injected fail");
    assert!(
        !owned.exists(),
        "owned identity created then failed must be unlinked"
    );
    assert!(
        neighbor.exists(),
        "neighbor directory is not ours to delete"
    );
    let _ = std::fs::remove_dir_all(&neighbor);
}

/// D03: ordered word claims plus an early-stoppable visitor.
#[test]
fn d03_word_claims_and_early_stop_visitor() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work);
    let first = ScratchClaimKey::new([1, 10, 20]);
    let second = ScratchClaimKey::new([1, 30, 40]);
    scratch.put_words(first, b"a").expect("first");
    scratch.put_words(second, b"b").expect("second");
    let mut seen = 0u32;
    scratch
        .visit(&mut |_key: &[u8], _value: &[u8]| {
            seen += 1;
            Ok(false)
        })
        .expect("early stop");
    assert_eq!(seen, 1, "visitor may stop after the first entry");
    let mut out = Vec::new();
    assert!(scratch.get_words(second, &mut out).expect("get"));
    assert_eq!(out, b"b");
}

#[test]
fn d03_pack_claim_key_is_three_be64_and_inline() {
    assert_eq!(ScratchClaimKey::BYTE_LEN, 24);
    const {
        assert!(ScratchClaimKey::BYTE_LEN <= MAX_INLINE_KEY);
    }
    let key = ScratchClaimKey::new([7, 10, 20]);
    let bytes = key.encode();
    assert_eq!(&bytes[..8], &7u64.to_be_bytes());
    assert_eq!(&bytes[8..16], &10u64.to_be_bytes());
    assert_eq!(&bytes[16..], &20u64.to_be_bytes());
}

/// Claim visit and group-header get share one env / one directory.
#[test]
fn d03_named_maps_share_one_env() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work);
    scratch.force_spill().expect("one env");
    let path = scratch.scratch_path().expect("spilled");
    let claim = ScratchClaimKey::new([1, 0, 8]);
    let header = b"group-head-bytes";
    let mut batch = ScratchWriteBatch::new();
    batch
        .put(ScratchMapId::Default, &claim.encode(), &[])
        .expect("stage claim");
    batch
        .put(ScratchMapId::TokenToGroup, &1u64.to_be_bytes(), header)
        .expect("stage header");
    batch
        .put(ScratchMapId::GroupToToken, header, &1u64.to_be_bytes())
        .expect("stage token");
    batch.commit(&mut scratch).expect("one txn");
    assert_eq!(scratch.scratch_path().as_ref(), Some(&path));
    let mut seen = 0u32;
    let mut header_out = Vec::new();
    scratch
        .visit_with_lookup(ScratchMapId::Default, &mut |lookup, key, _| {
            seen += 1;
            assert_eq!(ScratchClaimKey::decode(key), Some(claim));
            assert!(lookup.get(
                ScratchMapId::TokenToGroup,
                &1u64.to_be_bytes(),
                &mut header_out
            )?);
            Ok(true)
        })
        .expect("claim cursor + header get");
    assert_eq!(seen, 1);
    assert_eq!(header_out, header);
}

/// Insertion order is a roster slot on the same env as Default — not a
/// second `ScratchRelation`.
#[test]
fn d03_order_log_shares_one_env() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work);
    scratch.force_spill().expect("one env");
    let path = scratch.scratch_path().expect("spilled");
    let mut append = ScratchAppend::new(&mut scratch);
    append
        .append(ScratchMapId::Default, b"row-key", &[])
        .expect("set");
    append
        .append(ScratchMapId::OrderLog, &0u64.to_be_bytes(), b"row-key")
        .expect("log");
    append.finish().expect("finish");
    assert_eq!(scratch.scratch_path().as_ref(), Some(&path));
    let mut out = Vec::new();
    assert!(
        scratch
            .get_map(ScratchMapId::OrderLog, &0u64.to_be_bytes(), &mut out)
            .expect("get log")
    );
    assert_eq!(out, b"row-key");
}

#[test]
fn lookup_walk_visits_inline_and_bucketed_keys_once_and_stops_exactly() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        let keys = [
            b"small".to_vec(),
            vec![0; super::MAX_INLINE_KEY + 1],
            vec![255; super::MAX_INLINE_KEY + 1],
        ];
        let mut append = ScratchAppend::new(&mut scratch);
        for key in &keys {
            append
                .append(ScratchMapId::Default, key, b"row")
                .expect("row");
            append
                .append(ScratchMapId::TokenToGroup, key, b"header")
                .expect("lookup");
        }
        append.finish().expect("commit");
        let mut seen = std::collections::BTreeSet::new();
        let mut header = Vec::new();
        scratch
            .visit_with_lookup(ScratchMapId::Default, &mut |lookup, key, value| {
                assert!(
                    seen.insert(key.to_vec()),
                    "a wide key must advance physically"
                );
                assert_eq!(value, b"row");
                assert!(lookup.get(ScratchMapId::TokenToGroup, key, &mut header)?);
                assert_eq!(header, b"header");
                Ok(true)
            })
            .expect("complete walk");
        assert_eq!(seen, keys.into_iter().collect());
        let mut visits = 0;
        scratch
            .visit_with_lookup(ScratchMapId::Default, &mut |_, _, _| {
                visits += 1;
                Ok(false)
            })
            .expect("early stop");
        assert_eq!(visits, 1);
        visits = 0;
        let error = crate::error::Error::ResultBytesOverflow;
        let result = scratch.visit_with_lookup(ScratchMapId::Default, &mut |_, _, _| {
            visits += 1;
            Err(error.clone())
        });
        assert!(matches!(
            result,
            Err(crate::error::Error::ResultBytesOverflow)
        ));
        assert_eq!(visits, 1);
    }
}

#[test]
fn public_batch_retry_commits_named_maps_and_wide_keys_exactly_once() {
    let context = work();
    let mut scratch = ScratchRelation::new(&context);
    scratch.force_spill().unwrap();
    scratch.put(b"seed", b"original").unwrap();
    let committed = scratch.committed_txn_for_test().unwrap();
    let wide = vec![7; MAX_INLINE_KEY + 1];
    let mut batch = ScratchWriteBatch::new();
    for key in [b"inline".as_slice(), wide.as_slice()] {
        batch.put(ScratchMapId::Default, key, &[9; 8192]).unwrap();
        batch.put(ScratchMapId::OrderLog, key, b"index").unwrap();
    }
    scratch.inject_map_full_before_commit(1);
    batch.commit(&mut scratch).unwrap();
    assert_eq!(scratch.len(), 3);
    assert_eq!(scratch.committed_txn_for_test(), Some(committed + 1));
    let mut out = Vec::new();
    for key in [b"inline".as_slice(), wide.as_slice()] {
        assert!(scratch.get(key, &mut out).unwrap());
        assert_eq!(out, [9; 8192]);
        assert!(
            scratch
                .get_map(ScratchMapId::OrderLog, key, &mut out)
                .unwrap()
        );
        assert_eq!(out, b"index");
    }
}

#[test]
fn batch_cardinality_counts_distinct_default_keys_on_both_tiers() {
    for disk in [false, true] {
        let context = WorkContext::new();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        let wide = vec![b'x'; super::MAX_INLINE_KEY + 1];
        let mut batch = ScratchWriteBatch::new();
        for key in [b"small".as_slice(), wide.as_slice()] {
            batch
                .put(ScratchMapId::Default, key, b"old")
                .expect("first");
            batch
                .put(ScratchMapId::Default, key, b"new")
                .expect("overwrite");
            batch
                .insert_if_absent(ScratchMapId::Default, key, b"ignored")
                .expect("duplicate");
            batch
                .put(ScratchMapId::OrderLog, key, b"index")
                .expect("other map");
        }
        batch.commit(&mut scratch).expect("commit");
        assert_eq!(
            scratch.len(),
            2,
            "duplicates and auxiliary maps add no rows"
        );
        let mut value = Vec::new();
        for key in [b"small".as_slice(), wide.as_slice()] {
            assert!(scratch.get(key, &mut value).expect("stored row"));
            assert_eq!(value, b"new");
        }
    }
}

#[test]
fn staged_inline_upserts_preserve_values_and_counts() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        let keys = [vec![], vec![0], vec![255; MAX_INLINE_KEY]];
        let maps = [ScratchMapId::Default, ScratchMapId::OrderLog];
        let mut model = std::collections::BTreeMap::new();
        for round in 0..4 {
            let mut batch = ScratchWriteBatch::new();
            // Duplicate absent-inserts, replacements, empty values, overflow
            // values and shrinkage all occur within the same transaction.
            for size in [0, 1, 8192, 3, 0, round + 1] {
                for (map_index, &map) in maps.iter().enumerate() {
                    for (key_index, key) in keys.iter().enumerate() {
                        for if_absent in [true, true, false] {
                            let value = vec![u8::try_from(round + key_index).unwrap(); size];
                            if if_absent {
                                batch
                                    .insert_if_absent(map, key, &value)
                                    .expect("stage absent");
                                model.entry((map_index, key_index)).or_insert(value);
                            } else {
                                batch.put(map, key, &value).expect("stage replacement");
                                model.insert((map_index, key_index), value);
                            }
                        }
                    }
                }
            }
            if round == 1 && scratch.spilled() {
                scratch.inject_map_full_before_commit(1);
            }
            batch.commit(&mut scratch).expect("commit or retry");
            assert_eq!(
                scratch.len(),
                keys.len() as u64,
                "only distinct default keys count"
            );
            let mut value = Vec::new();
            for (&(map_index, key_index), expected) in &model {
                assert!(
                    scratch
                        .get_map(maps[map_index], &keys[key_index], &mut value)
                        .unwrap()
                );
                assert_eq!(&value, expected);
            }
        }
    }
}

#[test]
fn cancelled_batch_rolls_back_new_keys_and_replacements_together() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        scratch.put(b"seed", b"original").unwrap();
        let mut batch = ScratchWriteBatch::new();
        batch
            .put(ScratchMapId::Default, b"seed", b"replacement")
            .unwrap();
        batch
            .put(ScratchMapId::OrderLog, b"new-index", b"index")
            .unwrap();
        batch
            .put(ScratchMapId::Default, b"new-row", &[0; 8192])
            .unwrap();
        context.cancel();
        assert_cancelled(&batch.commit(&mut scratch).unwrap_err());
        assert_eq!(scratch.len(), 1);
        scratch.work = work();
        let mut actual = Vec::new();
        assert!(scratch.get(b"seed", &mut actual).unwrap());
        assert_eq!(actual, b"original");
        assert!(!scratch.get(b"new-row", &mut actual).unwrap());
        assert!(
            !scratch
                .get_map(ScratchMapId::OrderLog, b"new-index", &mut actual)
                .unwrap()
        );
        scratch.put(b"seed", b"reusable").unwrap();
        assert!(scratch.get(b"seed", &mut actual).unwrap());
        assert_eq!(actual, b"reusable");
    }
}

#[test]
fn cancelled_append_stops_source_and_discards_only_the_pending_batch() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        let mut pulled = 0;
        let mut accepted = 0;
        let stop = usize::from(SPILL_BATCH) + 2;
        let result = {
            let mut append = ScratchAppend::new(&mut scratch);
            let source = (0..10_000).inspect(|_| {
                pulled += 1;
            });
            (|| {
                for index in source {
                    if index == stop {
                        context.cancel();
                    }
                    append.append(ScratchMapId::Default, &key(index as u64), &[7; 64])?;
                    accepted += 1;
                }
                append.finish()
            })()
        };
        assert_cancelled(&result.unwrap_err());
        assert_eq!(pulled, stop + 1, "no eager source collection");
        assert_eq!(accepted, stop);
        assert_eq!(
            scratch.len(),
            u64::from(SPILL_BATCH),
            "only a complete batch commits"
        );
        scratch.work = work();
        let mut seen = 0;
        scratch
            .for_each(&mut |key_bytes, value| {
                assert_eq!(key_bytes, key(seen));
                assert_eq!(value, [7; 64]);
                seen += 1;
                Ok(true)
            })
            .unwrap();
        assert_eq!(seen, u64::from(SPILL_BATCH));
    }
}

#[test]
fn dropping_pending_named_map_batch_publishes_neither_direction() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        let mut batch = ScratchWriteBatch::new();
        batch
            .put(ScratchMapId::GroupToToken, b"group", &1u64.to_be_bytes())
            .unwrap();
        batch
            .put(ScratchMapId::TokenToGroup, &1u64.to_be_bytes(), b"group")
            .unwrap();
        batch.abort();
        let mut out = Vec::new();
        assert!(
            !scratch
                .get_map(ScratchMapId::GroupToToken, b"group", &mut out)
                .unwrap()
        );
        assert!(
            !scratch
                .get_map(ScratchMapId::TokenToGroup, &1u64.to_be_bytes(), &mut out)
                .unwrap()
        );
        assert_eq!(scratch.len(), 0);
    }
}

/// Borrowed value length comes from the live map, not a side index.
#[test]
fn d03_value_len_borrows_before_copy() {
    let work = work();
    let mut rows = ScratchRelation::new(&work);
    let mut append = ScratchAppend::new(&mut rows);
    append
        .append(ScratchMapId::Default, b"row", b"abcdef")
        .expect("append");
    append.finish().expect("finish");
    let probe = rows.value_len(ScratchMapId::Default, b"row").expect("len");
    let ScratchProbe::Hit(len) = probe else {
        panic!("committed row must hit");
    };
    assert_eq!(len, 6);
    rows.lookup(ScratchMapId::Default, b"row", |probe| {
        let ScratchProbe::Hit(bytes) = probe else {
            panic!("borrowed hit");
        };
        assert_eq!(bytes.len() as u64, len);
        Ok(())
    })
    .expect("borrow");
    assert!(
        rows.value_len(ScratchMapId::Default, b"missing")
            .expect("miss")
            .is_miss()
    );
}

#[test]
fn cancelled_lookup_is_error_not_miss_and_never_calls_the_visitor() {
    for disk in [false, true] {
        let context = work();
        let mut scratch = ScratchRelation::new(&context);
        if disk {
            scratch.force_spill().unwrap();
        }
        scratch.put(b"present", b"value").unwrap();
        context.cancel();
        for key in [b"present".as_slice(), b"absent".as_slice()] {
            let failure = scratch
                .lookup::<()>(ScratchMapId::Default, key, |_| {
                    panic!("cancelled lookup must not report either hit or miss")
                })
                .unwrap_err();
            assert_cancelled(&failure);
        }
    }
}
