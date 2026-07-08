use bumbledb::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};

use crate::families::{Family, Kind, SKEW_HOT_TAGS};
use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::schema::ids;
use crate::translate::goldens;

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// point — `Q(amount, at) :- Posting(id = ?0, amount, at)`. Guard probe.
fn point_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, param(0)),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        predicates: vec![],
    }
}

fn point_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0001);
    let mut sets: Vec<Vec<Value>> = (0..3)
        .map(|_| vec![Value::U64(rng.range(sizes.postings))])
        .collect();
    sets.push(vec![Value::U64(sizes.postings + 1_000_000)]);
    sets
}

/// `fk_walk` — `Q(name, amount) :- Posting(account = ?0, amount),
/// Account(id = ?0, holder = h), Holder(id = h, name)`.
fn fk_walk_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, param(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![(ids::account::ID, param(0)), (ids::account::HOLDER, var(2))],
            },
            Atom {
                relation: ids::HOLDER,
                bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
            },
        ],
        predicates: vec![],
    }
}

fn fk_walk_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let hot = sizes.hot_accounts();
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0002);
    vec![
        vec![Value::U64(hot + rng.range(sizes.accounts - hot))],
        vec![Value::U64(hot + rng.range(sizes.accounts - hot))],
        vec![Value::U64(rng.range(hot))],
        vec![Value::U64(sizes.accounts + 1_000_000)],
    ]
}

/// chain — `Q(region, amount, at) :- Posting(account = a, amount, at),
/// Account(id = a, holder = h, status = Open), Holder(id = h, region)`
/// with `at >= ?0`.
fn chain_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(3)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::AT, var(2)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, var(3)),
                    (ids::account::HOLDER, var(4)),
                    (ids::account::STATUS, Term::Literal(Value::Enum(0))),
                ],
            },
            Atom {
                relation: ids::HOLDER,
                bindings: vec![(ids::holder::ID, var(4)), (ids::holder::REGION, var(0))],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: param(0),
        }],
    }
}

fn chain_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    // Four suffix edges near the corpus end, selecting ≈2/4/6/8%.
    (1..=4)
        .map(|k| vec![Value::I64(gen::AT_BASE + span - span * k / 50)])
        .collect()
}

/// range — `Q(id, amount) :- Posting(id, amount, at)`, `at >= ?0`,
/// `at < ?1` — the pure scan family.
fn range_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::AMOUNT, var(1)),
                (ids::posting::AT, var(2)),
            ],
        }],
        predicates: vec![
            Comparison {
                op: CmpOp::Ge,
                lhs: var(2),
                rhs: param(0),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(2),
                rhs: param(1),
            },
        ],
    }
}

fn range_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    let width = span / 50;
    // Four ≈2%-selectivity windows spread over the timestamp span.
    (0..4)
        .map(|k| {
            let start = gen::AT_BASE + span * (2 * k + 1) / 16;
            vec![Value::I64(start), Value::I64(start + width)]
        })
        .collect()
}

/// balance — `Q(a, Sum(amount)) :- Posting(id, account = a, amount),
/// Account(id = a, holder = ?0)`. The serial id binding makes every
/// posting a distinct binding, so the fold is the *ledger balance* —
/// duplicate amounts on one account count once each, not once total —
/// and the distinct-bindings elision engages (unique coverage), putting
/// the seen-set-elided aggregate path under the oracle.
pub(super) fn balance_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ID, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![(ids::account::ID, var(0)), (ids::account::HOLDER, param(0))],
            },
        ],
        predicates: vec![],
    }
}

fn balance_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    // The hot-owning holder: whoever holds hot account 0 (deterministic —
    // the generator is a pure function of (cfg, relation, row)).
    let account0 = gen::row(cfg, &sizes, ids::ACCOUNT, 0);
    let hot_holder = account0[usize::from(ids::account::HOLDER.0)].clone();
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0005);
    let mut sets = vec![vec![hot_holder]];
    sets.extend((0..3).map(|_| vec![Value::U64(rng.range(sizes.holders))]));
    sets
}

/// stats — `Q(k, Min(at), Max(amount), Count) :- Posting(instrument = i,
/// amount, at), Instrument(id = i, kind = k)`.
fn stats_query() -> Query {
    Query {
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
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::INSTRUMENT, var(3)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::AT, var(2)),
                ],
            },
            Atom {
                relation: ids::INSTRUMENT,
                bindings: vec![
                    (ids::instrument::ID, var(3)),
                    (ids::instrument::KIND, var(0)),
                ],
            },
        ],
        predicates: vec![],
    }
}

fn stats_params(_: &GenConfig) -> Vec<Vec<Value>> {
    // Literal-free full fold: one empty param set.
    vec![vec![]]
}

/// string — `Q(id, amount) :- Posting(id, amount, memo = ?0)`.
fn string_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::AMOUNT, var(1)),
                (ids::posting::MEMO, param(0)),
            ],
        }],
        predicates: vec![],
    }
}

fn string_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0007);
    let mut sets: Vec<Vec<Value>> = (0..3)
        .map(|_| {
            vec![Value::String(
                format!("m{}", rng.range(gen::MEMO_VOCAB))
                    .into_bytes()
                    .into(),
            )]
        })
        .collect();
    // The never-interned miss: no corpus vocabulary starts with this.
    sets.push(vec![Value::String(b"missing-family".to_vec().into())]);
    sets
}

/// skew — `Q(label, amount) :- Posting(account = a, amount),
/// AccountTag(account = a, tag = t), Tag(id = t, label)` with
/// `label = ?0`.
fn skew_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(2)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT_TAG,
                bindings: vec![
                    (ids::account_tag::ACCOUNT, var(2)),
                    (ids::account_tag::TAG, var(3)),
                ],
            },
            Atom {
                relation: ids::TAG,
                bindings: vec![(ids::tag::ID, var(3)), (ids::tag::LABEL, var(0))],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: param(0),
        }],
    }
}

fn skew_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let label = |tag: u64| vec![Value::String(format!("tag-{tag:03}").into_bytes().into())];
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0008);
    // Two hot-attached tags, two uniform tags (drawn above the hot k = 1
    // band, which tops out well below 150).
    vec![
        label(SKEW_HOT_TAGS[0]),
        label(SKEW_HOT_TAGS[1]),
        label(150 + rng.range(100)),
        label(150 + rng.range(100)),
    ]
}

/// spread — `Q(x, y) :- Posting(transfer = t, amount = x),
/// Posting(transfer = t, amount = y)` with `x < y` — the cross-atom
/// residual family: a self-join whose ordered comparison exercises
/// `PlacedComparison` placement, per-node residual evaluation, and
/// survivor compaction (no other family or generated shape did).
fn spread_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::AMOUNT, var(0)),
                ],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        }],
    }
}

fn spread_params(_: &GenConfig) -> Vec<Vec<Value>> {
    // Param-less full-relation family (like stats): ~1 ordered pair per
    // transfer at any scale (2 postings per transfer), so the result is
    // ~transfers rows — measured acceptable at S.
    vec![vec![]]
}

/// triangle — `Q(a) :- Posting(account = a, instrument = i),
/// Posting(instrument = i, transfer = w), Posting(transfer = w,
/// account = a)` with `?0 <= a < ?1` — a true 3-cycle over the ledger via
/// self-joins on Posting's three FK fields: three occurrences, three
/// shared variables, a cyclic hypergraph — exactly the dynamic-cover
/// stress the paper's triangle exposes.
fn triangle_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::INSTRUMENT, var(1)),
                ],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::INSTRUMENT, var(1)),
                ],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                ],
            },
        ],
        predicates: vec![
            Comparison {
                op: CmpOp::Ge,
                lhs: var(0),
                rhs: param(0),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: param(1),
            },
        ],
    }
}

fn triangle_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    // The unnarrowed cycle is O(postings x instrument-fanout) work —
    // beyond the 10 ms budget class at S (measured) — so the account
    // window `?0 <= a < ?1` keeps the family inside it. Windows are
    // *cold* (they start past the hot set — any window containing hot
    // account 0 is dominated by its posting share): three ~1%-of-
    // accounts slices spread over the cold range, plus the empty window.
    let sizes = Sizes::of(cfg.scale);
    let hot = sizes.hot_accounts();
    let width = (sizes.accounts / 100).max(1);
    let cold = sizes.accounts - hot;
    let window = |k: u64| {
        let lo = hot + cold * k / 3;
        vec![Value::U64(lo), Value::U64(lo + width)]
    };
    let mut sets: Vec<Vec<Value>> = (0..3).map(window).collect();
    sets.push(vec![Value::U64(sizes.accounts), Value::U64(sizes.accounts)]);
    sets
}

/// The registry: the ten, in the suite's canonical order.
#[must_use]
pub fn all() -> &'static [Family] {
    &[
        Family {
            name: "point",
            kind: Kind::Gate,
            query: point_query,
            params: point_params,
            golden_sql: goldens::POINT,
            param_policy: "3 existing posting ids + 1 miss (id = postings + 10^6).",
        },
        Family {
            name: "fk_walk",
            kind: Kind::Gate,
            query: fk_walk_query,
            params: fk_walk_params,
            golden_sql: goldens::FK_WALK,
            param_policy: "2 cold accounts, 1 hot account, 1 miss (id = accounts + 10^6).",
        },
        Family {
            name: "chain",
            kind: Kind::Gate,
            query: chain_query,
            params: chain_params,
            golden_sql: goldens::CHAIN,
            param_policy: "4 suffix edges near the corpus end (at >= edge selects ~2/4/6/8%).",
        },
        Family {
            name: "range",
            kind: Kind::Gate,
            query: range_query,
            params: range_params,
            golden_sql: goldens::RANGE,
            param_policy: "4 windows of the pinned ~2% selectivity, spread over the span.",
        },
        Family {
            name: "balance",
            kind: Kind::Gate,
            query: balance_query,
            params: balance_params,
            golden_sql: goldens::BALANCE,
            param_policy: "4 holders, the first owning hot account 0.",
        },
        Family {
            name: "stats",
            kind: Kind::Gate,
            query: stats_query,
            params: stats_params,
            golden_sql: goldens::STATS,
            param_policy: "No params — literal-free full fold; one empty set.",
        },
        Family {
            name: "string",
            kind: Kind::Gate,
            query: string_query,
            params: string_params,
            golden_sql: goldens::STRING,
            param_policy: "3 vocabulary memos + 1 never-interned miss.",
        },
        Family {
            name: "skew",
            kind: Kind::Gate,
            query: skew_query,
            params: skew_params,
            golden_sql: goldens::SKEW,
            param_policy: "2 hot-attached tag labels (tags 0 and 97) + 2 uniform tag labels.",
        },
        Family {
            name: "spread",
            kind: Kind::Gate,
            query: spread_query,
            params: spread_params,
            golden_sql: goldens::SPREAD,
            param_policy: "No params — full-relation cross-atom residual; one empty set.",
        },
        Family {
            name: "triangle",
            kind: Kind::Gate,
            query: triangle_query,
            params: triangle_params,
            golden_sql: goldens::TRIANGLE,
            param_policy: "3 cold ~1%-of-accounts windows (?0 <= a < ?1, past the hot set) + the empty window.",
        },
    ]
}
