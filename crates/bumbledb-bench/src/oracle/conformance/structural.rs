//! The engine against `structural-algebra.json`: exact `MulDiv` quotients and
//! interval intersection/difference, whose expectations come from independent
//! Python rational and endpoint-cell oracles (`scripts/structural-corpus.py`).
use bumbledb::{Interval, Rounding, ScalarError, ScalarEvaluator, ScalarExpr, Value};

use crate::json::{self, Value as Json};

fn text(row: &[Json], index: usize) -> &str {
    row[index].as_str().expect("corpus cells are strings")
}

fn quotient(evaluator: &ScalarEvaluator, row: &[Json]) -> String {
    let signed = text(row, 0) == "i64";
    let literal = |index: usize| {
        let cell = text(row, index);
        Box::new(ScalarExpr::Literal(if signed {
            Value::I64(cell.parse().expect("an i64 cell"))
        } else {
            Value::U64(cell.parse().expect("a u64 cell"))
        }))
    };
    let expression = ScalarExpr::MulDiv {
        a: literal(1),
        b: literal(2),
        divisor: literal(3),
        rounding: Rounding::from_name(text(row, 4)).expect("a rounding name"),
    };
    match evaluator.evaluate(&expression, |_| unreachable!("literal-only expression")) {
        Ok(Value::I64(n)) => n.to_string(),
        Ok(Value::U64(n)) => n.to_string(),
        Err(ScalarError::DivisionByZero) => "divisionByZero".to_owned(),
        Err(ScalarError::NonPositiveDivisor) => "nonPositiveDivisor".to_owned(),
        Err(ScalarError::Overflow) => "overflow".to_owned(),
        other => panic!("unexpected quotient outcome {other:?}"),
    }
}

fn span(cell: &Json) -> Interval<i64> {
    let ends = cell.as_arr().expect("a span is a pair");
    Interval::new(
        text(ends, 0).parse().expect("an i64 start"),
        text(ends, 1).parse().expect("an i64 end"),
    )
    .expect("corpus spans are nonempty")
}

fn spans(cell: &Json) -> Vec<Interval<i64>> {
    cell.as_arr()
        .expect("a span list")
        .iter()
        .map(span)
        .collect()
}

#[test]
fn the_engine_matches_the_independent_structural_corpus() {
    let corpus = json::parse(
        &std::fs::read_to_string(super::corpus_dir().join("structural-algebra.json"))
            .expect("read the structural corpus"),
    )
    .expect("the structural corpus parses");
    let evaluator = ScalarEvaluator::new().expect("the default float environment");
    let quotients = corpus
        .get("quotients")
        .and_then(Json::as_arr)
        .expect("quotient rows");
    assert!(!quotients.is_empty());
    for row in quotients {
        let row = row.as_arr().expect("a quotient row");
        assert_eq!(quotient(&evaluator, row), text(row, 5), "{row:?}");
    }
    let intervals = corpus
        .get("intervals")
        .and_then(Json::as_arr)
        .expect("interval rows");
    assert!(!intervals.is_empty());
    for row in intervals {
        let row = row.as_arr().expect("an interval row");
        let (a, b) = (span(&row[0]), span(&row[1]));
        assert_eq!(
            a.intersection(b).into_iter().collect::<Vec<_>>(),
            spans(&row[2]),
            "{row:?}"
        );
        assert_eq!(
            a.difference(b).into_iter().flatten().collect::<Vec<_>>(),
            spans(&row[3]),
            "{row:?}"
        );
    }
}
