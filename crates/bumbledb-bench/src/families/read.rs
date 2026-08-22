use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, ParamId, Query, Rule,
    Term, Value, VarId,
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

    (1..=4)
        .map(|k| scalar_draw(vec![Value::I64(corpus_gen::AT_BASE + span - span * k / 50)]))
        .collect()
}

fn deep_chain_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
            FindTerm::Var(VarId(3)),
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ENTRY, var(4)),
                    (ids::posting::ACCOUNT, var(5)),
                    (ids::posting::AMOUNT, var(2)),
                    (ids::posting::AT, var(3)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::JOURNAL_ENTRY),
                bindings: vec![
                    (ids::journal_entry::ID, var(4)),
                    (ids::journal_entry::SOURCE, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![(ids::account::ID, var(5)), (ids::account::HOLDER, var(6))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::HOLDER),
                bindings: vec![(ids::holder::ID, var(6)), (ids::holder::NAME, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(3),
            rhs: param(0),
        })],
    })
}

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

    (0..4)
        .map(|k| {
            let start = corpus_gen::AT_BASE + span * (2 * k + 1) / 16;
            scalar_draw(vec![Value::I64(start), Value::I64(start + width)])
        })
        .collect()
}

pub(super) fn balance_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
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

    let account0 = corpus_gen::row(cfg, &sizes, ids::ACCOUNT, 0);
    let hot_holder = account0[usize::from(ids::account::HOLDER.0)].clone();
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0005);
    let mut sets = vec![scalar_draw(vec![hot_holder])];
    sets.extend((0..3).map(|_| scalar_draw(vec![Value::U64(rng.range(sizes.holders))])));
    sets
}

fn stats_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Min,
                over: VarId(2),
            },
            FindTerm::Aggregate {
                op: FoldOp::Max,
                over: VarId(1),
            },
            FindTerm::Count,
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
    vec![scalar_draw(vec![])]
}

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
                format!("SYM{:04}", rng.range(sizes.instruments)).into(),
            )])
        })
        .collect();

    sets.push(scalar_draw(vec![Value::String("missing-family".into())]));
    sets
}

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
    vec![
        scalar_draw(vec![Value::U64(0)]),
        scalar_draw(vec![Value::U64(1)]),
        scalar_draw(vec![Value::U64(2)]),
    ]
}

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
    vec![scalar_draw(vec![])]
}

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

fn latest_posting_per_account_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Max,
                over: VarId(2),
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
    vec![scalar_draw(vec![])]
}

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
                mask: AllenMask::INTERSECTS,
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

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
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
            golden_sql: goldens::LATEST_POSTING,
            param_policy: "No params — full Max(at) over every account; one empty draw.",
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
        Family {
            name: "deep_chain",
            kind: Kind::Report,
            query: deep_chain_query,
            params: chain_params,
            golden_sql: goldens::DEEP_CHAIN,
            param_policy: "4 suffix edges near the corpus end (at >= edge selects ~2/4/6/8%) — chain's draws, shared.",
            indexes: &[("idx_posting_at", "Posting", &["at"])],
        },
    ]
}
