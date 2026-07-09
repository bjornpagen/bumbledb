//! The generator's target schema seam.
//!
//! The query grammar is schema-specific by design
//! (`docs/architecture/60-validation.md` owns the ledger shape); this
//! module is the one place the grammar touches a concrete schema:
//! relation/field ids, per-relation domains, literal vocabularies, and
//! the deterministic corpus value functions the dressing recomputes
//! (in-vocabulary hits are *actual* seeded values, never guesses). The
//! ledger rebuild (PRD 24) replaces this module's declarations with the
//! bench schema and its corpus; the grammar above does not change.
//!
//! The declared ledger is `60-validation.md`'s, with the two coverage
//! extensions the seven-type matrix needs: `Posting.{memo, reconciled}`
//! (interned-string vocabulary and the Bool column) and
//! `Transfer { extref: bytes, window: interval<u64> }` (the Bytes
//! exerciser and the U64-element interval lane; `Mandate.active` is the
//! I64-element lane).

use std::sync::OnceLock;

use bumbledb::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType,
};
use bumbledb::{Schema, Value};

use crate::gen::{GenConfig, Rng, Scale};
use crate::querygen::interval_data;

/// Relation and field ids by name — declaration order is the id order,
/// no magic numbers in the grammar.
pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const HOLDER: RelationId = RelationId(0);
    pub const ACCOUNT: RelationId = RelationId(1);
    pub const INSTRUMENT: RelationId = RelationId(2);
    pub const JOURNAL_ENTRY: RelationId = RelationId(3);
    pub const POSTING: RelationId = RelationId(4);
    pub const POSTING_TAG: RelationId = RelationId(5);
    pub const ORG: RelationId = RelationId(6);
    pub const ORG_PARENT: RelationId = RelationId(7);
    pub const MANDATE: RelationId = RelationId(8);
    pub const TRANSFER: RelationId = RelationId(9);

    pub mod holder {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod account {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const HOLDER: FieldId = FieldId(1);
        pub const CURRENCY: FieldId = FieldId(2);
    }
    pub mod instrument {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SYMBOL: FieldId = FieldId(1);
    }
    pub mod journal_entry {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SOURCE: FieldId = FieldId(1);
        pub const CREATED_AT: FieldId = FieldId(2);
    }
    pub mod posting {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const ENTRY: FieldId = FieldId(1);
        pub const ACCOUNT: FieldId = FieldId(2);
        pub const INSTRUMENT: FieldId = FieldId(3);
        pub const AMOUNT: FieldId = FieldId(4);
        pub const AT: FieldId = FieldId(5);
        pub const MEMO: FieldId = FieldId(6);
        pub const RECONCILED: FieldId = FieldId(7);
    }
    pub mod posting_tag {
        use super::FieldId;
        pub const POSTING: FieldId = FieldId(0);
        pub const TAG: FieldId = FieldId(1);
    }
    pub mod org {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod org_parent {
        use super::FieldId;
        pub const CHILD: FieldId = FieldId(0);
        pub const PARENT: FieldId = FieldId(1);
    }
    pub mod mandate {
        use super::FieldId;
        pub const ACCOUNT: FieldId = FieldId(0);
        pub const ORG: FieldId = FieldId(1);
        pub const ACTIVE: FieldId = FieldId(2);
    }
    pub mod transfer {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const EXTREF: FieldId = FieldId(1);
        pub const WINDOW: FieldId = FieldId(2);
    }
}

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn serial(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Serial,
    }
}

fn enum_type(variants: &[&str]) -> ValueType {
    ValueType::Enum {
        variants: variants.iter().map(|v| Box::from(*v)).collect(),
    }
}

/// The target ledger, sealed. Statements are the write side's concern;
/// the query grammar needs only relations and field types, so none are
/// declared here (the serial auto-keys still materialize — coverage
/// derives key-coveredness from the `Serial` generation attribute).
///
/// # Panics
///
/// Never in practice: the declared ledger passes the acceptance gate
/// (asserted on first use).
pub fn schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Holder".into(),
                    fields: vec![serial("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    name: "Account".into(),
                    fields: vec![
                        serial("id"),
                        field("holder", ValueType::U64),
                        field("currency", enum_type(&["Usd", "Eur", "Gbp"])),
                    ],
                },
                RelationDescriptor {
                    name: "Instrument".into(),
                    fields: vec![serial("id"), field("symbol", ValueType::String)],
                },
                RelationDescriptor {
                    name: "JournalEntry".into(),
                    fields: vec![
                        serial("id"),
                        field("source", enum_type(&["Manual", "Import", "System"])),
                        field("created_at", ValueType::I64),
                    ],
                },
                RelationDescriptor {
                    name: "Posting".into(),
                    fields: vec![
                        serial("id"),
                        field("entry", ValueType::U64),
                        field("account", ValueType::U64),
                        field("instrument", ValueType::U64),
                        field("amount", ValueType::I64),
                        field("at", ValueType::I64),
                        field("memo", ValueType::String),
                        field("reconciled", ValueType::Bool),
                    ],
                },
                RelationDescriptor {
                    name: "PostingTag".into(),
                    fields: vec![
                        field("posting", ValueType::U64),
                        field("tag", enum_type(&["Fee", "Rebate", "Adjustment"])),
                    ],
                },
                RelationDescriptor {
                    name: "Org".into(),
                    fields: vec![serial("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    name: "OrgParent".into(),
                    fields: vec![
                        field("child", ValueType::U64),
                        field("parent", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    name: "Mandate".into(),
                    fields: vec![
                        field("account", ValueType::U64),
                        field("org", ValueType::U64),
                        field(
                            "active",
                            ValueType::Interval {
                                element: IntervalElement::I64,
                            },
                        ),
                    ],
                },
                RelationDescriptor {
                    name: "Transfer".into(),
                    fields: vec![
                        serial("id"),
                        field("extref", ValueType::Bytes),
                        field(
                            "window",
                            ValueType::Interval {
                                element: IntervalElement::U64,
                            },
                        ),
                    ],
                },
            ],
            statements: vec![],
        }
        .validate()
        .expect("the target ledger validates")
    })
}

/// Derived per-relation domains (dense ids are `0..n`) — the dressing
/// draws literals in-domain so predicates select real subsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Domains {
    pub postings: u64,
    pub entries: u64,
    pub accounts: u64,
    pub holders: u64,
    pub instruments: u64,
    pub orgs: u64,
    pub mandates: u64,
    pub transfers: u64,
    pub posting_tags: u64,
}

impl Domains {
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        let postings: u64 = match scale {
            Scale::S => 100_000,
            Scale::M => 1_000_000,
            Scale::L => 10_000_000,
        };
        let accounts = postings / 200;
        Self {
            postings,
            entries: postings / 2,
            accounts,
            holders: (accounts / 4).max(1),
            instruments: 512,
            orgs: 64,
            mandates: accounts * interval_data::PER_GROUP,
            transfers: postings / 2,
            posting_tags: postings,
        }
    }
}

/// The memo vocabulary size (interning realism).
pub const MEMO_VOCAB: u64 = 4096;

/// Timestamps: `AT_BASE + row × AT_STEP`, strictly monotone — every
/// posting's `at` is distinct by construction, so `at` is the **tie-free
/// Arg key** (`docs/architecture/20-query-ir.md`: with distinct keys
/// ties cannot occur).
pub const AT_BASE: i64 = 1_700_000_000_000_000;
pub const AT_STEP: i64 = 50;

/// Amounts quantized to [`AMOUNT_LEVELS`] values — within any account's
/// posting group the extreme is attained by many rows, so `amount` is
/// the **tie-rich Arg key**: tie data constructed, not hoped for.
pub const AMOUNT_LEVELS: i64 = 8;
pub const AMOUNT_STEP: i64 = 1_000;

/// A per-row generator seeded by `(seed, relation tag, row)` — streams
/// are restartable and random-access, so dressing recomputes any row's
/// values without materializing a corpus.
fn row_rng(seed: u64, tag: u64, row: u64) -> Rng {
    Rng::new(
        seed ^ tag.wrapping_mul(0xA24B_AED4_963E_E407) ^ row.wrapping_mul(0x9FB2_1C65_1E98_DF25),
    )
}

/// The seeded extref of one Transfer row — 16 bytes, a pure function of
/// the config, so in-vocabulary Bytes literals recompute exactly.
#[must_use]
pub fn extref(cfg: GenConfig, row: u64) -> Value {
    let mut rng = row_rng(cfg.seed, u64::from(ids::TRANSFER.0), row);
    let mut raw = Vec::with_capacity(16);
    for _ in 0..2 {
        raw.extend_from_slice(&rng.u64().to_le_bytes());
    }
    Value::Bytes(raw.into())
}

/// One posting's quantized amount (see [`AMOUNT_LEVELS`]).
///
/// # Panics
///
/// On a programmer-invariant violation only (the level arithmetic fits
/// its domain).
#[must_use]
pub fn posting_amount(cfg: GenConfig, row: u64) -> i64 {
    let mut rng = row_rng(cfg.seed, u64::from(ids::POSTING.0), row);
    let level =
        i64::try_from(rng.range(u64::try_from(AMOUNT_LEVELS).expect("positive"))).expect("small");
    (level - AMOUNT_LEVELS / 2) * AMOUNT_STEP
}

/// One posting's strictly monotone timestamp.
///
/// # Panics
///
/// On a corpus row index past `i64::MAX` — unreachable at every scale.
#[must_use]
pub fn posting_at(row: u64) -> i64 {
    AT_BASE + i64::try_from(row).expect("corpus rows fit") * AT_STEP
}

/// One `PostingTag` row: `(posting, tag ordinal)`. Even postings carry
/// **two** tags (rows `2p` and `2p + 1`), odd postings none — the
/// negated side's duplicate-witness exerciser is data, by construction:
/// `¬PostingTag(posting = v)` must reject a doubly-tagged posting
/// exactly as it rejects a singly-witnessed one, and must pass the
/// tagless half.
///
/// # Panics
///
/// Never: `row % 3` fits the three-variant ordinal by construction.
#[must_use]
pub fn posting_tag(row: u64) -> (u64, u8) {
    let posting = (row / 2) * 2;
    let tag = u8::try_from(row % 3).expect("three variants");
    (posting, tag)
}

/// One Mandate row: `(account, org, active)`. Mandate row `r` is
/// interval `r % PER_GROUP` of collision group `r / PER_GROUP`; the
/// group's scalar prefix is its account, so every account carries
/// [`interval_data::PER_GROUP`] intervals — the shape the judgments and
/// interval joins discriminate.
#[must_use]
pub fn mandate(cfg: GenConfig, domains: &Domains, row: u64) -> (u64, u64, (i64, i64)) {
    let group = row / interval_data::PER_GROUP;
    let k = row % interval_data::PER_GROUP;
    let account = group % domains.accounts.max(1);
    let org = group % domains.orgs.max(1);
    (account, org, interval_data::group_i64(cfg.seed, group, k))
}

/// One Transfer row's window: transfer row `r` is interval
/// `r % PER_GROUP` of U64-element collision group `r / PER_GROUP`.
#[must_use]
pub fn transfer_window(cfg: GenConfig, row: u64) -> (u64, u64) {
    interval_data::group_u64(
        cfg.seed,
        row / interval_data::PER_GROUP,
        row % interval_data::PER_GROUP,
    )
}

/// An in-vocabulary string for a (relation, field) — the corpus
/// vocabulary the string dressing hits.
#[must_use]
pub fn string_hit(rel: bumbledb::RelationId, field: bumbledb::FieldId, rng: &mut Rng) -> String {
    match (rel, field) {
        (ids::HOLDER, ids::holder::NAME) => format!("holder-{}", rng.range(MEMO_VOCAB)),
        (ids::INSTRUMENT, ids::instrument::SYMBOL) => format!("SYM{:04}", rng.range(512)),
        (ids::ORG, ids::org::NAME) => format!("org-{:02}", rng.range(64)),
        _ => format!("m{}", rng.range(MEMO_VOCAB)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: GenConfig = GenConfig {
        seed: 7,
        scale: Scale::S,
    };

    /// Ties are constructed: within one account's posting rows the
    /// maximum amount is attained more than once (postings are dealt to
    /// accounts round-robin at `row % accounts` in family fixtures; any
    /// 400-row slice of one account draws from 8 quantized levels).
    #[test]
    fn amounts_carry_constructed_ties() {
        let domains = Domains::of(Scale::S);
        let account_rows: Vec<u64> = (0..domains.postings)
            .filter(|row| row % domains.accounts == 3)
            .take(64)
            .collect();
        let amounts: Vec<i64> = account_rows
            .iter()
            .map(|row| posting_amount(CFG, *row))
            .collect();
        let max = amounts.iter().max().expect("nonempty");
        let attained = amounts.iter().filter(|a| *a == max).count();
        assert!(attained >= 2, "the maximum is attained {attained} time(s)");
    }

    /// `at` is strictly monotone — the tie-free key.
    #[test]
    fn at_is_tie_free() {
        assert!(posting_at(0) < posting_at(1));
        assert!(posting_at(41) < posting_at(42));
    }

    /// Even postings carry two tags, odd none — multiply-witnessed and
    /// unwitnessed negated sides both exist by construction.
    #[test]
    fn posting_tags_are_multiply_witnessed() {
        let (p0, t0) = posting_tag(0);
        let (p1, t1) = posting_tag(1);
        assert_eq!(p0, p1, "rows 0 and 1 tag the same posting");
        assert_ne!(t0, t1, "with distinct tags");
        let tagged: std::collections::BTreeSet<u64> =
            (0..100).map(|row| posting_tag(row).0).collect();
        assert!(!tagged.contains(&1), "odd postings are tagless");
    }

    /// Extrefs recompute exactly (the dressing's in-vocabulary hits).
    #[test]
    fn extref_recomputes() {
        assert_eq!(extref(CFG, 9), extref(CFG, 9));
        assert_ne!(extref(CFG, 9), extref(CFG, 10));
    }

    /// Every mandate row's interval is valid and the group prefix
    /// collides: `PER_GROUP` rows share one account.
    #[test]
    fn mandate_groups_collide_on_account() {
        let domains = Domains::of(Scale::S);
        let first: Vec<_> = (0..interval_data::PER_GROUP)
            .map(|row| mandate(CFG, &domains, row))
            .collect();
        assert!(first.iter().all(|(account, _, _)| *account == first[0].0));
        for (_, _, (start, end)) in first {
            assert!(start < end);
        }
    }
}
