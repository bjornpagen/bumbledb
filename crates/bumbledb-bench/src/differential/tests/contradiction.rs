use bumbledb::{Atom, FindTerm, Query, Rule, Term, Value, VarId};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::differential::{Op, engine_query, run};
use crate::fixture::{TempDir, string};
use crate::naive::query::ParamValue;
use crate::naive::{Delta, NaiveDb};
use crate::querygen::target::{self, ids};
use crate::querygen::{contradiction_query, params_for};

const CFG: GenConfig = GenConfig {
    seed: 33,
    scale: Scale::S,
};

fn base_delta() -> Delta {
    let mut inserts = vec![
        (ids::HOLDER, vec![Value::U64(0), string("h0")]),
        (ids::HOLDER, vec![Value::U64(1), string("h1")]),
        (
            ids::ACCOUNT,
            vec![Value::U64(0), Value::U64(0), Value::U64(0)],
        ),
        (
            ids::ACCOUNT,
            vec![Value::U64(1), Value::U64(1), Value::U64(2)],
        ),
        (
            ids::ACCOUNT,
            vec![Value::U64(2), Value::U64(0), Value::U64(1)],
        ),
        (ids::INSTRUMENT, vec![Value::U64(0), string("i0")]),
        (ids::INSTRUMENT, vec![Value::U64(1), string("i1")]),
        (
            ids::CURRENCY_BACKING,
            vec![Value::U64(0), Value::U64(1_000)],
        ),
        (
            ids::CURRENCY_BACKING,
            vec![Value::U64(1), Value::U64(1_001)],
        ),
        (
            ids::CURRENCY_BACKING,
            vec![Value::U64(2), Value::U64(1_002)],
        ),
        (
            ids::CASH_ROUNDING,
            vec![Value::U64(target::ZERO_DECIMAL_CURRENCY)],
        ),
        (
            ids::JOURNAL_ENTRY,
            vec![
                Value::U64(0),
                Value::U64(0),
                Value::I64(target::posting_at(0)),
            ],
        ),
        (
            ids::JOURNAL_ENTRY,
            vec![
                Value::U64(1),
                Value::U64(2),
                Value::I64(target::posting_at(2)),
            ],
        ),
    ];
    for id in 0..6u64 {
        inserts.push((
            ids::POSTING,
            vec![
                Value::U64(id),
                Value::U64(id % 2),
                Value::U64(id % 3),
                Value::U64(id % 2),
                Value::I64(1_000 * i64::try_from(id + 1).expect("small")),
                Value::I64(target::posting_at(id)),
                string(if id % 2 == 0 { "even" } else { "odd" }),
                Value::Bool(id % 3 == 0),
            ],
        ));
    }
    Delta {
        deletes: vec![],
        inserts,
    }
}

#[test]
fn contradiction_draws_are_empty_on_both_sides() {
    let dir = TempDir::new("differential-contradiction");
    let db = target::publish_admitted(dir.path());
    let mut naive = NaiveDb::new(&target::descriptor());

    let mut rng = Rng::new(CFG.seed);
    let mut ops = vec![Op::Write(base_delta())];
    let mut draws = Vec::new();
    for _ in 0..20 {
        let query = contradiction_query(&mut rng, CFG);
        for draw in params_for(&query, &mut rng, CFG) {
            let mut params: Vec<ParamValue> =
                vec![ParamValue::Scalar(Value::Bool(false)); draw.scalars.len() + draw.sets.len()];
            for (param, value) in &draw.scalars {
                params[usize::from(param.0)] = ParamValue::Scalar(value.clone());
            }
            for (param, values) in &draw.sets {
                params[usize::from(param.0)] = ParamValue::Set(values.clone());
            }
            draws.push((query.clone(), params.clone()));
            ops.push(Op::Query {
                query: query.clone(),
                params,
            });
        }
    }

    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree: {divergence:#?}");
    });
    assert_eq!(summary.commits, 1, "the seed commits");
    assert_eq!(summary.queries, draws.len() as u64);

    for (query, params) in &draws {
        assert!(
            naive
                .query(query, params)
                .expect("the model evaluates what validation accepted")
                .is_empty(),
            "a poisoned rule survived the fold's vocabulary"
        );
    }

    let control = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
            bindings: vec![(ids::account::ID, Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let crate::differential::Answers::Ok(rows) = engine_query(&db, &control, &[]) else {
        panic!("the control scan errors");
    };
    assert!(!rows.is_empty(), "the control scan sees the seed");
}
