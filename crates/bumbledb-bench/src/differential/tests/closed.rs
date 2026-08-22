use bumbledb::{Direction, Value};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::differential::{Op, run};
use crate::fixture::{TempDir, string};
use crate::naive::{Delta, NaiveDb, ParamValue, Violation};
use crate::querygen::target::{self, ids};
use crate::querygen::writes::{ClosedWriteCase, ClosedWriteKind, closed_write_cases};
use crate::querygen::{params_for, random_query};

const CFG: GenConfig = GenConfig {
    seed: 21,
    scale: Scale::S,
};

pub(super) fn base_delta() -> Delta {
    Delta {
        deletes: vec![],
        inserts: vec![
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
        ],
    }
}

fn case_delta(case: &ClosedWriteCase) -> Delta {
    if case.delete {
        Delta {
            deletes: vec![(case.relation, case.fact.clone())],
            inserts: vec![],
        }
    } else {
        Delta {
            deletes: vec![],
            inserts: vec![(case.relation, case.fact.clone())],
        }
    }
}

#[test]
fn the_closed_write_classes_agree_with_the_engine() {
    let dir = TempDir::new("differential-closed");
    let db = target::publish_admitted(dir.path());
    let mut naive = NaiveDb::new(&target::descriptor());

    let mut rng = Rng::new(CFG.seed);
    let cases = closed_write_cases(&mut rng, 18);
    for kind in [
        ClosedWriteKind::ClosedInsert,
        ClosedWriteKind::ClosedDelete,
        ClosedWriteKind::DanglingHandle,
        ClosedWriteKind::BeyondRosterCap,
        ClosedWriteKind::PsiExcluded,
        ClosedWriteKind::PsiOutOfRange,
    ] {
        assert!(
            cases.iter().any(|case| case.kind == kind),
            "the batch must cover {kind:?}"
        );
    }

    let strand = Delta {
        deletes: vec![(
            ids::CURRENCY_BACKING,
            vec![Value::U64(1), Value::U64(1_001)],
        )],
        inserts: vec![],
    };
    let replace = Delta {
        deletes: vec![(
            ids::CURRENCY_BACKING,
            vec![Value::U64(0), Value::U64(1_000)],
        )],
        inserts: vec![(
            ids::CURRENCY_BACKING,
            vec![Value::U64(0), Value::U64(5_000)],
        )],
    };

    let mut ops = vec![Op::Write(base_delta())];
    ops.extend(cases.iter().map(|case| Op::Write(case_delta(case))));

    ops.push(Op::Write(strand.clone()));
    ops.push(Op::Write(replace));
    ops.push(Op::Write(Delta {
        deletes: vec![(
            ids::CASH_ROUNDING,
            vec![Value::U64(target::ZERO_DECIMAL_CURRENCY)],
        )],
        inserts: vec![],
    }));

    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: vec![(
            ids::ACCOUNT,
            vec![Value::U64(3), Value::U64(1), Value::U64(1)],
        )],
    }));

    for _ in 0..25 {
        let query = random_query(&mut rng, CFG);
        for draw in params_for(&query, &mut rng, CFG) {
            let mut params: Vec<ParamValue> =
                vec![ParamValue::Scalar(Value::Bool(false)); draw.scalars.len() + draw.sets.len()];
            for (param, value) in &draw.scalars {
                params[usize::from(param.0)] = ParamValue::Scalar(value.clone());
            }
            for (param, values) in &draw.sets {
                params[usize::from(param.0)] = ParamValue::Set(values.clone());
            }
            ops.push(Op::Query {
                query: query.clone(),
                params,
            });
        }
    }

    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree: {divergence:#?}");
    });
    assert_eq!(
        summary.aborts,
        cases.len() as u64 + 1,
        "every judgment case plus the stranding delete aborts"
    );
    assert_eq!(summary.commits, 4, "seed, replacement, drop, control");
    assert_eq!(summary.queries, 100, "25 queries x 4 draws");

    for case in &cases {
        assert_eq!(
            naive.apply(&case_delta(case)),
            Err(vec![case.expected]),
            "{:?} must abort with its hand-derived violation",
            case.kind
        );
    }
    assert_eq!(
        naive.apply(&strand),
        Err(vec![Violation::Containment {
            statement: target::CURRENCY_BACKED,
            direction: Direction::TargetRequired,
        }]),
        "the domain quantification judges the stranded axiom target-side"
    );
}
