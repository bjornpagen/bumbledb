//! F3 finding C regressions for the charged scratch map: the predecessor
//! query is exact across both tiers, refused growth rolls its counter
//! back, and the spill copy stays exact through the single-transaction
//! batch path.

use super::*;
use crate::api::prepared::source::UNBOUNDED_POLICY;
use crate::storage::store::StoreError;
use crate::work::{ExecutionPolicy, Resource, WorkContext, WorkError};
use super::{
    ScratchAppend, ScratchCapability, ScratchClaimKey, ScratchExactKey, ScratchMapId,
    ScratchPolicy, ScratchProbe, ScratchTextLookup,
};

fn work() -> WorkContext {
    UNBOUNDED_POLICY.start().expect("unbounded ledger")
}

fn key(word: u64) -> [u8; 8] {
    word.to_be_bytes()
}

/// `last_at_or_before` answers the same predecessor question in RAM and on
/// the spilled tier: exact key ≤ bound, none when everything is above.
#[test]
fn predecessor_queries_agree_across_tiers() {
    let work = work();
    let mut ram = ScratchRelation::with_default_budget(&work);
    let mut disk = ScratchRelation::with_default_budget(&work);
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
    let mut scratch = ScratchRelation::new(&work, 0); // disk from entry zero
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

/// A refused RAM-tier charge rolls the byte counter back: the failed
/// insert did not happen, and the accounting says so (early-error
/// accounting, refund-exactly-once).
#[test]
fn refused_growth_rolls_the_byte_counter_back() {
    let context = ExecutionPolicy {
        working_bytes: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, usize::MAX);
    let big = vec![0u8; 2000];
    scratch.insert_if_absent(b"first", &big).expect("fits");
    let used_before = context.used(Resource::WorkingBytes);
    let len_before = scratch.len();
    scratch
        .insert_if_absent(b"second", &vec![0u8; 8000])
        .expect_err("beyond the working allowance");
    assert_eq!(scratch.len(), len_before, "the refused insert is absent");
    assert_eq!(
        context.used(Resource::WorkingBytes),
        used_before,
        "no charge survives the refusal"
    );
    assert!(
        scratch
            .insert_if_absent(b"tiny", b"")
            .expect("small insert")
    );
}

/// Repeated overwrites of one bounded key charge net retention, not traffic.
#[test]
fn overwrites_charge_net_retention_not_traffic() {
    let context = ExecutionPolicy {
        working_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, usize::MAX);
    scratch.put(b"slot", &[0u8; 128]).expect("put");
    let after_first = context.used(Resource::WorkingBytes);
    for index in 0..10_000u32 {
        scratch
            .put(b"slot", &index.to_be_bytes())
            .expect("overwrite");
    }
    let after_many = context.used(Resource::WorkingBytes);
    assert!(
        after_many <= after_first + 64,
        "one slot's overwrites do not accumulate retained charges ({after_first} -> {after_many})"
    );
}

/// Dropping the relation refunds every held reservation exactly once: the
/// ledger returns to its pre-relation state.
#[test]
fn disposal_refunds_the_whole_charge() {
    let context = ExecutionPolicy {
        working_bytes: 1 << 20,
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let working_before = context.used(Resource::WorkingBytes);
    let scratch_before = context.used(Resource::ScratchBytes);
    let mut scratch = ScratchRelation::new(&context, 2048);
    for word in 0..64u64 {
        scratch.put(&key(word), &[0u8; 64]).expect("put");
    }
    assert!(scratch.spilled(), "the tiny RAM allowance forces the spill");
    assert!(context.used(Resource::ScratchBytes) > scratch_before);
    drop(scratch);
    assert_eq!(context.used(Resource::WorkingBytes), working_before);
    assert_eq!(context.used(Resource::ScratchBytes), scratch_before);
}

/// The single-transaction spill batch preserves every entry, value and
/// verdict — including oversized bucketed keys crossing the tier change.
#[test]
fn spill_batch_is_exact_including_oversized_keys() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work, 4096);
    let long_a = vec![0xCD; MAX_INLINE_KEY + 50];
    let mut long_b = long_a.clone();
    long_b.push(9);
    scratch.insert_if_absent(&long_a, b"a").expect("insert");
    scratch.insert_if_absent(&long_b, b"b").expect("insert");
    for word in 0..256u64 {
        scratch.put(&key(word), &word.to_be_bytes()).expect("put");
    }
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
fn scratch_capability_opens_the_one_substrate() {
    use crate::exec::scratch::capability::{ScratchCapability, ScratchPolicy};
    let cap = ScratchCapability::start(UNBOUNDED_POLICY, ScratchPolicy::unbounded())
        .expect("start");
    let mut relation = cap.relation();
    assert!(relation.insert_if_absent(b"k", b"v").expect("insert"));
    assert!(!relation.insert_if_absent(b"k", b"v").expect("dup"));
}

#[test]
fn scratch_policy_is_enforced_at_capability_start() {
    use crate::exec::scratch::capability::{ScratchCapability, ScratchPolicy};
    let execution = ExecutionPolicy {
        scratch_bytes: 1024,
        ..UNBOUNDED_POLICY
    };
    let mismatch = ScratchPolicy {
        scratch_bytes: 4096,
        ..ScratchPolicy::unbounded()
    };
    assert!(ScratchCapability::start(execution, mismatch).is_err());
    let matched = ScratchPolicy::from_execution(execution);
    ScratchCapability::start(execution, matched).expect("matched policy");
}

/// `on_work` charges the execute ledger, not a reconstructed twin.
#[test]
fn d03_on_work_shares_the_execute_ledger() {
    use crate::exec::scratch::capability::{ScratchCapability, ScratchPolicy};
    let work = ExecutionPolicy {
        scratch_bytes: 8192,
        working_bytes: 8192,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    work.reserve(crate::work::ByteKind::Scratch, 64)
        .expect("prior charge");
    let before = work.used(Resource::ScratchBytes);
    let cap = ScratchCapability::on_work(&work, ScratchPolicy::from_work(&work)).expect("on_work");
    assert_eq!(cap.work().used(Resource::ScratchBytes), before);
    assert_eq!(cap.work().limit(Resource::ScratchBytes), work.limit(Resource::ScratchBytes));
    let mut relation = cap.relation();
    relation.put(b"k", &[0u8; 32]).expect("put");
    assert!(
        work.used(Resource::ScratchBytes) > before
            || work.used(Resource::WorkingBytes) > 0,
        "relation charges the execute ledger"
    );
    let twin = ScratchCapability::start(
        ExecutionPolicy {
            scratch_bytes: 8192,
            working_bytes: 8192,
            ..UNBOUNDED_POLICY
        },
        ScratchPolicy::from_work(&work),
    )
    .expect("twin");
    assert_eq!(
        twin.work().used(Resource::ScratchBytes),
        0,
        "sensitivity: start() is a twin ledger with zero prior charge"
    );
}

/// D03: MapFull after reservation and before commit, then a successful
/// retry, charges exactly one committed entry — not the aborted attempt
/// plus the retry.
#[test]
fn d03_map_full_retry_charges_once_not_twice() {
    let context = ExecutionPolicy {
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, 0);
    scratch.put(b"seed", &[0u8; 8]).expect("spill + seed");
    assert!(scratch.spilled());
    let after_seed = context.used(Resource::ScratchBytes);
    scratch.inject_map_full_after_reserve(1);
    scratch
        .put(b"retry-key", &[0u8; super::CHARGE_CHUNK])
        .expect("retry after MapFull");
    let after_retry = context.used(Resource::ScratchBytes);
    let once = after_retry - after_seed;
    assert_eq!(
        once,
        super::CHARGE_CHUNK as u64,
        "one committed chunk; sensitivity: abort+retry would be {}",
        super::CHARGE_CHUNK as u64 * 2
    );
    assert_eq!(scratch.len(), 2);
    let mut out = Vec::new();
    assert!(scratch.get(b"retry-key", &mut out).expect("get"));
    assert_eq!(out.len(), super::CHARGE_CHUNK);
}

/// D03: equal-size overwrite does not linearly consume the budget.
#[test]
fn d03_equal_size_overwrite_does_not_bill_traffic() {
    let context = ExecutionPolicy {
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, 0);
    scratch.put(b"slot", &[0u8; 128]).expect("put");
    let after_first = context.used(Resource::ScratchBytes);
    let logical = scratch.logical_bytes();
    let reserved = scratch.reserved_bytes();
    for index in 0..10_000u32 {
        scratch
            .put(b"slot", &index.to_be_bytes().repeat(32)[..128])
            .expect("overwrite");
    }
    assert_eq!(scratch.logical_bytes(), logical);
    assert_eq!(scratch.reserved_bytes(), reserved);
    assert_eq!(
        context.used(Resource::ScratchBytes),
        after_first,
        "equal-size overwrite charges once"
    );
}

/// D03: shrink updates live logical bytes; reserved-page charge stays.
#[test]
fn d03_shrink_reuses_reserved_pages() {
    let context = ExecutionPolicy {
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, 0);
    scratch.put(b"slot", &[0u8; 512]).expect("put");
    let reserved = scratch.reserved_bytes();
    let charged = context.used(Resource::ScratchBytes);
    scratch.put(b"slot", &[1u8; 8]).expect("shrink");
    assert!(scratch.logical_bytes() < reserved);
    assert_eq!(scratch.reserved_bytes(), reserved);
    assert_eq!(context.used(Resource::ScratchBytes), charged);
    scratch.put(b"slot", &[2u8; 64]).expect("reuse");
    assert_eq!(scratch.reserved_bytes(), reserved);
    assert_eq!(context.used(Resource::ScratchBytes), charged);
}

/// D03: colliding wide keys stay exact; reservation failure restores.
#[test]
fn d03_colliding_wide_keys_and_failed_reservation() {
    let context = ExecutionPolicy {
        scratch_bytes: super::CHARGE_CHUNK as u64,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, 0);
    let mut a = vec![0xABu8; MAX_INLINE_KEY + 80];
    let mut b = a.clone();
    b.push(0x01);
    a.push(0x02);
    scratch.put_exact(ScratchExactKey::new(&a), b"a").expect("a");
    scratch.put_exact(ScratchExactKey::new(&b), b"b").expect("b");
    assert_eq!(scratch.len(), 2);
    let mut out = Vec::new();
    assert!(scratch.get(&a, &mut out).expect("get a"));
    assert_eq!(out, b"a");
    assert!(scratch.get(&b, &mut out).expect("get b"));
    assert_eq!(out, b"b");
    let used = context.used(Resource::ScratchBytes);
    let len = scratch.len();
    scratch
        .put(b"overflow", &[0u8; super::CHARGE_CHUNK])
        .expect_err("policy/ledger refuses");
    assert_eq!(scratch.len(), len);
    assert_eq!(context.used(Resource::ScratchBytes), used);
}

/// D03: exclusive setup does not adopt or delete an unowned directory.
#[test]
fn d03_failed_setup_does_not_touch_unowned_directories() {
    let context = ExecutionPolicy {
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let foreign = std::env::temp_dir().join(format!(
        "bumbledb-scratch-foreign-{}-{:x}",
        std::process::id(),
        super::fastrand_seed()
    ));
    std::fs::create_dir(&foreign).expect("foreign dir");
    std::fs::write(foreign.join("owned-by-other"), b"keep").expect("marker");
    let err = ScratchRelation::setup_at(&context, foreign.clone()).expect_err("collision");
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
    let context = ExecutionPolicy {
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
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
    ScratchRelation::setup_at(&context, owned.clone()).expect_err("injected fail");
    assert!(
        !owned.exists(),
        "owned identity created then failed must be unlinked"
    );
    assert!(neighbor.exists(), "neighbor directory is not ours to delete");
    let _ = std::fs::remove_dir_all(&neighbor);
}

/// D03: ordered word claims plus an early-stoppable visitor.
#[test]
fn d03_word_claims_and_early_stop_visitor() {
    let work = work();
    let mut scratch = ScratchRelation::with_default_budget(&work);
    let first = ScratchClaimKey::new([1, 10, 20]);
    let second = ScratchClaimKey::new([1, 30, 40]);
    scratch.put_words(first, b"a").expect("first");
    scratch.put_words(second, b"b").expect("second");
    let mut seen = 0u32;
    scratch
        .visit(&mut |_key, _value| {
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
    assert!(ScratchClaimKey::BYTE_LEN <= MAX_INLINE_KEY);
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
    let mut scratch = ScratchRelation::new(&work, 0);
    scratch
        .open_map(ScratchMapId::GroupToToken)
        .expect("open g2t");
    scratch
        .open_map(ScratchMapId::TokenToGroup)
        .expect("open t2g");
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
            assert!(lookup.get(ScratchMapId::TokenToGroup, &1u64.to_be_bytes(), &mut header_out)?);
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
    let mut scratch = ScratchRelation::new(&work, 0);
    scratch
        .open_map(ScratchMapId::OrderLog)
        .expect("open order log");
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
    assert!(scratch
        .get_map(ScratchMapId::OrderLog, &0u64.to_be_bytes(), &mut out)
        .expect("get log"));
    assert_eq!(out, b"row-key");
}

/// Public commit: MapFull after reserve / before txn commit, then retry,
/// charges once.
#[test]
fn d03_public_commit_map_full_charges_once() {
    let context = ExecutionPolicy {
        scratch_bytes: 1 << 20,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, 0);
    scratch.force_spill().expect("spill");
    let mut seed = ScratchWriteBatch::new();
    seed.put(ScratchMapId::Default, b"seed", &[0u8; 8])
        .expect("stage seed");
    seed.commit(&mut scratch).expect("seed");
    let after_seed = context.used(Resource::ScratchBytes);
    scratch.inject_map_full_after_reserve(1);
    let mut batch = ScratchWriteBatch::new();
    batch
        .put(ScratchMapId::Default, b"retry-key", &[0u8; super::CHARGE_CHUNK])
        .expect("stage");
    batch.commit(&mut scratch).expect("retry commit");
    let once = context.used(Resource::ScratchBytes) - after_seed;
    assert_eq!(
        once,
        super::CHARGE_CHUNK as u64,
        "sensitivity: abort+retry would be {}",
        super::CHARGE_CHUNK as u64 * 2
    );
}

/// Mid-stream append refusal stops the walk. Later source rows are not
/// copied into a pending collection; scratch does not hold the full set.
#[test]
fn d03_mid_stream_append_does_not_collect_the_rest() {
    let context = ExecutionPolicy {
        working_bytes: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, usize::MAX);
    let source: [&[u8]; 4] = [&[0u8; 2000], &[1u8; 2000], &[2u8; 2000], &[3u8; 2000]];
    let mut accepted = 0u32;
    {
        let mut append = ScratchAppend::new(&mut scratch);
        let result = (|| {
            for (index, value) in source.iter().enumerate() {
                append.append(
                    ScratchMapId::Default,
                    &(index as u64).to_be_bytes(),
                    value,
                )?;
                accepted += 1;
            }
            append.finish()
        })();
        assert!(
            result.is_err(),
            "a later row exceeds the working allowance"
        );
    }
    let mut seen = 0u32;
    scratch
        .visit(&mut |_, _| {
            seen += 1;
            Ok(true)
        })
        .expect("visit");
    assert!(
        accepted < source.len() as u32,
        "the walk stopped; remaining source rows were not collected"
    );
    assert!(
        seen <= accepted,
        "scratch holds only the prefix that append retained"
    );
    assert_ne!(
        seen,
        source.len() as u32,
        "the full source is not a silent collection in scratch"
    );
}

/// A refused mid-stream text put does not leave a readable cache hit
/// (or a scratch hit) for the failed key.
#[test]
fn d03_failed_append_does_not_leave_cache_hit() {
    let work = ExecutionPolicy {
        working_bytes: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let capability = ScratchCapability::on_work(&work, ScratchPolicy::from_work(&work))
        .expect("bind live ledger");
    let mut lookup = ScratchTextLookup::open(&capability).expect("open");
    lookup
        .put(b"kept", &1u64.to_be_bytes())
        .expect("first put commits then caches");
    lookup
        .put(&[0u8; 8000], &2u64.to_be_bytes())
        .expect_err("second put refuses before cache admit");
    let mut out = Vec::new();
    assert!(
        lookup
            .get_forward(b"kept", &mut out)
            .expect("committed get")
            .is_hit(),
        "successful put remains readable"
    );
    assert_eq!(out, 1u64.to_be_bytes());
    assert!(
        lookup
            .get_forward(&[0u8; 8000], &mut out)
            .expect("failed get")
            .is_miss(),
        "failed append is not a cache or scratch hit"
    );
    assert!(
        lookup
            .get_reverse(&2u64.to_be_bytes(), &mut out)
            .expect("failed reverse")
            .is_miss(),
        "failed reverse is not a cache hit"
    );
}

/// Borrowed value length comes from the live map, not a side index.
#[test]
fn d03_value_len_borrows_before_copy() {
    let work = work();
    let mut rows = ScratchRelation::new(&work, usize::MAX);
    let mut append = ScratchAppend::new(&mut rows);
    append
        .append(ScratchMapId::Default, b"row", b"abcdef")
        .expect("append");
    append.finish().expect("finish");
    let probe = rows
        .value_len(ScratchMapId::Default, b"row")
        .expect("len");
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

/// Work refusal on lookup is `Err`, not miss / false / empty.
#[test]
fn d03_lookup_work_refusal_is_error_not_miss() {
    let work = ExecutionPolicy {
        work_units: 0,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let capability = ScratchCapability::on_work(&work, ScratchPolicy::from_work(&work))
        .expect("bind live ledger");
    let mut lookup = ScratchTextLookup::open(&capability).expect("open");
    let error = lookup
        .lookup_forward(b"any", |_| Ok(()))
        .expect_err("zero work units refuse the lookup");
    let crate::Error::Store(store) = error else {
        panic!("lookup failure is a typed store/work error, not a miss");
    };
    assert!(
        matches!(
            store.as_ref(),
            StoreError::Work(WorkError::Exhausted { .. })
        ),
        "cancelled/exhausted work, not inequality"
    );
}
