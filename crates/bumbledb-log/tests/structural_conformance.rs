//! Native correspondence with the independent rational and endpoint-cell corpus.
use bumbledb::{Interval, Rounding, ScalarError, ScalarEvaluator, ScalarExpr, Value};
use serde_json::Value as Json;

#[test]
fn structural_algebra_matches_the_independent_corpus() {
    let corpus: Json = serde_json::from_str(include_str!(
        "../../bumbledb-bench/fixtures/conformance/structural-algebra.json"
    ))
    .unwrap();
    let evaluator = ScalarEvaluator::new().unwrap();
    for row in corpus["quotients"].as_array().unwrap() {
        let kind = row[0].as_str().unwrap();
        let value = |i: usize| {
            let s = row[i].as_str().unwrap();
            ScalarExpr::Literal(if kind == "i64" {
                Value::I64(s.parse().unwrap())
            } else {
                Value::U64(s.parse().unwrap())
            })
        };
        let expr = ScalarExpr::MulDiv {
            a: Box::new(value(1)),
            b: Box::new(value(2)),
            divisor: Box::new(value(3)),
            rounding: Rounding::from_name(row[4].as_str().unwrap()).unwrap(),
        };
        let got = match evaluator.evaluate(&expr, |_| unreachable!()) {
            Ok(Value::I64(n)) => n.to_string(),
            Ok(Value::U64(n)) => n.to_string(),
            Err(ScalarError::DivisionByZero) => "divisionByZero".into(),
            Err(ScalarError::NonPositiveDivisor) => "nonPositiveDivisor".into(),
            Err(ScalarError::Overflow) => "overflow".into(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(got, row[5].as_str().unwrap(), "{row}");
    }
    let span = |j: &Json| {
        Interval::new(
            j[0].as_str().unwrap().parse::<i64>().unwrap(),
            j[1].as_str().unwrap().parse::<i64>().unwrap(),
        )
        .unwrap()
    };
    for row in corpus["intervals"].as_array().unwrap() {
        let a = span(&row[0]);
        let b = span(&row[1]);
        let common: Vec<_> = row[2].as_array().unwrap().iter().map(span).collect();
        let rest: Vec<_> = row[3].as_array().unwrap().iter().map(span).collect();
        assert_eq!(
            a.intersection(b).into_iter().collect::<Vec<_>>(),
            common,
            "{row}"
        );
        assert_eq!(
            a.difference(b).into_iter().flatten().collect::<Vec<_>>(),
            rest,
            "{row}"
        );
    }
}
