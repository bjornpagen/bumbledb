//! THE MEASURE-OR-MERGE TWINS (cleanup-0.5.0 rulings 6 and 7,
//! `docs/prds/cleanup-0.5.0/prd-M-measure.md` items 1 and 2), prepared
//! by U2, MEASURED by the Measure phase alone (idle M2 Max, release,
//! threshold stated in the run's report BEFORE measuring — the house
//! law). Each twin runs the fast path against the generic machinery
//! interleaved (min-of-N, the pins idiom), asserts RESULT EQUIVALENCE
//! only, and prints the numbers: no margin is pinned here, because no
//! verdict exists yet — the Measure phase records law or executes the
//! merge. Invocation:
//!
//! ```text
//! cargo test -p bumbledb --release --lib -- --ignored \
//!     measure_twins --nocapture --test-threads=1
//! ```
//!
//! (The third twin of the trio — the permuted-identity determinant,
//! ruling 8 — lives beside its machinery in `storage/keys.rs` tests.)

use super::*;
use crate::exec::run::NoopCounters;

/// Answer rows as sorted debug strings — order-insensitive equivalence
/// between two execution routes.
fn row_set(out: &Answers) -> Vec<String> {
    let arity = out.arity();
    let mut rows: Vec<String> = (0..out.len())
        .map(|row| {
            (0..arity)
                .map(|col| format!("{:?}", out.get(row, col)))
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// Twin 1 — THE LEAF-ELISION COMPLEX (ruling 6): the single-subatom
/// leaf classification (`exec/run/leaf_precompute.rs`), the pinned-row
/// arm (`run_leaf_pinned`), and the scan arm, against the same plan
/// with the classification forced off (`Executor::disable_leaf_elision`,
/// the cfg(test) -off switch) — one measurement covers the complex.
/// The fixture mixes single-posting accounts (pinned-row leaf cursors)
/// with multi-posting accounts (scan leaf cursors) so both arms fire.
#[test]
#[ignore = "measure-or-merge twin: the Measure phase runs it (release, idle machine, --nocapture)"]
#[expect(
    clippy::too_many_lines,
    reason = "one twin reads as one protocol: fixture, firing proof, equivalence, interleaved timing"
)]
fn measure_twins_leaf_elision() {
    const REPS: usize = 7;
    const EXECS: usize = 10;
    let dir = TempDir::new("measure-twin-leaf-elision");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // 256 accounts with one posting each (pinned-row leaf cursors) and
    // 128 accounts with 8 postings each (scan leaf cursors).
    let mut rows: Vec<(u64, u64, String, i64)> = Vec::new();
    let mut id = 1u64;
    for account in 0..256u64 {
        rows.push((
            id,
            account,
            format!("m{}", id % 13),
            i64::try_from(id % 50).expect("small"),
        ));
        id += 1;
    }
    for account in 1000..1128u64 {
        for _ in 0..8 {
            rows.push((
                id,
                account,
                format!("m{}", id % 13),
                i64::try_from(id % 50).expect("small"),
            ));
            id += 1;
        }
    }
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    insert_postings(&env, &schema, &borrowed);

    // Q(x, y) :- Posting(account = A, amount = x), Posting(account = A,
    // amount = y) — the self-join whose leaf node carries exactly one
    // subatom with width-1 variables.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut elided = prepare(&txn, &cache, &schema, &query).expect("prepares");
    let mut generic = prepare(&txn, &cache, &schema, &query).expect("prepares");

    // A-arm firing proof + B-arm rerouting, on the executors themselves.
    let engaged = |prepared: &PreparedQuery<'_, ()>| match prepared.program.rules() {
        [PreparedRule::FreeJoin(rule)] => rule.executor.leaf_elision_engaged(),
        other => panic!(
            "the self-join prepares as one Free Join rule, got {} rules",
            other.len()
        ),
    };
    assert!(
        engaged(&elided),
        "the fixture must classify a single-subatom leaf, or the twin measures nothing"
    );
    let Program::Rules(rules) = &mut generic.program else {
        unreachable!("asserted Free Join above");
    };
    let [PreparedRule::FreeJoin(rule)] = rules.as_mut_slice() else {
        unreachable!("asserted Free Join above");
    };
    rule.executor.disable_leaf_elision();
    assert!(!engaged(&generic), "the B arm must route generic");

    // Equivalence: identical answer sets through both routes.
    let mut out_a = Answers::new();
    let mut out_b = Answers::new();
    elided.execute(&txn, &cache, &[], &mut out_a).expect("A");
    generic.execute(&txn, &cache, &[], &mut out_b).expect("B");
    assert!(!out_a.is_empty(), "a vacuous fixture measures nothing");
    assert_eq!(
        row_set(&out_a),
        row_set(&out_b),
        "the generic route must produce the identical answer set"
    );

    // Interleaved min-of-N timing (the pins idiom), EXECS whole
    // executions per sample.
    let mut elided_best = std::time::Duration::MAX;
    let mut generic_best = std::time::Duration::MAX;
    for _ in 0..REPS {
        let start = std::time::Instant::now();
        for _ in 0..EXECS {
            elided.execute(&txn, &cache, &[], &mut out_a).expect("A");
            std::hint::black_box(out_a.len());
        }
        elided_best = elided_best.min(start.elapsed());
        let start = std::time::Instant::now();
        for _ in 0..EXECS {
            generic.execute(&txn, &cache, &[], &mut out_b).expect("B");
            std::hint::black_box(out_b.len());
        }
        generic_best = generic_best.min(start.elapsed());
    }
    let per_exec = |d: std::time::Duration| d.as_nanos() / EXECS as u128;
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let ratio = per_exec(generic_best) as f64 / per_exec(elided_best) as f64;
    println!(
        "leaf-elision twin ({} answers/exec): elided {} ns/exec, generic {} ns/exec, generic/elided {ratio:.3}",
        out_a.len(),
        per_exec(elided_best),
        per_exec(generic_best),
    );
}

/// Twin 2 — THE ALL-WORDS FINALIZE FAST PATH (ruling 7):
/// `fill_word_answers` vs `fill_resolved_answers` and the two
/// aggregate-drain siblings (`api/prepared/finalize.rs`), isolated by
/// calling `finalize` directly with each `AnswerHeap` value on the same
/// filled sink — the routes differ in nothing else. Equivalence is
/// already falsifier-guarded end to end
/// (`tests/fixpoint_finalize_hunt.rs`); it is re-asserted here on the
/// twin's own fixture. Both sinks measured, per the ruling.
#[test]
#[ignore = "measure-or-merge twin: the Measure phase runs it (release, idle machine, --nocapture)"]
#[expect(
    clippy::too_many_lines,
    reason = "one twin reads as one protocol: fixture, equivalence, interleaved timing, both sinks"
)]
fn measure_twins_all_words_finalize() {
    const REPS: usize = 7;
    let dir = TempDir::new("measure-twin-all-words-finalize");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let rows: Vec<(u64, u64, &str, i64)> = (1..=20_000u64)
        .map(|id| (id, id % 997, "m", i64::try_from(id % 50).expect("small")))
        .collect();
    insert_postings(&env, &schema, &rows);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // One sub-twin per sink. `finalize` on a projection sink is a
    // non-mutating drain, so the sink fills once (untimed) and each rep
    // times one finalize per heap route; the aggregate sink finalizes
    // mutably (Pack sorts in place), so it refills (untimed) before
    // every timed finalize.
    let projection_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let aggregate_query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: crate::ir::AggOp::CountDistinct,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });

    for (name, query, refill_per_finalize) in [
        ("projection", &projection_query, false),
        ("aggregate", &aggregate_query, true),
    ] {
        let mut prepared = prepare(&txn, &cache, &schema, query).expect("prepares");
        assert_eq!(
            prepared.answer_heap,
            AnswerHeap::Words,
            "the fixture's finds are all word-backed, or the twin measures nothing"
        );
        let mut out = Answers::new();
        let run = |prepared: &mut PreparedQuery<'_, ()>| {
            let ran = prepared
                .run_rules(&txn, &cache, &mut NoopCounters)
                .expect("runs");
            assert!(ran, "the fixture rule must run");
        };
        let finalize_with =
            |prepared: &mut PreparedQuery<'_, ()>, heap: AnswerHeap, out: &mut Answers| {
                out.clear();
                out.arity = prepared.predicate.columns.len();
                super::finalize::finalize(
                    &mut prepared.sink,
                    &mut prepared.answer_scratch,
                    &mut prepared.resolve_memo,
                    &txn,
                    &prepared.predicate.columns,
                    heap,
                    out,
                )
                .expect("finalizes");
            };

        // Equivalence on this fixture (the hunt falsifier guards it
        // end to end; this is the twin's own in-place check).
        run(&mut prepared);
        let mut words = Answers::new();
        finalize_with(&mut prepared, AnswerHeap::Words, &mut words);
        if refill_per_finalize {
            run(&mut prepared);
        }
        let mut resolved = Answers::new();
        finalize_with(&mut prepared, AnswerHeap::Bytes, &mut resolved);
        assert!(!words.is_empty(), "a vacuous fixture measures nothing");
        assert_eq!(
            row_set(&words),
            row_set(&resolved),
            "the resolving route must produce the identical answer set"
        );

        let mut words_best = std::time::Duration::MAX;
        let mut resolved_best = std::time::Duration::MAX;
        for _ in 0..REPS {
            if refill_per_finalize {
                run(&mut prepared);
            }
            let start = std::time::Instant::now();
            finalize_with(&mut prepared, AnswerHeap::Words, &mut out);
            words_best = words_best.min(start.elapsed());
            std::hint::black_box(out.len());
            if refill_per_finalize {
                run(&mut prepared);
            }
            let start = std::time::Instant::now();
            finalize_with(&mut prepared, AnswerHeap::Bytes, &mut out);
            resolved_best = resolved_best.min(start.elapsed());
            std::hint::black_box(out.len());
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let ratio = resolved_best.as_nanos() as f64 / words_best.as_nanos() as f64;
        println!(
            "all-words finalize twin, {name} sink ({} answers): words {} ns, resolved {} ns, resolved/words {ratio:.3}",
            words.len(),
            words_best.as_nanos(),
            resolved_best.as_nanos(),
        );
    }
}
