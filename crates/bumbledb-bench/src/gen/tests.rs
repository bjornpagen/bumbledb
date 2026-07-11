use super::*;
use bumbledb::{Interval, Value};

use crate::schema::ids;

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

#[test]
fn the_corpus_digest_is_deterministic_and_pinned() {
    let a = corpus_digest(CFG);
    let b = corpus_digest(CFG);
    assert_eq!(a, b, "same config, same bytes");
    let other = corpus_digest(GenConfig {
        seed: 2,
        scale: Scale::S,
    });
    assert_ne!(a, other, "seeds diverge");
    // The golden: changing the generator — or the storage format, now a
    // live ingredient — re-baselines every corpus. Re-baselined when
    // the calendar theory's rows joined the digest (ALG 16 — one
    // corpus identity, both theories inside).
    assert_eq!(
        digest_hex(&a),
        "96b067335ede49bd5d8a6db0989e14a6d4a81336f523ac2696a66db1bb8160fd",
        "generator output changed — re-baseline deliberately"
    );
}

#[test]
fn hot_accounts_receive_their_share() {
    let sizes = Sizes::of(Scale::S);
    let hot = sizes.hot_accounts();
    let mut hot_postings = 0u64;
    for r in relation_rows(CFG, ids::POSTING) {
        let Value::U64(account) = r[2] else {
            panic!("account column")
        };
        if account < hot {
            hot_postings += 1;
        }
    }
    let share = hot_postings * 100 / sizes.postings;
    // 50% routed + the uniform arm occasionally landing in the hot
    // range too (hot/accounts is tiny at S) — bound generously.
    assert!((48..=53).contains(&share), "hot share {share}% (hot={hot})");
}

#[test]
fn ids_are_dense_and_under_the_sqlite_bound() {
    let sizes = Sizes::of(Scale::S);
    for (idx, r) in relation_rows(CFG, ids::ACCOUNT).enumerate() {
        let Value::U64(id) = r[0] else { panic!("id") };
        assert_eq!(id, idx as u64, "dense 0..n");
        assert!(id < 1 << 63);
    }
    assert_eq!(
        relation_rows(CFG, ids::ACCOUNT).count() as u64,
        sizes.accounts
    );
}

#[test]
fn the_range_window_selects_about_two_percent() {
    let sizes = Sizes::of(Scale::S);
    let (start, end) = range_window(&sizes);
    let mut selected = 0u64;
    for r in relation_rows(CFG, ids::POSTING) {
        let Value::I64(at) = r[5] else { panic!("at") };
        if (start..end).contains(&at) {
            selected += 1;
        }
    }
    let permille = selected * 1000 / sizes.postings;
    assert!((15..=30).contains(&permille), "window selects {permille}‰");
}

#[test]
fn containment_sources_resolve_by_construction() {
    let sizes = Sizes::of(Scale::S);
    let mut rng = Rng::new(7);
    for _ in 0..1000 {
        let i = rng.range(sizes.postings);
        let r = row(&CFG, &sizes, ids::POSTING, i);
        let (Value::U64(entry), Value::U64(account), Value::U64(instrument)) =
            (&r[1], &r[2], &r[3])
        else {
            panic!("containment columns")
        };
        assert!(*entry < sizes.entries);
        assert!(*account < sizes.accounts);
        assert!(*instrument < sizes.instruments);
    }
    // PostingTag targets even postings only, with distinct tag pairs.
    for pair in 0..64 {
        let a = row(&CFG, &sizes, ids::POSTING_TAG, pair * 2);
        let b = row(&CFG, &sizes, ids::POSTING_TAG, pair * 2 + 1);
        assert_eq!(a[0], b[0], "one posting per pair");
        assert_ne!(a[1], b[1], "distinct tags per posting");
        let Value::U64(posting) = a[0] else {
            panic!("posting")
        };
        assert!(posting.is_multiple_of(2) && posting < sizes.postings);
    }
    // OrgParent edges reference existing orgs, acyclically.
    for r in relation_rows(CFG, ids::ORG_PARENT) {
        let (Value::U64(child), Value::U64(parent)) = (&r[0], &r[1]) else {
            panic!("edge")
        };
        assert!(*child < sizes.orgs && *parent < sizes.orgs);
        assert!(parent < child, "parents precede children — no cycles");
    }
}

/// The corpus-validity criterion, structural half: every
/// account's mandate history is sequential and non-overlapping under
/// the pointwise key, and the three boundary shapes all exist —
/// **abutting** (every account, segments 0→1), **gapped** (every
/// account, segments 1→2), and the **ray end** (every even
/// account).
#[test]
fn mandate_histories_carry_all_three_shapes_validly() {
    let sizes = Sizes::of(Scale::S);
    let (mut abutting, mut gapped, mut sentinel) = (0u64, 0u64, 0u64);
    for account in 0..sizes.accounts {
        let segments = mandate_segments(CFG.seed, &sizes, account);
        for segment in &segments {
            assert!(segment.start < segment.end, "nonempty interval");
            assert!(segment.org < sizes.orgs);
        }
        for pair in segments.windows(2) {
            assert!(
                pair[0].end <= pair[1].start,
                "account {account}: segments overlap — the pointwise key would abort"
            );
            if pair[0].end == pair[1].start {
                abutting += 1;
            } else {
                gapped += 1;
            }
        }
        if segments[3].end == Interval::<i64>::MAX_END {
            sentinel += 1;
        }
    }
    assert!(abutting >= sizes.accounts, "one abutting pair per account");
    assert!(gapped >= sizes.accounts, "one gapped pair per account");
    assert_eq!(sentinel, sizes.accounts / 2, "even accounts stay active");
}

/// The mandate rows stream exactly the segment table (the row fn and
/// the segment fn cannot drift apart).
#[test]
fn mandate_rows_stream_the_segment_table() {
    let sizes = Sizes::of(Scale::S);
    for i in [0u64, 1, 2, 3, 4, 7, 401] {
        let r = row(&CFG, &sizes, ids::MANDATE, i);
        let segment = mandate_segments(CFG.seed, &sizes, i / MANDATE_SEGMENTS)
            [usize::try_from(i % MANDATE_SEGMENTS).expect("small")];
        assert_eq!(
            r,
            vec![
                Value::U64(i / MANDATE_SEGMENTS),
                Value::U64(segment.org),
                Value::IntervalI64(segment.start, segment.end),
            ]
        );
    }
}
