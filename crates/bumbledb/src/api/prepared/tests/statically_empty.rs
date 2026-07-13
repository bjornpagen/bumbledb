//! Statically empty: condition folding at normalize (docs/architecture/
//! 20-query-ir.md § normalization, 40-execution.md § access paths). A
//! rule whose constant conditions are mutually unsatisfiable dies at
//! prepare; a program of only dead rules prepares to `Program::Empty`
//! — params bind first (errors surface), then nothing runs. The fold's
//! set-preservation rides the folded/unfolded differential below.

use super::*;
use crate::allen::AllenMask;
use crate::encoding::ValueRef;
use crate::ir::normalize::with_fold_disabled;
use crate::ir::{HeadTerm, MaskTerm, ParamId};
use crate::schema::{IntervalElement, SchemaDescriptor};

/// Event(id u64 fresh, kind u64, during interval<i64>, score i64) — the
/// interval field feeds the mask-param leg; kind splits the rules.
fn event_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Event".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "score".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const EVENT: RelationId = RelationId(0);

fn insert_events(env: &Environment, schema: &Schema, rows: &[(u64, u64, (i64, i64), i64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, kind, (start, end), score) in rows {
        let mut bytes = Vec::new();
        crate::encoding::encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*kind),
                ValueRef::IntervalI64(
                    crate::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
                ValueRef::I64(*score),
            ],
            schema.relation(EVENT).layout(),
            &mut bytes,
        );
        delta.insert(&view, EVENT, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// One rule over Event: `Event(kind == <kind>, score: v0)` plus the
/// given extra comparisons on v0 — finds (v0).
fn by_kind_rule(kind: u64, conditions: Vec<Comparison>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: EVENT,
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(kind))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: conditions.into_iter().map(ConditionTree::Leaf).collect(),
    }
}

fn score_cmp(op: CmpOp, value: i64) -> Comparison {
    Comparison {
        op,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Literal(Value::I64(value)),
    }
}

/// `score > 5 ∧ score < 3` — the statically-empty kernel of every dead
/// rule below.
fn contradiction() -> Vec<Comparison> {
    vec![score_cmp(CmpOp::Gt, 5), score_cmp(CmpOp::Lt, 3)]
}

fn scores_of(buffer: &ResultBuffer) -> Vec<i64> {
    let mut scores: Vec<i64> = (0..buffer.len())
        .map(|row| {
            let ResultValue::I64(score) = buffer.get(row, 0) else {
                panic!("column 0 is an i64");
            };
            score
        })
        .collect();
    scores.sort_unstable();
    scores
}

#[test]
fn a_dead_rule_beside_a_live_one_runs_the_live_one_only() {
    let dir = TempDir::new("statically-empty-multi");
    let schema = event_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_events(
        &env,
        &schema,
        &[
            (1, 3, (0, 10), 10),
            (2, 3, (0, 10), 25),
            (3, 7, (0, 10), 40),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction()), by_kind_rule(7, vec![])],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    // The dead rule was deleted at prepare — only the live plan exists.
    assert_eq!(
        prepared.program.rules().len(),
        1,
        "the dead rule prepared no plan"
    );
    assert!(matches!(
        prepared.program.rules(),
        [PreparedRule::FreeJoin(_)]
    ));

    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(scores_of(&out), vec![40], "kind 7's row; kind 3 never ran");

    // The death record names the killing condition — EXPLAIN's line.
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(stats.rules.len(), 1, "stats cover the live rule only");
    assert_eq!(stats.dead.len(), 1);
    assert_eq!(stats.dead[0].rule, 0);
    assert_eq!(stats.dead[0].rendered, "Event: score > 5 ∧ score < 3");
    let (_, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert!(
        report.contains("statically empty: rule 0: Event: score > 5 ∧ score < 3"),
        "{report}"
    );
}

/// The one-RULE-span proof: the dead rule not only emits nothing, it
/// never enters the rule loop at all.
#[cfg(feature = "trace")]
#[test]
fn a_dead_rule_opens_no_rule_span() {
    use crate::obs;

    let dir = TempDir::new("statically-empty-span");
    let schema = event_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_events(&env, &schema, &[(1, 7, (0, 10), 40)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction()), by_kind_rule(7, vec![])],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let events = obs::finish_capture();
    let rule_spans: Vec<&str> = events
        .iter()
        .map(|e| e.name)
        .filter(|name| name.starts_with("rule_"))
        .collect();
    assert_eq!(rule_spans, vec!["rule_0"], "one rule span: the live rule");
}

#[test]
fn an_all_dead_program_prepares_to_empty_and_binds_params_first() {
    let dir = TempDir::new("statically-empty-all");
    let schema = event_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_events(&env, &schema, &[(1, 3, (0, 10), 10)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Both rules die on the score contradiction; rule 1 also carries an
    // Allen mask param — params are stage-3, so the mask leg never
    // folds, and bind must still judge it on the empty program.
    let mut masked = contradiction();
    masked.push(Comparison {
        op: CmpOp::Allen {
            mask: MaskTerm::Param(ParamId(0)),
        },
        lhs: Term::Var(VarId(1)),
        rhs: Term::Literal(Value::IntervalI64(
            crate::Interval::<i64>::new(7, 9).expect("nonempty interval"),
        )),
    });
    let mut rule1 = by_kind_rule(7, masked);
    rule1.atoms[0]
        .bindings
        .push((FieldId(2), Term::Var(VarId(1))));
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction()), rule1],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.program, Program::Empty),
        "all rules dead: the empty program"
    );

    // Params bind FIRST: a vacuous mask param still errors, exactly as
    // on a live plan.
    let err = prepared
        .execute_collect(&txn, &cache, &[BindValue::AllenMask(AllenMask::EMPTY)])
        .expect_err("a vacuous mask param must still be rejected");
    assert!(
        matches!(err, Error::EmptyAllenMaskParam { param: ParamId(0) }),
        "typed, named: {err:?}"
    );

    // A well-formed bind executes to the empty result — correctly
    // shaped: the empty program still has an arity and buffer types,
    // read off the predicate (it sits beside the program exactly so
    // this path can type an empty buffer).
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::AllenMask(AllenMask::INTERSECTS)])
        .expect("execute");
    assert_eq!(out.len(), 0, "stage-2-known empty");
    assert_eq!(out.arity(), 1, "the predicate shapes the empty buffer");

    // EXPLAIN prints the program kind and both killing conditions.
    let (out, report) = prepared
        .explain(&txn, &cache, &[BindValue::AllenMask(AllenMask::INTERSECTS)])
        .expect("explain");
    assert_eq!(out.len(), 0);
    assert!(report.contains("access path: statically empty"), "{report}");
    assert!(
        report.contains("statically empty: rule 0: Event: score > 5 ∧ score < 3"),
        "{report}"
    );
    assert!(
        report.contains("statically empty: rule 1: Event: score > 5 ∧ score < 3"),
        "{report}"
    );
}

/// The `[shape]` leg: the empty program touches no image and binds no view
/// — the obs counters that would record either stay silent.
#[cfg(feature = "trace")]
#[test]
fn the_empty_program_builds_no_image_and_binds_no_view() {
    use crate::obs;

    let dir = TempDir::new("statically-empty-no-images");
    let schema = event_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_events(&env, &schema, &[(1, 3, (0, 10), 10)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction())],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(prepared.program, Program::Empty));

    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let events = obs::finish_capture();
    assert_eq!(out.len(), 0);
    let names: Vec<&str> = events.iter().map(|e| e.name).collect();
    for touched in [
        obs::names::IMAGE_BUILD,
        obs::names::CACHE_HIT,
        obs::names::VIEW_BUILD,
        obs::names::JOIN,
    ] {
        assert!(
            !names.contains(&touched),
            "the empty program must not reach {touched}: {names:?}"
        );
    }
}

/// Fold-preservation: randomized single-slot filter sets, folded vs
/// unfolded (the `with_fold_disabled` switch — the ground-off precedent)
/// over one fixture corpus, identical results. Folding is conjunction
/// reassociation over one slot's total order — set-preserving by
/// construction; this pins it against the executor.
#[test]
fn folded_and_unfolded_executions_agree_on_random_single_slot_filters() {
    let dir = TempDir::new("statically-empty-differential");
    let schema = event_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let rows: Vec<(u64, u64, (i64, i64), i64)> = (0..40u64)
        .map(|i| {
            let score = i64::try_from(i).expect("small") - 20; // spans [-20, 19]
            (i + 1, i % 5, (0, 10), score)
        })
        .collect();
    insert_events(&env, &schema, &rows);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // The house xorshift — no rand dependency (the dependency law).
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for round in 0..64 {
        // 1..=4 comparisons, all on the ONE score slot, operators and
        // constants drawn to make merges, contradictions, and Eq pins
        // all likely.
        let count = next() % 4 + 1;
        let conditions: Vec<Comparison> = (0..count)
            .map(|_| {
                let op = match next() % 6 {
                    0 => CmpOp::Lt,
                    1 => CmpOp::Le,
                    2 => CmpOp::Gt,
                    3 => CmpOp::Ge,
                    4 => CmpOp::Eq,
                    _ => CmpOp::Ne,
                };
                let value = i64::try_from(next() % 17).expect("small") - 8;
                score_cmp(op, value)
            })
            .collect();
        let query = Query::single(by_kind_rule(next() % 5, conditions));

        let mut folded = prepare(&txn, &cache, &schema, &query).expect("prepare folded");
        let mut unfolded =
            with_fold_disabled(|| prepare(&txn, &cache, &schema, &query)).expect("prepare raw");
        let folded_rows = scores_of(&folded.execute_collect(&txn, &cache, &[]).expect("folded"));
        let unfolded_rows = scores_of(
            &unfolded
                .execute_collect(&txn, &cache, &[])
                .expect("unfolded"),
        );
        assert_eq!(
            folded_rows, unfolded_rows,
            "round {round}: the fold changed the denotation"
        );
    }
}
