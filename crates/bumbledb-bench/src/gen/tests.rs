use super::*;
use bumbledb::Value;

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
    // The golden: changing the generator re-baselines every corpus.
    assert_eq!(
        digest_hex(&a),
        "12d08c93fe2b654aa74fbe1f1a5e84fa255e805e284e6d216ba1702f2ddc1af0",
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
fn memos_draw_from_the_vocabulary_plus_rare_uniques() {
    let mut vocab = std::collections::HashSet::new();
    let mut uniques = 0u64;
    for r in relation_rows(CFG, ids::POSTING) {
        let Value::String(raw) = &r[6] else {
            panic!("memo")
        };
        if raw.starts_with(b"uniq-") {
            uniques += 1;
        } else {
            vocab.insert(raw.clone());
        }
    }
    assert!(vocab.len() as u64 <= MEMO_VOCAB);
    assert!(vocab.len() as u64 > MEMO_VOCAB / 2, "{}", vocab.len());
    let expected = Sizes::of(Scale::S).postings / UNIQUE_MEMO_DEN;
    assert!(
        uniques > expected * 8 / 10 && uniques < expected * 12 / 10,
        "uniques {uniques} vs expected {expected}"
    );
}

#[test]
fn foreign_keys_close_by_construction() {
    let sizes = Sizes::of(Scale::S);
    let mut rng = Rng::new(7);
    for _ in 0..1000 {
        let i = rng.range(sizes.postings);
        let r = row(&CFG, &sizes, ids::POSTING, i);
        let (Value::U64(transfer), Value::U64(account), Value::U64(instrument)) =
            (&r[1], &r[2], &r[3])
        else {
            panic!("fk columns")
        };
        assert!(*transfer < sizes.transfers);
        assert!(*account < sizes.accounts);
        assert!(*instrument < sizes.instruments);
    }
    // TagNote pairs are a subset of AccountTag pairs by construction.
    let pairs: std::collections::HashSet<(u64, u64)> = relation_rows(CFG, ids::ACCOUNT_TAG)
        .map(|r| {
            let (Value::U64(a), Value::U64(t)) = (&r[0], &r[1]) else {
                panic!("pair")
            };
            (*a, *t)
        })
        .collect();
    assert_eq!(pairs.len() as u64, sizes.account_tags, "pairs distinct");
    for r in relation_rows(CFG, ids::TAG_NOTE) {
        let (Value::U64(a), Value::U64(t)) = (&r[0], &r[1]) else {
            panic!("pair")
        };
        assert!(pairs.contains(&(*a, *t)), "({a}, {t}) must exist");
    }
}
