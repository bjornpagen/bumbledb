use super::{Db, Run, VerifyConfig};

use bumbledb::{Atom, FieldId, FindTerm, Interval, Query, RelationId, Rule, Term, Value, VarId};

use crate::corpus_gen::{self, MANDATE_SEGMENTS, Sizes, mandate_segments};
use crate::differential::{self, Op};
use crate::families::{self, Draw, scalar_draw};
use crate::naive::{Delta, NaiveDb, ParamValue};
use crate::schema::{Ledger, ids};

/// The unit-scale corpus of the naive lane: small enough for the
/// brute-force model's nested loops, large enough that every family's
/// joins have witnesses.
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

/// The closed-vocabulary read: accounts joined to `Currency` on the
/// handle — the closed atom is an ordinary atom on the naive side too
/// (the model reads its seeded extension), so engine and model compare
/// closed-relation reads exactly like any other join.
fn closed_join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, Term::Var(VarId(0))),
                    (ids::account::CURRENCY, Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: ids::CURRENCY,
                bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// The insert stream as write deltas, chunked — every chunk judged over
/// the full final state on both sides.
fn load_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let cfg = corpus_gen::GenConfig {
        seed,
        scale: corpus_gen::Scale::S, // unused: rows take explicit unit sizes
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

/// Deltas that must ABORT, verdict and violating statement agreeing on
/// both sides: a dangling containment source, a pointwise-key overlap,
/// a scalar-key duplicate, a target-required delete, the
/// net-disposition pattern class (a redundant insert alongside a delete
/// of its containment target — the Direction-divergence shape), a write
/// naming a closed relation (`ClosedRelationWrite`, typed on both
/// oracles), and an out-of-range closed-vocabulary reference (the
/// compiled-subset miss — the same containment violation as any
/// dangling reference).
fn violating_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let cfg = corpus_gen::GenConfig {
        seed,
        scale: corpus_gen::Scale::S,
    };
    let segment = mandate_segments(seed, sizes, 0)[0];
    let overlap = Interval::<i64>::new(segment.start, segment.start + 1).expect("nonempty");
    vec![
        // Posting -> Account containment, source side.
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
        // The pointwise Mandate key: a one-point overlap on account 0
        // under a different org (the identical fact would be a no-op).
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
        // The Holder fresh key: a second fact under id 0.
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::HOLDER,
                vec![
                    Value::U64(0),
                    Value::String(b"holder-duplicate".to_vec().into()),
                ],
            )],
        }),
        // Deleting account 0 strands its postings and mandates: the
        // target-required direction.
        Op::Write(Delta {
            deletes: vec![(ids::ACCOUNT, corpus_gen::row(&cfg, sizes, ids::ACCOUNT, 0))],
            inserts: vec![],
        }),
        // The net-disposition pattern class: a committed posting
        // deleted and re-inserted (netting to nothing) alongside the
        // delete of its containment target — the posting was not
        // genuinely added, so the verdict classifies target-side on both
        // oracles, Direction included.
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
        // typed `ClosedRelationWrite` verdict on both.
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(ids::CURRENCY, vec![Value::U64(5)])],
        }),
        // An out-of-range vocabulary reference: currency 9 is beyond the
        // three-row extension, so `Account(currency) <= Currency(id)`
        // misses — source-unsatisfied, exactly like any dangling id.
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

/// One in-domain draw per family at unit scale (the S-scale rotations
/// are mostly misses here; these make the joins produce witnesses).
fn unit_draw(name: &str, seed: u64, sizes: &Sizes) -> Draw {
    let cfg = corpus_gen::GenConfig {
        seed,
        scale: corpus_gen::Scale::S,
    };
    let span = i64::try_from(sizes.postings).expect("fits") * corpus_gen::AT_STEP;
    match name {
        "point" => scalar_draw(vec![Value::U64(3)]),
        "containment_walk" | "postings_without_tag" | "skew" => scalar_draw(vec![Value::U64(0)]),
        "chain" => scalar_draw(vec![Value::I64(corpus_gen::AT_BASE)]),
        "range" => scalar_draw(vec![
            Value::I64(corpus_gen::AT_BASE + span / 4),
            Value::I64(corpus_gen::AT_BASE + span / 2),
        ]),
        // orgs and holders both have >1 unit-scale id 1.
        "balance" | "mandate_overlap" => scalar_draw(vec![Value::U64(1)]),
        "stats" | "spread" | "latest_posting_per_account" => scalar_draw(vec![]),
        "string" => scalar_draw(vec![Value::String(b"SYM0003".to_vec().into())]),
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

/// The naive-model differential slice (docs/architecture/60-validation.md
/// § the two oracles): a fresh
/// unit-scale store replays the corpus stream, seven judgment-violating
/// deltas (the closed-relation write refusal and the out-of-range
/// vocabulary reference included), the closed-vocabulary join read,
/// every family query (its unit draw plus its seeded S
/// rotation), and the algebra oracle rows (`run_algebra`: rules, DNF
/// trees, `Pack` — naive-only by decision, counted and reported — and
/// the measure's ray verdicts) against [`NaiveDb`]; any verdict,
/// violator, or result-set disagreement is an arbitration bundle. The
/// error-parity cases (cap-exceeding DNF, vacuous masks) run after the
/// differential, against the same store.
///
/// # Panics
///
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
    let db = Db::create(&naive_dir, Ledger).expect("create naive-slice store");
    // The declared descriptor, extensions included — the model seeds the
    // closed vocabularies from the ground axioms at construction.
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
