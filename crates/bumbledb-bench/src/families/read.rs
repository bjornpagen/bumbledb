use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, MaskTerm, ParamId, Query,
    Rule, Term, Value, VarId,
};

use crate::corpus_gen::{self, GenConfig, Rng, Sizes};
use crate::families::{Draw, Family, Kind, scalar_draw};
use crate::fixture::var;
use crate::naive::ParamValue;
use crate::schema::ids;
use crate::translate::goldens;

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// point — `Q(amount, at) :- Posting(id = ?0, amount, at)`. Key probe.
fn point_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, param(0)),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn point_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0001);
    let mut sets: Vec<Draw> = (0..3)
        .map(|_| scalar_draw(vec![Value::U64(rng.range(sizes.postings))]))
        .collect();
    sets.push(scalar_draw(vec![Value::U64(sizes.postings + 1_000_000)]));
    sets
}

/// `containment_walk` — `Q(name, amount) :- Posting(account = ?0, amount),
/// Account(id = ?0, holder = h), Holder(id = h, name)`.
fn containment_walk_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ACCOUNT, param(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![(ids::account::ID, param(0)), (ids::account::HOLDER, var(2))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::HOLDER),
                bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn cold_account(rng: &mut Rng, sizes: &Sizes) -> u64 {
    let hot = sizes.hot_accounts();
    hot + rng.range(sizes.accounts - hot)
}

fn containment_walk_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0002);
    vec![
        scalar_draw(vec![Value::U64(cold_account(&mut rng, &sizes))]),
        scalar_draw(vec![Value::U64(cold_account(&mut rng, &sizes))]),
        scalar_draw(vec![Value::U64(rng.range(sizes.hot_accounts()))]),
        scalar_draw(vec![Value::U64(sizes.accounts + 1_000_000)]),
    ]
}

/// chain — `Q(src, amount, at) :- Posting(entry = e, account = a,
/// amount, at), JournalEntry(id = e, source = src),
/// Account(id = a, currency = Usd)` with `at >= ?0` — the multi-hop
/// walk across postings/entries/accounts, an enum literal pinning the
/// account side (~1/3 of accounts).
fn chain_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ENTRY, var(3)),
                    (ids::posting::ACCOUNT, var(4)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::AT, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::JOURNAL_ENTRY),
                bindings: vec![
                    (ids::journal_entry::ID, var(3)),
                    (ids::journal_entry::SOURCE, var(0)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![
                    (ids::account::ID, var(4)),
                    (ids::account::CURRENCY, Term::Literal(Value::U64(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: param(0),
        })],
    })
}

fn chain_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let span = i64::try_from(sizes.postings).expect("fits") * corpus_gen::AT_STEP;
    // Four suffix edges near the corpus end, selecting ≈2/4/6/8%.
    (1..=4)
        .map(|k| scalar_draw(vec![Value::I64(corpus_gen::AT_BASE + span - span * k / 50)]))
        .collect()
}

/// range — `Q(id, amount) :- Posting(id, amount, at)`, `at >= ?0`,
/// `at < ?1` — the pure scan family.
fn range_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::AMOUNT, var(1)),
                (ids::posting::AT, var(2)),
            ],
        }],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(2),
                rhs: param(0),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(2),
                rhs: param(1),
            }),
        ],
    })
}

fn range_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let span = i64::try_from(sizes.postings).expect("fits") * corpus_gen::AT_STEP;
    let width = span / 50;
    // Four ≈2%-selectivity windows spread over the timestamp span.
    (0..4)
        .map(|k| {
            let start = corpus_gen::AT_BASE + span * (2 * k + 1) / 16;
            scalar_draw(vec![Value::I64(start), Value::I64(start + width)])
        })
        .collect()
}

/// balance — `Q(a, Sum(amount)) :- Posting(id, account = a, amount),
/// Account(id = a, holder = ?0)`. The fresh id binding makes every
/// posting a distinct binding, so the fold is the *ledger balance* —
/// duplicate amounts on one account count once each, not once total —
/// and the distinct-bindings elision engages (key coverage), putting
/// the seen-set-elided aggregate path under the oracle.
pub(super) fn balance_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ID, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![(ids::account::ID, var(0)), (ids::account::HOLDER, param(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn balance_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    // The hot-owning holder: whoever holds hot account 0 (deterministic —
    // the generator is a pure function of (cfg, relation, row)).
    let account0 = corpus_gen::row(cfg, &sizes, ids::ACCOUNT, 0);
    let hot_holder = account0[usize::from(ids::account::HOLDER.0)].clone();
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0005);
    let mut sets = vec![scalar_draw(vec![hot_holder])];
    sets.extend((0..3).map(|_| scalar_draw(vec![Value::U64(rng.range(sizes.holders))])));
    sets
}

/// stats — `Q(c, Min(at), Max(amount), Count) :- Posting(account = a,
/// amount, at), Account(id = a, currency = c)` — the literal-free full
/// fold grouped by currency.
fn stats_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(2)),
            },
            FindTerm::Aggregate {
                op: AggOp::Max,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ACCOUNT, var(3)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::AT, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![(ids::account::ID, var(3)), (ids::account::CURRENCY, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn stats_params(_: &GenConfig) -> Vec<Draw> {
    // Literal-free full fold: one empty draw.
    vec![scalar_draw(vec![])]
}

/// string — `Q(id, amount) :- Posting(id, amount, instrument = i),
/// Instrument(id = i, symbol = ?0)` — the interned-string point lookup.
fn string_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ID, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::INSTRUMENT, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::INSTRUMENT),
                bindings: vec![
                    (ids::instrument::ID, var(2)),
                    (ids::instrument::SYMBOL, param(0)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn string_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0007);
    let mut sets: Vec<Draw> = (0..3)
        .map(|_| {
            scalar_draw(vec![Value::String(
                format!("SYM{:04}", rng.range(sizes.instruments))
                    .into_bytes()
                    .into(),
            )])
        })
        .collect();
    // The never-interned miss: no corpus vocabulary starts with this.
    sets.push(scalar_draw(vec![Value::String(
        b"missing-family".to_vec().into(),
    )]));
    sets
}

/// skew — `Q(p, amount) :- Posting(id = p, amount),
/// PostingTag(posting = p, tag = ?0)` — the skewed tag join: the
/// generator routes [`corpus_gen::HOT_TAG_PCT`]% of first tags to `Fee`
/// (ordinal 0), so the rotation spans hot and uniform fan-outs.
fn skew_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![(ids::posting::ID, var(0)), (ids::posting::AMOUNT, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
                bindings: vec![
                    (ids::posting_tag::POSTING, var(0)),
                    (ids::posting_tag::TAG, param(0)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn skew_params(_: &GenConfig) -> Vec<Draw> {
    // The hot tag, then the two uniform tags (all three ordinals — an
    // out-of-range ordinal is unrepresentable, so no miss draw exists).
    vec![
        scalar_draw(vec![Value::U64(0)]),
        scalar_draw(vec![Value::U64(1)]),
        scalar_draw(vec![Value::U64(2)]),
    ]
}

/// spread — `Q(x, y) :- Posting(entry = e, amount = x),
/// Posting(entry = e, amount = y)` with `x < y` — the cross-atom
/// residual family and the duplicate-witness projection: a self-join
/// whose ordered comparison exercises residual placement, and whose
/// `(x, y)` pairs are witnessed by many distinct entries.
fn spread_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ENTRY, var(2)),
                    (ids::posting::AMOUNT, var(0)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ENTRY, var(2)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        })],
    })
}

fn spread_params(_: &GenConfig) -> Vec<Draw> {
    // Param-less full-relation family (like stats): ~1 ordered pair per
    // entry at any scale (2 postings per entry), so the result is
    // ~entries rows — measured acceptable at S.
    vec![scalar_draw(vec![])]
}

/// triangle — `Q(a) :- Posting(account = a, instrument = i),
/// Posting(instrument = i, entry = w), Posting(entry = w, account = a)`
/// with `?0 <= a < ?1` — a true 3-cycle over the ledger via self-joins
/// on Posting's three containment fields: three occurrences, three shared
/// variables, a cyclic hypergraph — exactly the dynamic-cover stress
/// the paper's triangle exposes.
fn triangle_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::INSTRUMENT, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ENTRY, var(2)),
                    (ids::posting::INSTRUMENT, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ENTRY, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(0),
                rhs: param(0),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: param(1),
            }),
        ],
    })
}

fn triangle_params(cfg: &GenConfig) -> Vec<Draw> {
    // The unnarrowed cycle is O(postings x instrument-fanout) work —
    // beyond the 10 ms budget class at S (measured on the previous
    // ledger) — so the account window `?0 <= a < ?1` keeps the family
    // inside it. Windows are *cold* (they start past the hot set — any
    // window containing hot account 0 is dominated by its posting
    // share): three ~1%-of-accounts slices spread over the cold range,
    // plus the empty window.
    let sizes = Sizes::of(cfg.scale);
    let hot = sizes.hot_accounts();
    let width = (sizes.accounts / 100).max(1);
    let cold = sizes.accounts - hot;
    let window = |k: u64| {
        let lo = hot + cold * k / 3;
        scalar_draw(vec![Value::U64(lo), Value::U64(lo + width)])
    };
    let mut sets: Vec<Draw> = (0..3).map(window).collect();
    sets.push(scalar_draw(vec![
        Value::U64(sizes.accounts),
        Value::U64(sizes.accounts),
    ]));
    sets
}

/// `entries_for_account_set` — `Q(e) :- Posting(entry = e,
/// account ∈ ?set0)` — the param-set family (`ParamSet` replaces the
/// retired host-side union convention): entries touching any account of
/// a bound set, `IN`-list on the `SQLite` side, re-rendered per draw.
fn entries_for_account_set_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ENTRY, var(0)),
                (ids::posting::ACCOUNT, Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn entries_for_account_set_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_000B);
    let mut cold = |n: usize| -> Vec<Value> {
        (0..n)
            .map(|_| Value::U64(cold_account(&mut rng, &sizes)))
            .collect()
    };
    let singleton = cold(1);
    let mut with_hot = cold(2);
    with_hot.push(Value::U64(0));
    let eight = cold(8);
    vec![
        vec![ParamValue::Set(singleton)],
        vec![ParamValue::Set(with_hot)],
        vec![ParamValue::Set(eight)],
        vec![ParamValue::Set(Vec::new())],
    ]
}

/// `postings_without_tag` — `Q(p, amount) :- Posting(id = p,
/// account = ?0, amount), ¬PostingTag(posting = p)` — the negation
/// family: one account's untagged postings (the generator tags even
/// posting ids only, so roughly half of any account's postings
/// survive the anti-join).
///
/// **The cross-process p50 bimodality is the rotation-boundary
/// tail-max, not an engine mode (mechanism hunt, 2026-07-17).** The
/// two cold-account draws (≈ 2.6 µs each) fill ranks 0–127 of the
/// 256-sample rotation exactly, with the miss at ≈ 11.5 µs and the
/// hot account at ≈ 1.05 ms above them — so the nearest-rank p50
/// (`sorted[127]`) is the MAX of 128 cold samples, an extreme order
/// statistic that swung 2.75–9.54 µs across 30 fresh processes while
/// every draw median held within ±0.5% (colds 2500–2625 ns). Same
/// evidence and refutations as `slot_booking_overlap`
/// (calendar/families.rs): same binary + same store still flips,
/// byte-identical regenerated stores flip identically, a relinked
/// binary moves nothing — the flip is the statistic, not the engine.
fn postings_without_tag_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::ACCOUNT, param(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
            bindings: vec![(ids::posting_tag::POSTING, var(0))],
        }],
        conditions: vec![],
    })
}

fn postings_without_tag_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_000C);
    vec![
        scalar_draw(vec![Value::U64(cold_account(&mut rng, &sizes))]),
        scalar_draw(vec![Value::U64(cold_account(&mut rng, &sizes))]),
        scalar_draw(vec![Value::U64(rng.range(sizes.hot_accounts()))]),
        scalar_draw(vec![Value::U64(sizes.accounts + 1_000_000)]),
    ]
}

/// `latest_posting_per_account` — `Q(a, ArgMax_at(p)) :- Posting(id = p,
/// account = a, at = t)` — the Arg-restriction family: each account's
/// latest posting (the join-back template on the `SQLite` side; `at` is
/// strictly increasing per posting id, so groups are tie-free — tie
/// semantics are the query generator's lane).
fn latest_posting_per_account_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::ArgMax {
                    key: bumbledb::ArgKey::Var(VarId(2)),
                },
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, var(1)),
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AT, var(2)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn latest_posting_per_account_params(_: &GenConfig) -> Vec<Draw> {
    // Param-less full restriction: one empty draw.
    vec![scalar_draw(vec![])]
}

/// `mandate_at_instant` — `Q(o) :- Posting(account = ?0, at = ?1),
/// Mandate(account = ?0, org = o, active ∋ ?1)` — the interval
/// membership probe through a **param point**: which orgs held a
/// mandate on the account at the instant of one of its postings. The
/// posting atom anchors `?1` as a scalar point (the bivalent-anchor
/// rule: a lone interval-position param would read as interval value
/// equality), exactly the doc's at-instant probe form.
fn mandate_at_instant_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ACCOUNT, param(0)),
                    (ids::posting::AT, param(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, param(0)),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, param(1)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn mandate_at_instant_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_000E);
    // Three real postings' (account, at) instants — the mandate windows
    // tile the posting-at span, so most instants land inside a segment —
    // plus the account miss.
    let mut sets: Vec<Draw> = (0..3)
        .map(|_| {
            let posting = corpus_gen::row(cfg, &sizes, ids::POSTING, rng.range(sizes.postings));
            scalar_draw(vec![
                posting[usize::from(ids::posting::ACCOUNT.0)].clone(),
                posting[usize::from(ids::posting::AT.0)].clone(),
            ])
        })
        .collect();
    sets.push(scalar_draw(vec![
        Value::U64(sizes.accounts + 1_000_000),
        Value::I64(corpus_gen::AT_BASE),
    ]));
    sets
}

/// `mandate_overlap` — `Q(a1, a2) :- Mandate(account = a1, org = ?0,
/// active = u), Mandate(account = a2, org = ?0, active = v),
/// Allen(u, v, INTERSECTS)` — the interval-intersection family.
/// **Chosen shape:** Mandate × Mandate joined through a shared org param
/// — a true Allen-mask JOIN across accounts (account pairs concurrently
/// mandated to one org), not a window filter; the pointwise key makes
/// same-account intersection impossible, so the join must cross accounts
/// to produce anything beyond the reflexive pairs.
fn mandate_overlap_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(0)),
                    (ids::mandate::ORG, param(0)),
                    (ids::mandate::ACTIVE, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(1)),
                    (ids::mandate::ORG, param(0)),
                    (ids::mandate::ACTIVE, var(3)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: var(2),
            rhs: var(3),
        })],
    })
}

fn mandate_overlap_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_000F);
    (0..4)
        .map(|_| scalar_draw(vec![Value::U64(rng.range(sizes.orgs))]))
        .collect()
}

/// The registry: the fifteen, in the suite's canonical order — the
/// ported ten, then the redesign's five (param set, negation,
/// Arg-restriction, membership, overlap).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the registry is a table, one entry per family
pub fn all() -> &'static [Family] {
    &[
        Family {
            name: "point",
            kind: Kind::Gate,
            query: point_query,
            params: point_params,
            golden_sql: goldens::POINT,
            param_policy: "3 existing posting ids + 1 miss (id = postings + 10^6).",
            indexes: &[],
        },
        Family {
            name: "containment_walk",
            kind: Kind::Gate,
            query: containment_walk_query,
            params: containment_walk_params,
            golden_sql: goldens::CONTAINMENT_WALK,
            param_policy: "2 cold accounts, 1 hot account, 1 miss (id = accounts + 10^6).",
            indexes: &[],
        },
        Family {
            name: "chain",
            kind: Kind::Gate,
            query: chain_query,
            params: chain_params,
            golden_sql: goldens::CHAIN,
            param_policy: "4 suffix edges near the corpus end (at >= edge selects ~2/4/6/8%).",
            indexes: &[("idx_posting_at", "Posting", &["at"])],
        },
        Family {
            name: "range",
            kind: Kind::Gate,
            query: range_query,
            params: range_params,
            golden_sql: goldens::RANGE,
            param_policy: "4 windows of the pinned ~2% selectivity, spread over the span.",
            indexes: &[("idx_posting_at", "Posting", &["at"])],
        },
        Family {
            name: "balance",
            kind: Kind::Gate,
            query: balance_query,
            params: balance_params,
            golden_sql: goldens::BALANCE,
            param_policy: "4 holders, the first owning hot account 0.",
            indexes: &[],
        },
        Family {
            name: "stats",
            kind: Kind::Gate,
            query: stats_query,
            params: stats_params,
            golden_sql: goldens::STATS,
            param_policy: "No params — literal-free full fold; one empty draw.",
            indexes: &[],
        },
        Family {
            name: "string",
            kind: Kind::Gate,
            query: string_query,
            params: string_params,
            golden_sql: goldens::STRING,
            param_policy: "3 existing symbols + 1 never-interned miss.",
            indexes: &[("idx_instrument_symbol", "Instrument", &["symbol"])],
        },
        Family {
            name: "skew",
            kind: Kind::Gate,
            query: skew_query,
            params: skew_params,
            golden_sql: goldens::SKEW,
            param_policy: "The hot tag (Fee, ~60% of first tags), then the two uniform tags.",
            indexes: &[(
                "idx_postingtag_tag_posting",
                "PostingTag",
                &["tag", "posting"],
            )],
        },
        Family {
            name: "spread",
            kind: Kind::Gate,
            query: spread_query,
            params: spread_params,
            golden_sql: goldens::SPREAD,
            param_policy: "No params — full-relation cross-atom residual; one empty draw.",
            indexes: &[],
        },
        Family {
            name: "triangle",
            kind: Kind::Gate,
            query: triangle_query,
            params: triangle_params,
            golden_sql: goldens::TRIANGLE,
            param_policy: "3 cold ~1%-of-accounts windows (?0 <= a < ?1, past the hot set) + the empty window.",
            indexes: &[],
        },
        Family {
            name: "entries_for_account_set",
            kind: Kind::Gate,
            query: entries_for_account_set_query,
            params: entries_for_account_set_params,
            golden_sql: goldens::IN_THREE,
            param_policy: "Account sets of sizes 1, 3 (hot account 0 included), 8, and 0 — the golden pins the representative set {3, 7, 9}.",
            indexes: &[(
                "idx_posting_account_entry",
                "Posting",
                &["account", "entry"],
            )],
        },
        Family {
            name: "postings_without_tag",
            kind: Kind::Gate,
            query: postings_without_tag_query,
            params: postings_without_tag_params,
            golden_sql: goldens::POSTINGS_WITHOUT_TAG,
            param_policy: "2 cold accounts, 1 hot account, 1 miss (id = accounts + 10^6).",
            indexes: &[],
        },
        Family {
            name: "latest_posting_per_account",
            kind: Kind::Gate,
            query: latest_posting_per_account_query,
            params: latest_posting_per_account_params,
            golden_sql: goldens::ARG_MAX,
            param_policy: "No params — full Arg-restriction over every account; one empty draw.",
            indexes: &[("idx_posting_account_at", "Posting", &["account", "at"])],
        },
        Family {
            name: "mandate_at_instant",
            kind: Kind::Gate,
            query: mandate_at_instant_query,
            params: mandate_at_instant_params,
            golden_sql: goldens::MEMBERSHIP_PARAM,
            param_policy: "3 real postings' (account, at) instants + 1 account miss — gap instants occur naturally (segments 1-2 and 2-3 are gapped).",
            indexes: &[],
        },
        Family {
            name: "mandate_overlap",
            kind: Kind::Gate,
            query: mandate_overlap_query,
            params: mandate_overlap_params,
            golden_sql: goldens::MANDATE_OVERLAP,
            param_policy: "4 org ids (mandates spread uniformly over 64 orgs).",
            indexes: &[(
                "idx_mandate_org_active",
                "Mandate",
                &["org", "active_start", "active_end"],
            )],
        },
    ]
}
