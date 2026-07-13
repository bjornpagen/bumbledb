//! The generator's target schema seam.
//!
//! The query grammar is schema-specific by design
//! (`docs/architecture/60-validation.md` owns the ledger shape); this
//! module is the one place the grammar touches a concrete schema:
//! relation/field ids, per-relation domains, literal vocabularies, and
//! the deterministic corpus value functions the dressing recomputes
//! (in-vocabulary hits are *actual* seeded values, never guesses). The
//! declarations here are the bench schema and its corpus; the grammar
//! above does not depend on them.
//!
//! The declared ledger is `60-validation.md`'s, with the coverage
//! extensions the six-type matrix needs: `Posting.{memo, reconciled}`
//! (interned-string vocabulary and the Bool column) and
//! `Transfer { extref: bytes<32>, window: interval<u64>, tag7..tag64 }`
//! (the bytes<N> exerciser — extref is a keyed adversarial digest and
//! the tags cover the pad-boundary widths 7/8/9/16/63/64 — and the
//! U64-element interval lane; `Mandate.active` is the I64-element
//! lane) — plus, for the chase shapes (`shapes_chase.rs`),
//! the ledger's containment statements and one discriminated-union pair
//! `JournalEntry(id | source == Import) == ImportBatch(entry)`, so the
//! randomized lane exercises the occurrence elimination and its
//! refusals against corpora that satisfy the statements by
//! construction (`docs/architecture/40-execution.md` § the chase) —
//! plus the closed-relation write surface (PRD 06): `Currency` carries
//! a payload column (`minor_units`), `CurrencyBacking` is the small
//! keyed relation the domain quantification `Currency(id) <=
//! CurrencyBacking(currency)` targets, and `CashRounding` rides the
//! ψ-sub-vocabulary `Currency(id | minor_units == 0)` — the three
//! write-scenario classes and the closed query shapes draw from here.

use std::sync::OnceLock;

use bumbledb::schema::{IntervalElement, RelationDescriptor, Row, SchemaDescriptor, ValueType};
use bumbledb::{Schema, Value};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::fixture::{field, fresh};
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
    pub const IMPORT_BATCH: RelationId = RelationId(10);
    pub const CURRENCY_BACKING: RelationId = RelationId(11);
    pub const CASH_ROUNDING: RelationId = RelationId(12);
    pub const CURRENCY: RelationId = RelationId(13);
    pub const SOURCE: RelationId = RelationId(14);
    pub const TAG: RelationId = RelationId(15);

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
        /// The pad-boundary digest tags, in width order 7/8/9/16/63/64.
        pub const TAGS: [FieldId; 6] = [
            FieldId(3),
            FieldId(4),
            FieldId(5),
            FieldId(6),
            FieldId(7),
            FieldId(8),
        ];
    }
    pub mod import_batch {
        use super::FieldId;
        pub const ENTRY: FieldId = FieldId(0);
        pub const BATCH: FieldId = FieldId(1);
    }
    pub mod currency_backing {
        use super::FieldId;
        pub const CURRENCY: FieldId = FieldId(0);
        pub const RESERVE: FieldId = FieldId(1);
    }
    pub mod cash_rounding {
        use super::FieldId;
        pub const CURRENCY: FieldId = FieldId(0);
    }
    /// A closed relation's fields sit in the **sealed** space: the
    /// synthetic id at 0, then the declared payload columns.
    pub mod currency {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const MINOR_UNITS: FieldId = FieldId(1);
    }
}

/// A closed relation: no payload columns, the extension rows are the
/// handles alone (the engine prepends the synthetic `(id, U64)` field).
fn closed(name: &str, handles: &[&str]) -> RelationDescriptor {
    RelationDescriptor {
        extension: Some(
            handles
                .iter()
                .map(|handle| Row {
                    handle: (*handle).into(),
                    values: Box::new([]),
                })
                .collect(),
        ),
        name: name.into(),
        fields: vec![],
    }
}

/// The target ledger, sealed — relations for the query grammar's typing
/// walk, and the statements the chase shapes need: the ledger's nine
/// containments plus the discriminated-union pair
/// `JournalEntry(id | source == Import) == ImportBatch(entry)` (written
/// as its two containments; `ImportBatch(entry) -> ImportBatch` is the
/// declared key each direction's acceptance requires). The corpus
/// satisfies every statement by construction: every reference field
/// draws in-domain, and entry `i` has `source == Import` iff
/// `i % 3 == 1` iff `ImportBatch` row `(i - 1) / 3` exists.
///
/// The target ledger's schema definition — the value the target stores
/// are created with (`Db::create(dir, Target)`).
#[derive(Debug, Clone, Copy)]
pub struct Target;

impl bumbledb::Theory for Target {
    fn descriptor(self) -> SchemaDescriptor {
        descriptor()
    }
}

/// # Panics
///
/// Never in practice: the declared ledger passes the acceptance gate
/// (asserted on first use).
pub fn schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        descriptor()
            .validate()
            .expect("the target ledger validates")
    })
}

/// The declared target ledger, as the raw descriptor — the value the
/// naive model and the mirror's extension INSERTs consume beside the
/// sealed schema (`pub(crate)` for the closed-relation differential,
/// which drives all three write-scenario classes over this theory).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the declared ledger, one relation per block
pub(crate) fn descriptor() -> SchemaDescriptor {
    {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Holder".into(),
                    fields: vec![fresh("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        fresh("id"),
                        field("holder", ValueType::U64),
                        field("currency", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Instrument".into(),
                    fields: vec![fresh("id"), field("symbol", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "JournalEntry".into(),
                    fields: vec![
                        fresh("id"),
                        field("source", ValueType::U64),
                        field("created_at", ValueType::I64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Posting".into(),
                    fields: vec![
                        fresh("id"),
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
                    extension: None,
                    name: "PostingTag".into(),
                    fields: vec![
                        field("posting", ValueType::U64),
                        field("tag", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Org".into(),
                    fields: vec![fresh("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "OrgParent".into(),
                    fields: vec![
                        field("child", ValueType::U64),
                        field("parent", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
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
                    extension: None,
                    name: "Transfer".into(),
                    fields: {
                        let mut fields = vec![
                            fresh("id"),
                            field("extref", ValueType::FixedBytes { len: 32 }),
                            field(
                                "window",
                                ValueType::Interval {
                                    element: IntervalElement::U64,
                                },
                            ),
                        ];
                        // The pad-boundary digest tags (7/8/9/16/63/64):
                        // widths astride every word boundary, drawn from
                        // small adversarial vocabularies — shared
                        // prefixes, single-byte deltas, all-zeros.
                        for width in DIGEST_WIDTHS {
                            fields.push(field(
                                &format!("tag{width}"),
                                ValueType::FixedBytes { len: width },
                            ));
                        }
                        fields
                    },
                },
                RelationDescriptor {
                    extension: None,
                    name: "ImportBatch".into(),
                    fields: vec![
                        field("entry", ValueType::U64),
                        field("batch", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "CurrencyBacking".into(),
                    fields: vec![
                        field("currency", ValueType::U64),
                        field("reserve", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "CashRounding".into(),
                    fields: vec![field("currency", ValueType::U64)],
                },
                // Currency carries a payload column so payload-column
                // selections and ψ-sub-vocabularies exist to draw:
                // minor_units 2/2/0 — the 0 row is the ψ-member of
                // `CashRounding(currency) <= Currency(id | minor_units
                // == 0)`, giving the subset both members and
                // non-members.
                RelationDescriptor {
                    extension: Some(Box::new([
                        Row {
                            handle: "Usd".into(),
                            values: Box::new([Value::U64(2)]),
                        },
                        Row {
                            handle: "Eur".into(),
                            values: Box::new([Value::U64(2)]),
                        },
                        Row {
                            handle: "Gbp".into(),
                            values: Box::new([Value::U64(0)]),
                        },
                    ])),
                    name: "Currency".into(),
                    fields: vec![field("minor_units", ValueType::U64)],
                },
                closed("Source", &["Manual", "Import", "System"]),
                closed("Tag", &["Fee", "Rebate", "Adjustment"]),
            ],
            statements: statements(),
        }
    }
}

/// The declared statements: `ImportBatch`'s key, the ledger's nine
/// containments (`60-validation.md`'s block, in its source order), the
/// DU pair as its two containments (mirror-detected at sealing), and —
/// appended last so no earlier statement id shifts — the bytes<32> key
/// `Transfer(extref) -> Transfer`: every corpus load writes an
/// adversarial-digest guard per transfer, and an `Eq` extref binding is
/// key-covering (the guard-probe fast path over a multi-word key).
fn statements() -> Vec<bumbledb::schema::StatementDescriptor> {
    use bumbledb::schema::{Side, StatementDescriptor};
    let side = |relation: bumbledb::RelationId,
                projection: bumbledb::FieldId,
                selection: &[(bumbledb::FieldId, Value)]| Side {
        relation,
        projection: Box::new([projection]),
        selection: selection.iter().cloned().collect(),
    };
    let containment =
        |source: Side, target: Side| StatementDescriptor::Containment { source, target };
    let import = [(ids::journal_entry::SOURCE, Value::U64(SOURCE_IMPORT))];
    vec![
        StatementDescriptor::Functionality {
            relation: ids::IMPORT_BATCH,
            projection: Box::new([ids::import_batch::ENTRY]),
        },
        containment(
            side(ids::ACCOUNT, ids::account::HOLDER, &[]),
            side(ids::HOLDER, ids::holder::ID, &[]),
        ),
        containment(
            side(ids::POSTING, ids::posting::ENTRY, &[]),
            side(ids::JOURNAL_ENTRY, ids::journal_entry::ID, &[]),
        ),
        containment(
            side(ids::POSTING, ids::posting::ACCOUNT, &[]),
            side(ids::ACCOUNT, ids::account::ID, &[]),
        ),
        containment(
            side(ids::POSTING, ids::posting::INSTRUMENT, &[]),
            side(ids::INSTRUMENT, ids::instrument::ID, &[]),
        ),
        containment(
            side(ids::POSTING_TAG, ids::posting_tag::POSTING, &[]),
            side(ids::POSTING, ids::posting::ID, &[]),
        ),
        containment(
            side(ids::ORG_PARENT, ids::org_parent::CHILD, &[]),
            side(ids::ORG, ids::org::ID, &[]),
        ),
        containment(
            side(ids::ORG_PARENT, ids::org_parent::PARENT, &[]),
            side(ids::ORG, ids::org::ID, &[]),
        ),
        containment(
            side(ids::MANDATE, ids::mandate::ACCOUNT, &[]),
            side(ids::ACCOUNT, ids::account::ID, &[]),
        ),
        containment(
            side(ids::MANDATE, ids::mandate::ORG, &[]),
            side(ids::ORG, ids::org::ID, &[]),
        ),
        containment(
            side(ids::JOURNAL_ENTRY, ids::journal_entry::ID, &import),
            side(ids::IMPORT_BATCH, ids::import_batch::ENTRY, &[]),
        ),
        containment(
            side(ids::IMPORT_BATCH, ids::import_batch::ENTRY, &[]),
            side(ids::JOURNAL_ENTRY, ids::journal_entry::ID, &import),
        ),
        StatementDescriptor::Functionality {
            relation: ids::TRANSFER,
            projection: Box::new([ids::transfer::EXTREF]),
        },
        // The vocabulary containments (the enum funeral) — appended
        // after everything else so no earlier statement id shifts.
        containment(
            side(ids::ACCOUNT, ids::account::CURRENCY, &[]),
            side(ids::CURRENCY, ids::currency::ID, &[]),
        ),
        containment(
            side(ids::JOURNAL_ENTRY, ids::journal_entry::SOURCE, &[]),
            side(ids::SOURCE, bumbledb::FieldId(0), &[]),
        ),
        containment(
            side(ids::POSTING_TAG, ids::posting_tag::TAG, &[]),
            side(ids::TAG, bumbledb::FieldId(0), &[]),
        ),
        // The PRD 06 closed-relation write surface — appended last, so
        // no earlier statement id shifts:
        // the key the domain quantification probes,
        StatementDescriptor::Functionality {
            relation: ids::CURRENCY_BACKING,
            projection: Box::new([ids::currency_backing::CURRENCY]),
        },
        // backings reference real currencies (a plain closed target),
        containment(
            side(ids::CURRENCY_BACKING, ids::currency_backing::CURRENCY, &[]),
            side(ids::CURRENCY, ids::currency::ID, &[]),
        ),
        // domain quantification: every currency has a backing — judged
        // at backing-delete time via the extension scan,
        containment(
            side(ids::CURRENCY, ids::currency::ID, &[]),
            side(ids::CURRENCY_BACKING, ids::currency_backing::CURRENCY, &[]),
        ),
        // and the ψ-sub-vocabulary: only zero-decimal currencies round.
        containment(
            side(ids::CASH_ROUNDING, ids::cash_rounding::CURRENCY, &[]),
            side(
                ids::CURRENCY,
                ids::currency::ID,
                &[(ids::currency::MINOR_UNITS, Value::U64(0))],
            ),
        ),
    ]
}

/// The closed-relation statement ids, pinned (materialized order: seven
/// fresh auto-keys, three closed auto-keys, then the declared list —
/// asserted by `the_closed_statement_pins_hold`).
pub const VOCAB_CURRENCY: bumbledb::StatementId = bumbledb::StatementId(23);
/// `JournalEntry(source) <= Source(id)`.
pub const VOCAB_SOURCE: bumbledb::StatementId = bumbledb::StatementId(24);
/// `CurrencyBacking(currency) -> CurrencyBacking` — the declared key.
pub const BACKING_KEY: bumbledb::StatementId = bumbledb::StatementId(26);
/// `CurrencyBacking(currency) <= Currency(id)`.
pub const BACKING_VALID: bumbledb::StatementId = bumbledb::StatementId(27);
/// `Currency(id) <= CurrencyBacking(currency)` — domain quantification.
pub const CURRENCY_BACKED: bumbledb::StatementId = bumbledb::StatementId(28);
/// `CashRounding(currency) <= Currency(id | minor_units == 0)` — the
/// ψ-sub-vocabulary.
pub const CASH_ROUNDING_SUBSET: bumbledb::StatementId = bumbledb::StatementId(29);

/// The one ψ-member of the cash-rounding sub-vocabulary (`Gbp`, the
/// zero-decimal row).
pub const ZERO_DECIMAL_CURRENCY: u64 = 2;

/// The pad-boundary digest widths the tag fields cover, one field per
/// width: astride the first word boundary (7/8/9), one exact word pair
/// (16), and astride the ceiling (63/64).
pub const DIGEST_WIDTHS: [u16; 6] = [7, 8, 9, 16, 63, 64];

/// The digest-tag vocabulary size per width: small, so group-by and
/// `CountDistinct` over bytes<N> see real multiplicity.
pub const DIGEST_VOCAB: u64 = 61;

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
    /// Mirrors the corpus scale ladder's size table
    /// ([`crate::corpus_gen::Sizes::of`]), `Tiny` included.
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        let (postings, instruments, orgs): (u64, u64, u64) = match scale {
            Scale::Tiny => (1_024, 32, 8),
            Scale::S => (100_000, 512, 64),
            Scale::M => (1_000_000, 512, 64),
            Scale::L => (10_000_000, 512, 64),
        };
        let accounts = postings / 200;
        Self {
            postings,
            entries: postings / 2,
            accounts,
            holders: (accounts / 4).max(1),
            instruments,
            orgs,
            mandates: accounts * interval_data::PER_GROUP,
            transfers: postings / 2,
            posting_tags: postings,
        }
    }
}

/// The memo vocabulary size (interning realism).
pub const MEMO_VOCAB: u64 = 4096;

/// `JournalEntry.source`'s `Import` row id — the DU pair's
/// discriminator. Sources are deterministic (`row % 3`), so entry `i`
/// is an import iff `i % 3 == 1`, and `ImportBatch` row `k` names entry
/// `3k + 1`: both `==` directions hold by construction.
pub const SOURCE_IMPORT: u64 = 1;

/// The entry an `ImportBatch` row names (see [`SOURCE_IMPORT`]).
#[must_use]
pub fn import_batch_entry(row: u64) -> u64 {
    3 * row + 1
}

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

/// The seeded extref of one Transfer row — 32 bytes, a pure function of
/// the row, so in-vocabulary bytes<32> literals recompute exactly.
/// Deliberately adversarial for hashing and comparison: every extref
/// shares a 24-byte zero prefix (word 0..2 collide across the whole
/// corpus), consecutive rows are single-byte deltas at the tail, and
/// row 0 is the all-zeros digest — while staying unique per row, so the
/// appended `Transfer(extref) -> Transfer` key holds by construction.
#[must_use]
pub fn extref(cfg: GenConfig, row: u64) -> Value {
    let _ = cfg; // the shape is row-determined; the seed names nothing here
    let mut raw = [0u8; 32];
    raw[24..].copy_from_slice(&row.to_be_bytes());
    Value::FixedBytes(raw.as_slice().into())
}

/// Vocabulary value `k` of the width-`width` digest tags: all-zero but
/// for the last byte — every value shares the maximal prefix (whole
/// zero words for width > 8), neighbors are single-byte deltas, and
/// `k = 0` is the all-zeros digest.
/// # Panics
///
/// Never in practice (`width >= 1` by the digest roster).
#[must_use]
pub fn digest_vocab_value(width: u16, k: u64) -> Value {
    let mut raw = vec![0u8; usize::from(width)];
    *raw.last_mut().expect("width >= 1") = u8::try_from(k % 256).expect("byte");
    Value::FixedBytes(raw.into())
}

/// One Transfer row's digest tag at `width` — drawn from the
/// [`DIGEST_VOCAB`]-value vocabulary (a pure function of the config, so
/// hits recompute exactly).
#[must_use]
pub fn transfer_tag(cfg: GenConfig, row: u64, width: u16) -> Value {
    let mut rng = row_rng(
        cfg.seed ^ u64::from(width).wrapping_mul(0xD6E8_FEB8_6659_FD93),
        u64::from(ids::TRANSFER.0),
        row,
    );
    digest_vocab_value(width, rng.range(DIGEST_VOCAB))
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
#[must_use]
pub fn posting_tag(row: u64) -> (u64, u64) {
    let posting = (row / 2) * 2;
    (posting, row % 3)
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

/// The number of **writable** target relations — loaders iterate
/// `0..TARGET_RELATIONS`. The closed relations (`Currency`/`Source`/
/// `Tag`, ids 13..16) sit after every ordinary relation by declaration:
/// they are unwritable ground axioms, so no loader touches them (their
/// mirror rows are extension INSERTs — schema surface, not corpus).
pub const TARGET_RELATIONS: u32 = 13;

/// Row count of one target relation (the randomized lane's corpus).
#[must_use]
pub fn corpus_rows(domains: &Domains, rel: bumbledb::RelationId) -> u64 {
    match rel {
        ids::HOLDER => domains.holders,
        ids::ACCOUNT => domains.accounts,
        ids::INSTRUMENT => domains.instruments,
        ids::JOURNAL_ENTRY => domains.entries,
        ids::POSTING => domains.postings,
        ids::POSTING_TAG => domains.posting_tags,
        ids::ORG => domains.orgs,
        ids::ORG_PARENT => domains.orgs - 1,
        ids::MANDATE => domains.mandates,
        ids::TRANSFER => domains.transfers,
        // Import entries are `i % 3 == 1` in `0..entries`: count them.
        ids::IMPORT_BATCH => (domains.entries + 1) / 3,
        // One backing per currency: the domain quantification holds by
        // construction from the first load onward.
        ids::CURRENCY_BACKING => 3,
        // The one ψ-member row (`Gbp`, minor_units == 0).
        ids::CASH_ROUNDING => 1,
        _ => unreachable!("thirteen target relations"),
    }
}

/// One target-relation row, by index — the pure function the randomized
/// lane's corpus streams from, built entirely from this module's value
/// functions so the dressing's in-vocabulary hits are *actual* rows.
///
/// # Panics
///
/// Only on programmer-invariant violations (an unknown relation id).
#[must_use]
pub fn corpus_row(
    cfg: GenConfig,
    domains: &Domains,
    rel: bumbledb::RelationId,
    i: u64,
) -> Vec<Value> {
    let mut rng = row_rng(cfg.seed, u64::from(rel.0), i);
    match rel {
        ids::HOLDER => vec![
            Value::U64(i),
            Value::String(
                string_hit(ids::HOLDER, ids::holder::NAME, &mut rng)
                    .into_bytes()
                    .into(),
            ),
        ],
        ids::ACCOUNT => vec![
            Value::U64(i),
            Value::U64(rng.range(domains.holders)),
            Value::U64(rng.range(3)),
        ],
        ids::INSTRUMENT => vec![
            Value::U64(i),
            Value::String(format!("SYM{i:04}").into_bytes().into()),
        ],
        ids::JOURNAL_ENTRY => vec![
            Value::U64(i),
            // Deterministic (never drawn): the DU pair requires import
            // entries to be exactly the ImportBatch rows' entries.
            Value::U64(i % 3),
            Value::I64(posting_at(i * 2)),
        ],
        ids::POSTING => vec![
            Value::U64(i),
            Value::U64(rng.range(domains.entries)),
            // Round-robin accounts: every in-domain account id exists.
            Value::U64(i % domains.accounts.max(1)),
            Value::U64(rng.range(domains.instruments)),
            Value::I64(posting_amount(cfg, i)),
            Value::I64(posting_at(i)),
            Value::String(format!("m{}", rng.range(MEMO_VOCAB)).into_bytes().into()),
            Value::Bool(rng.chance(1, 2)),
        ],
        ids::POSTING_TAG => {
            let (posting, tag) = posting_tag(i);
            vec![Value::U64(posting), Value::U64(tag)]
        }
        ids::ORG => vec![
            Value::U64(i),
            Value::String(format!("org-{i:02}").into_bytes().into()),
        ],
        ids::ORG_PARENT => {
            let child = i + 1;
            vec![Value::U64(child), Value::U64(child / 2)]
        }
        ids::MANDATE => {
            let (account, org, (start, end)) = mandate(cfg, domains, i);
            vec![
                Value::U64(account),
                Value::U64(org),
                Value::IntervalI64(start, end),
            ]
        }
        ids::TRANSFER => {
            let (start, end) = transfer_window(cfg, i);
            let mut row = vec![
                Value::U64(i),
                extref(cfg, i),
                Value::IntervalU64(start, end),
            ];
            row.extend(DIGEST_WIDTHS.map(|width| transfer_tag(cfg, i, width)));
            row
        }
        ids::IMPORT_BATCH => vec![Value::U64(import_batch_entry(i)), Value::U64(i)],
        ids::CURRENCY_BACKING => vec![Value::U64(i), Value::U64(1_000 + i)],
        ids::CASH_ROUNDING => vec![Value::U64(ZERO_DECIMAL_CURRENCY)],
        _ => unreachable!("thirteen target relations"),
    }
}

/// One target relation's full row stream — O(1) memory, restartable.
pub fn corpus_relation_rows(
    cfg: GenConfig,
    rel: bumbledb::RelationId,
) -> impl Iterator<Item = Vec<Value>> + Clone {
    let domains = Domains::of(cfg.scale);
    (0..corpus_rows(&domains, rel)).map(move |i| corpus_row(cfg, &domains, rel, i))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::schema::StatementView;

    const CFG: GenConfig = GenConfig {
        seed: 7,
        scale: Scale::S,
    };

    /// The closed-relation statement pins: materialized order is seven
    /// fresh auto-keys, the three closed auto-keys, then the declared
    /// list — re-derived here so the differential's typed verdicts name
    /// real statements, never guessed ids.
    #[test]
    fn the_closed_statement_pins_hold() {
        let schema = schema();
        let containment = |id: bumbledb::StatementId| match schema.statement(id) {
            StatementView::Containment(_, statement) => {
                (statement.source.relation, statement.target.relation)
            }
            StatementView::Key(_, statement) => {
                panic!("statement {} is a key: {statement:?}", id.0)
            }
        };
        assert_eq!(containment(VOCAB_CURRENCY), (ids::ACCOUNT, ids::CURRENCY));
        assert_eq!(containment(VOCAB_SOURCE), (ids::JOURNAL_ENTRY, ids::SOURCE));
        assert!(matches!(
            schema.statement(BACKING_KEY),
            StatementView::Key(_, statement) if statement.relation == ids::CURRENCY_BACKING
        ));
        assert_eq!(
            containment(BACKING_VALID),
            (ids::CURRENCY_BACKING, ids::CURRENCY)
        );
        assert_eq!(
            containment(CURRENCY_BACKED),
            (ids::CURRENCY, ids::CURRENCY_BACKING)
        );
        assert_eq!(
            containment(CASH_ROUNDING_SUBSET),
            (ids::CASH_ROUNDING, ids::CURRENCY)
        );
        // The ψ-member is the zero-decimal row, by the extension itself.
        let descriptor = descriptor();
        let extension = descriptor.relations[ids::CURRENCY.0 as usize]
            .extension
            .as_ref()
            .expect("Currency is closed");
        for (row, axiom) in extension.iter().enumerate() {
            assert_eq!(
                axiom.values[0] == Value::U64(0),
                row as u64 == ZERO_DECIMAL_CURRENCY,
                "the ψ-subset has exactly one member"
            );
        }
    }

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

    /// The DU pair holds by construction: entry `i` is an import iff
    /// `i % 3 == 1` iff `ImportBatch` row `(i - 1) / 3` names it —
    /// the alignment the joint corpus commit relies on.
    #[test]
    fn import_batches_mirror_import_entries() {
        let domains = Domains::of(Scale::S);
        let import_entries: Vec<u64> = (0..domains.entries).filter(|i| i % 3 == 1).collect();
        assert_eq!(
            corpus_rows(&domains, ids::IMPORT_BATCH),
            import_entries.len() as u64
        );
        for (k, entry) in import_entries.iter().enumerate() {
            assert_eq!(import_batch_entry(k as u64), *entry);
        }
        let entry = corpus_row(CFG, &domains, ids::JOURNAL_ENTRY, 4);
        assert_eq!(entry[1], Value::U64(SOURCE_IMPORT));
        assert_ne!(
            corpus_row(CFG, &domains, ids::JOURNAL_ENTRY, 3)[1],
            entry[1]
        );
    }

    /// Extrefs recompute exactly (the dressing's in-vocabulary hits) and
    /// carry the adversarial shape: shared 24-byte zero prefix, all-zeros
    /// at row 0, single-byte deltas between neighbors — unique per row.
    #[test]
    fn extref_recomputes_and_is_adversarial() {
        assert_eq!(extref(CFG, 9), extref(CFG, 9));
        assert_ne!(extref(CFG, 9), extref(CFG, 10));
        let Value::FixedBytes(zero) = extref(CFG, 0) else {
            panic!("extref is bytes<32>")
        };
        assert_eq!(&zero[..], &[0u8; 32]);
        let (Value::FixedBytes(a), Value::FixedBytes(b)) = (extref(CFG, 5), extref(CFG, 6)) else {
            panic!("extref is bytes<32>")
        };
        assert_eq!(a[..24], b[..24], "shared prefix");
        assert_eq!(
            a.iter().zip(b.iter()).filter(|(x, y)| x != y).count(),
            1,
            "neighbors differ in one byte"
        );
    }

    /// Digest-tag vocabularies repeat (group-by sees multiplicity), stay
    /// in-width, and include the all-zeros value.
    #[test]
    fn digest_tags_draw_from_small_adversarial_vocabularies() {
        for width in DIGEST_WIDTHS {
            let Value::FixedBytes(raw) = transfer_tag(CFG, 3, width) else {
                panic!("tags are bytes<N>")
            };
            assert_eq!(raw.len(), usize::from(width));
            assert_eq!(transfer_tag(CFG, 3, width), transfer_tag(CFG, 3, width));
            let Value::FixedBytes(zero) = digest_vocab_value(width, 0) else {
                panic!("vocab values are bytes<N>")
            };
            assert!(zero.iter().all(|&b| b == 0), "k = 0 is all-zeros");
        }
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
