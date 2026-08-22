use super::{Db, Run, VerifyConfig};

use bumbledb::{Atom, FieldId, FindTerm, Interval, Query, RelationId, Rule, Term, Value, VarId};

use crate::corpus_gen::{self, MANDATE_SEGMENTS, Sizes, mandate_segments};
use crate::differential::{self, Op};
use crate::families::{self, Draw, scalar_draw};
use crate::naive::{Delta, NaiveDb, ParamValue};
use crate::schema::{Ledger, ids};

fn unit_sizes() -> Sizes {
    Sizes {
        postings: 120,
        entries: 60,
        accounts: 8,
        holders: 2,
        instruments: 8,
        orgs: 4,
        org_parents: 3,
        posting_tags: 120,
        mandates: 8 * MANDATE_SEGMENTS,
    }
}

fn closed_join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![
                    (ids::account::ID, Term::Var(VarId(0))),
                    (ids::account::CURRENCY, Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CURRENCY),
                bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn load_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let cfg = corpus_gen::GenConfig {
        seed,
        scale: corpus_gen::Scale::S, 
    };
    let mut ops = Vec::new();
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        let mut delta = Delta::default();
        for i in 0..sizes.rows(rel) {
            delta
                .inserts
                .push((rel, corpus_gen::row(&cfg, sizes, rel, i)));
            if delta.inserts.len() == 32 {
                ops.push(Op::Write(std::mem::take(&mut delta)));
            }
        }
        if !delta.inserts.is_empty() {
            ops.push(Op::Write(delta));
        }
    }
    ops
}

fn violating_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let cfg = corpus_gen::GenConfig {
        seed,
        scale: corpus_gen::Scale::S,
    };
    let segment = mandate_segments(seed, sizes, 0)[0];
    let overlap = Interval::<i64>::new(segment.start, segment.start + 1).expect("nonempty");
    vec![

        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::POSTING,
                vec![
                    Value::U64(sizes.postings + 1),
                    Value::U64(0),
                    Value::U64(sizes.accounts + 3),
                    Value::U64(0),
                    Value::I64(1),
                    Value::I64(corpus_gen::AT_BASE),
                ],
            )],
        }),

        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::MANDATE,
                vec![
                    Value::U64(0),
                    Value::U64((segment.org + 1) % sizes.orgs),
                    Value::from(overlap),
                ],
            )],
        }),

        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::HOLDER,
                vec![Value::U64(0), Value::String("holder-duplicate".into())],
            )],
        }),

        Op::Write(Delta {
            deletes: vec![(ids::ACCOUNT, corpus_gen::row(&cfg, sizes, ids::ACCOUNT, 0))],
            inserts: vec![],
        }),

        Op::Write({
            let posting = (0..sizes.postings)
                .map(|i| corpus_gen::row(&cfg, sizes, ids::POSTING, i))
                .find(|row| row[usize::from(ids::posting::ACCOUNT.0)] == Value::U64(0))
                .expect("some posting references account 0");
            Delta {
                deletes: vec![
                    (ids::POSTING, posting.clone()),
                    (ids::ACCOUNT, corpus_gen::row(&cfg, sizes, ids::ACCOUNT, 0)),
                ],
                inserts: vec![(ids::POSTING, posting)],
            }
        }),
        // A write naming the closed vocabulary: refused before the
        // delta on the engine, before applying on the model — the same

        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(ids::CURRENCY, vec![Value::U64(5)])],
        }),

        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::ACCOUNT,
                vec![
                    Value::U64(sizes.accounts + 11),
                    Value::U64(0),
                    Value::U64(9),
                ],
            )],
        }),
    ]
}

fn unit_draw(name: &str, seed: u64, sizes: &Sizes) -> Draw {
    let cfg = corpus_gen::GenConfig {
        seed,
        scale: corpus_gen::Scale::S,
    };
    let span = i64::try_from(sizes.postings).expect("fits") * corpus_gen::AT_STEP;
    match name {
        "point" => scalar_draw(vec![Value::U64(3)]),
        "containment_walk" | "postings_without_tag" | "skew" => scalar_draw(vec![Value::U64(0)]),
        "chain" | "deep_chain" => scalar_draw(vec![Value::I64(corpus_gen::AT_BASE)]),
        "range" => scalar_draw(vec![
            Value::I64(corpus_gen::AT_BASE + span / 4),
            Value::I64(corpus_gen::AT_BASE + span / 2),
        ]),

        "balance" | "mandate_overlap" => scalar_draw(vec![Value::U64(1)]),
        "stats" | "spread" | "latest_posting_per_account" => scalar_draw(vec![]),
        "string" => scalar_draw(vec![Value::String("SYM0003".into())]),
        "triangle" => scalar_draw(vec![Value::U64(0), Value::U64(sizes.accounts)]),
        "entries_for_account_set" => vec![ParamValue::Set(vec![
            Value::U64(0),
            Value::U64(3),
            Value::U64(5),
        ])],
        "mandate_at_instant" => {
            let posting = corpus_gen::row(&cfg, sizes, ids::POSTING, 7);
            scalar_draw(vec![
                posting[usize::from(ids::posting::ACCOUNT.0)].clone(),
                posting[usize::from(ids::posting::AT.0)].clone(),
            ])
        }
        other => unreachable!("unregistered family {other}"),
    }
}

/// The naive-model differential slice: a fresh unit-scale store replays the
/// corpus stream, seven judgment-violating deltas (the closed-relation write
/// refusal and the out-of-range vocabulary reference included), the
/// closed-vocabulary join read, every family query (its unit draw plus its
/// seeded S rotation), and the algebra oracle rows (`run_algebra`: rules, DNF
/// trees, `Pack` — naive-only by decision, counted and reported — and the
/// measure's ray verdicts) against [`NaiveDb`]; any verdict, violator, or
/// result-set disagreement is an arbitration bundle. The error-parity cases
/// (cap-exceeding DNF, vacuous masks) run after the differential, against the
/// same store.
/// # Panics
/// On tool-level invariant violations — never on a disagreement.
pub(super) fn run_naive_slice<S>(cfg: &VerifyConfig, run: &mut Run<'_, S>) {
    let sizes = unit_sizes();
    let mut ops = load_ops(cfg.corpus_gen.seed, &sizes);
    ops.extend(violating_ops(cfg.corpus_gen.seed, &sizes));
    ops.push(Op::Query {
        query: closed_join_query(),
        params: vec![],
    });
    for family in families::all() {
        let query = (family.query)();
        ops.push(Op::Query {
            query: query.clone(),
            params: unit_draw(family.name, cfg.corpus_gen.seed, &sizes),
        });
        for params in (family.params)(&cfg.corpus_gen) {
            ops.push(Op::Query {
                query: query.clone(),
                params,
            });
        }
    }
    let (algebra, naive_only) = super::run_algebra::algebra_ops(cfg.corpus_gen.seed, &sizes);
    ops.extend(algebra);
    eprintln!(
        "verify: {naive_only} naive-only cases (Pack — SQLite-inexpressible by \
         `Inexpressible::PackAggregate`, enumerated, never silently skipped)"
    );

    let naive_dir = cfg.out_dir.join("naive-db");
    let _ = std::fs::remove_dir_all(&naive_dir);
    let db = Db::create(&naive_dir, Ledger)
        .expect("create naive-slice store")
        .expect("accepted");

    let mut naive = NaiveDb::new(&bumbledb::Theory::descriptor(Ledger));
    eprintln!("verify: naive differential slice ({} ops)", ops.len());
    match differential::run(&db, &mut naive, &ops) {
        Ok(summary) => {
            assert!(
                summary.aborts >= 7,
                "the violating deltas must abort (got {})",
                summary.aborts
            );
            run.cases += summary.commits + summary.aborts + summary.queries;
        }
        Err(divergence) => {
            let bundle = run.out_dir.join(format!("mismatch-{}", run.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            std::fs::write(
                bundle.join("mismatch.txt"),
                format!("naive differential slice diverged:\n{divergence:#?}\n"),
            )
            .expect("bundle");
            eprintln!("verify: NAIVE MISMATCH -> {}", bundle.display());
            run.bundles.push(bundle);
        }
    }
    if run.bundles.len() < super::MAX_BUNDLES {
        super::run_algebra::error_parity(&db, run);
    }
}
