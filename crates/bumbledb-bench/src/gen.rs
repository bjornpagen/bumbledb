//! The deterministic corpus generator (docs/architecture/50-validation.md):
//! seeded, streaming, skewed ledger data at three scales. Identical
//! config ⇒ identical bytes, forever — corpora are never stored, always
//! regenerated.
//!
//! Every row's content derives from a per-row RNG seeded by
//! `(seed, relation, row index)`, so streams are restartable, random-
//! access, and independent across relations by construction.

use crate::schema::ids;
use bumbledb::{RelationId, Value};

/// The house LCG (the engine's test constants): deterministic, fast, and
/// dependency-free.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            // Scramble the seed so small seeds diverge immediately.
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 33
    }

    /// A value in `0..n` (`n > 0`).
    pub fn range(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        self.u64() % n
    }

    /// True with probability `num/den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.range(den) < num
    }
}

/// Corpus scale points (docs/architecture/50-validation.md: 10⁵–10⁷).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    S,
    M,
    L,
}

impl Scale {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::M => "M",
            Self::L => "L",
        }
    }
}

/// The corpus identity: seed + scale. Everything else derives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenConfig {
    pub seed: u64,
    pub scale: Scale,
}

/// Derived per-relation row counts (the documented size table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sizes {
    pub postings: u64,
    pub transfers: u64,
    pub accounts: u64,
    pub holders: u64,
    pub instruments: u64,
    pub currencies: u64,
    pub tags: u64,
    pub account_tags: u64,
    pub tag_notes: u64,
}

impl Sizes {
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        let postings: u64 = match scale {
            Scale::S => 100_000,
            Scale::M => 1_000_000,
            Scale::L => 10_000_000,
        };
        let accounts = postings / 200;
        let account_tags = accounts * 2;
        Self {
            postings,
            transfers: postings / 2,
            accounts,
            holders: accounts / 4,
            instruments: 512,
            currencies: 16,
            tags: 256,
            account_tags,
            tag_notes: account_tags / 4,
        }
    }

    /// Rows for one relation.
    #[must_use]
    pub fn rows(&self, rel: RelationId) -> u64 {
        match rel {
            ids::CURRENCY => self.currencies,
            ids::HOLDER => self.holders,
            ids::INSTRUMENT => self.instruments,
            ids::ACCOUNT => self.accounts,
            ids::TRANSFER => self.transfers,
            ids::POSTING => self.postings,
            ids::TAG => self.tags,
            ids::ACCOUNT_TAG => self.account_tags,
            ids::TAG_NOTE => self.tag_notes,
            _ => unreachable!("nine ledger relations"),
        }
    }

    /// The hot-account set: the first `max(1, accounts/1000)` account ids
    /// receive [`HOT_SHARE_PCT`]% of postings.
    #[must_use]
    pub fn hot_accounts(&self) -> u64 {
        (self.accounts / 1000).max(1)
    }
}

/// Share of postings routed to the hot set, in percent.
pub const HOT_SHARE_PCT: u64 = 50;

/// The memo vocabulary size (interning realism); 1-in-[`UNIQUE_MEMO_DEN`]
/// postings carry a never-repeated memo instead.
pub const MEMO_VOCAB: u64 = 4096;
pub const UNIQUE_MEMO_DEN: u64 = 64;

/// Timestamps: base + `i × AT_STEP` + jitter in `0..AT_STEP`; the range
/// family's fixed window ([`range_window`]) selects ≈2% of postings.
pub const AT_BASE: i64 = 1_700_000_000_000_000;
pub const AT_STEP: i64 = 50;

/// The range family's `[start, end)` window over posting timestamps —
/// ≈2% of the corpus by construction.
///
/// # Panics
///
/// Only on a programmer-invariant violation: a posting count whose span
/// exceeds i64 (the scale table tops out at 10⁷).
#[must_use]
pub fn range_window(sizes: &Sizes) -> (i64, i64) {
    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let start = AT_BASE + span / 4;
    (start, start + span / 50)
}

/// The `SQLite` mapping axiom (docs/architecture/50-validation.md): every `u64` stays below
/// 2⁶³ so INTEGER columns compare correctly.
fn checked_id(id: u64) -> u64 {
    assert!(id < 1 << 63, "the SQLite mapping axiom: u64 < 2^63");
    id
}

fn mix(seed: u64, rel: RelationId, row: u64) -> u64 {
    // splitmix-style avalanche over the triple.
    let mut z = seed ^ (u64::from(rel.0) << 56) ^ row;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One `AccountTag` pair by index — shared by `AccountTag` and `TagNote`
/// (the subset-by-construction compound FK). Two distinct pairs per
/// account, without rejection: pair `k` of account `a` uses tag
/// `(a + k*97) % tags` (97 coprime to 256) — except that hot accounts
/// always carry **tag 0** as their `k = 0` pair (the skew family's
/// guarantee, docs/architecture/50-validation.md).
///
/// # Panics
///
/// Only on a programmer-invariant violation: a hot-account count reaching
/// `tags - 97`, where a hot account's `k = 1` tag would collide with the
/// pinned tag 0 (the scale table tops out at 50 hot accounts).
#[must_use]
pub fn account_tag_pair(sizes: &Sizes, i: u64) -> (u64, u64) {
    let account = i / 2;
    let k = i % 2;
    assert!(
        sizes.hot_accounts() < sizes.tags - 97,
        "hot-account ids stay below the k = 1 collision point"
    );
    let tag = if k == 0 && account < sizes.hot_accounts() {
        0
    } else {
        (account + k * 97) % sizes.tags
    };
    (account, tag)
}

fn memo(rng: &mut Rng, row: u64) -> Value {
    let text = if rng.chance(1, UNIQUE_MEMO_DEN) {
        format!("uniq-{row}")
    } else {
        format!("m{}", rng.range(MEMO_VOCAB))
    };
    Value::String(text.into_bytes().into())
}

/// One relation's row, by index — the pure function everything streams
/// from.
///
/// # Panics
///
/// Only on programmer-invariant violations: an unknown relation id, or
/// derived values exceeding their documented ranges (the size table and
/// the `SQLite` mapping axiom bound everything).
#[must_use]
pub fn row(cfg: &GenConfig, sizes: &Sizes, rel: RelationId, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(cfg.seed, rel, i));
    match rel {
        ids::CURRENCY => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("CUR{i:02}").into_bytes().into()),
        ],
        ids::HOLDER => vec![
            Value::U64(checked_id(i)),
            Value::String(
                format!("holder-{}", rng.range(MEMO_VOCAB))
                    .into_bytes()
                    .into(),
            ),
            Value::Enum(u8::try_from(rng.range(4)).expect("4 regions")),
        ],
        ids::INSTRUMENT => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("SYM{i:04}").into_bytes().into()),
            Value::U64(rng.range(sizes.currencies)),
            Value::Enum(u8::try_from(rng.range(4)).expect("4 kinds")),
        ],
        ids::ACCOUNT => vec![
            Value::U64(checked_id(i)),
            Value::U64(rng.range(sizes.holders)),
            Value::U64(rng.range(sizes.currencies)),
            // status: 90% Open.
            Value::Enum(if rng.chance(9, 10) {
                0
            } else {
                u8::try_from(1 + rng.range(2)).expect("3 statuses")
            }),
            Value::I64(AT_BASE - i64::try_from(rng.range(1 << 30)).expect("fits")),
        ],
        ids::TRANSFER => {
            let mut extref = Vec::with_capacity(16);
            for _ in 0..2 {
                extref.extend_from_slice(&rng.u64().to_le_bytes());
            }
            vec![
                Value::U64(checked_id(i)),
                Value::I64(AT_BASE + i64::try_from(i).expect("fits") * AT_STEP * 2),
                Value::Bytes(extref.into()),
            ]
        }
        ids::POSTING => {
            let account = if rng.chance(HOT_SHARE_PCT, 100) {
                rng.range(sizes.hot_accounts())
            } else {
                rng.range(sizes.accounts)
            };
            let amount = {
                let magnitude = 1 + rng.range(5_000_000);
                let signed = i64::try_from(magnitude).expect("fits");
                if rng.chance(1, 2) {
                    signed
                } else {
                    -signed
                }
            };
            let at = AT_BASE
                + i64::try_from(i).expect("fits") * AT_STEP
                + i64::try_from(rng.range(u64::try_from(AT_STEP).expect("positive")))
                    .expect("fits");
            vec![
                Value::U64(checked_id(i)),
                Value::U64(rng.range(sizes.transfers)),
                Value::U64(account),
                Value::U64(rng.range(sizes.instruments)),
                Value::I64(amount),
                Value::I64(at),
                memo(&mut rng, i),
                Value::Bool(rng.chance(3, 4)),
            ]
        }
        ids::TAG => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("tag-{i:03}").into_bytes().into()),
        ],
        ids::ACCOUNT_TAG => {
            let (account, tag) = account_tag_pair(sizes, i);
            vec![Value::U64(account), Value::U64(tag)]
        }
        ids::TAG_NOTE => {
            // Every 4th AccountTag pair carries a note — a subset by
            // construction, so the compound FK always resolves.
            let (account, tag) = account_tag_pair(sizes, i * 4);
            vec![
                Value::U64(account),
                Value::U64(tag),
                Value::String(
                    format!("note-{}", rng.range(MEMO_VOCAB))
                        .into_bytes()
                        .into(),
                ),
            ]
        }
        _ => unreachable!("nine ledger relations"),
    }
}

/// One relation's full row stream — O(1) memory, deterministically
/// restartable (regenerate, never store).
pub fn relation_rows(cfg: GenConfig, rel: RelationId) -> impl Iterator<Item = Vec<Value>> + Clone {
    let sizes = Sizes::of(cfg.scale);
    (0..sizes.rows(rel)).map(move |i| row(&cfg, &sizes, rel, i))
}

/// Canonical bytes of one value, for the corpus digest (length-prefixed
/// variable content; fixed-width scalars little-endian).
fn value_bytes(digest: &mut bumbledb::digest::Digest, value: &Value) {
    match value {
        Value::Bool(v) => digest.update(&[0, u8::from(*v)]),
        Value::U64(v) => {
            digest.update(&[1]);
            digest.update(&v.to_le_bytes());
        }
        Value::I64(v) => {
            digest.update(&[2]);
            digest.update(&v.to_le_bytes());
        }
        Value::Enum(v) => digest.update(&[3, *v]),
        Value::String(raw) => {
            digest.update(&[4]);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(raw);
        }
        Value::Bytes(raw) => {
            digest.update(&[5]);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(raw);
        }
    }
}

/// The corpus identity: a blake3 over every relation's streamed rows.
/// Stamps, cache directories, and reports key on this.
#[must_use]
pub fn corpus_digest(cfg: GenConfig) -> [u8; 32] {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(&cfg.seed.to_le_bytes());
    digest.update(cfg.scale.label().as_bytes());
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        digest.update(&rel.0.to_le_bytes());
        for row in relation_rows(cfg, rel) {
            for value in &row {
                value_bytes(&mut digest, value);
            }
        }
    }
    digest.finalize()
}

/// Hex rendering of a digest (directory names, stamps, goldens).
#[must_use]
pub fn digest_hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    digest.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
