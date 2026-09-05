use super::*;
use crate::{F64, FoldOp};

const READING: RelationId = RelationId(0);

fn float_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Reading".into(),
            fields: [
                ("group", ValueType::U64),
                ("id", ValueType::U64),
                ("value", ValueType::F64),
            ]
            .into_iter()
            .map(|(name, value_type)| FieldDescriptor {
                name: name.into(),
                value_type,
            })
            .collect(),
        }],
        statements: vec![],
    }
}

fn readings(rows: &[(u64, u64, F64)]) -> Fix {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|&(group, id, value)| vec![Value::U64(group), Value::U64(id), Value::F64(value)])
        .collect();
    Fix::heap(float_descriptor(), &[(READING, facts)])
}

fn reduction_rule(identity: bool) -> Rule {
    let mut bindings = vec![
        (FieldId(0), Term::Var(VarId(0))),
        (FieldId(2), Term::Var(VarId(2))),
    ];
    if identity {
        bindings.push((FieldId(1), Term::Var(VarId(1))));
    }
    Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Count,
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(2),
            },
            FindTerm::Aggregate {
                op: FoldOp::Mean,
                over: VarId(2),
            },
        ],
        atoms: vec![Atom {
            source: AtomSource::Edb(READING),
            bindings,
        }],
        negated: vec![],
        conditions: vec![],
    }
}

#[test]
fn real_query_sum_mean_match_all_independent_rational_fixture_bits() {
    let mut rows = Vec::new();
    let mut expected = std::collections::BTreeMap::new();
    for line in include_str!("../../../../tests/fixtures/f64_reference.txt").lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.first() != Some(&"reduce") {
            continue;
        }
        let bits = |word: &str| u64::from_str_radix(word, 16).unwrap();
        let group = expected.len() as u64;
        expected.insert(
            group,
            ((words.len() - 3) as u64, bits(words[1]), bits(words[2])),
        );
        for (id, word) in words[3..].iter().enumerate() {
            rows.push((group, id as u64, F64::from_bits(bits(word))));
        }
    }
    assert_eq!(expected.len(), 317);
    rows.reverse();
    let fix = readings(&rows);
    let mut prepared = fix.prepare(&Query::single(reduction_rule(true))).unwrap();
    for _ in 0..2 {
        let answers = fix.execute(&mut prepared, &[] as &[BindValue]).unwrap();
        let actual: std::collections::BTreeMap<_, _> = (0..answers.len())
            .map(|row| {
                let (
                    AnswerValue::U64(group),
                    AnswerValue::U64(count),
                    AnswerValue::F64(sum),
                    AnswerValue::F64(mean),
                ) = (
                    answers.get(row, 0),
                    answers.get(row, 1),
                    answers.get(row, 2),
                    answers.get(row, 3),
                )
                else {
                    panic!("typed float aggregate answers")
                };
                (group, (count, sum.to_bits(), mean.to_bits()))
            })
            .collect();
        assert_eq!(actual, expected);
    }
}

#[test]
fn float_folds_keep_full_binding_grain_dnf_transparency_and_no_empty_group() {
    let fix = readings(&[(0, 1, F64::from(3.0)), (0, 2, F64::from(3.0))]);
    for (identity, dnf, expected_count) in [
        (true, false, 2),
        (true, true, 2),
        (false, false, 1),
        (false, true, 1),
    ] {
        let mut rule = reduction_rule(identity);
        if dnf {
            let leaf = ConditionTree::Leaf(Comparison {
                op: CmpOp::Eq,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(Value::U64(0)),
            });
            rule.conditions
                .push(ConditionTree::Or(vec![leaf.clone(), leaf]));
        }
        let mut prepared = fix.prepare(&Query::single(rule)).unwrap();
        let answers = fix.execute(&mut prepared, &[] as &[BindValue]).unwrap();
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 1), AnswerValue::U64(expected_count));
        assert_eq!(
            answers.get(0, 2),
            AnswerValue::F64(F64::from(if identity { 6.0 } else { 3.0 }))
        );
        assert_eq!(answers.get(0, 3), AnswerValue::F64(F64::from(3.0)));
    }
    let mut rule = reduction_rule(true);
    rule.finds.remove(0); // ungrouped reductions also produce no row on empty input
    rule.conditions.push(ConditionTree::Leaf(Comparison {
        op: CmpOp::Eq,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Literal(Value::U64(1)),
    }));
    let mut prepared = fix.prepare(&Query::single(rule)).unwrap();
    assert_eq!(
        fix.execute(&mut prepared, &[] as &[BindValue])
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn mean_requires_explicit_float_input() {
    let fix = readings(&[]);
    let mut rule = reduction_rule(true);
    rule.finds = vec![FindTerm::Aggregate {
        op: FoldOp::Mean,
        over: VarId(1),
    }];
    assert!(matches!(
        fix.prepare(&Query::single(rule)),
        Err(Error::Validation(
            crate::error::ValidationError::AggregateInputType { .. }
        ))
    ));
}

#[test]
fn spilled_and_resident_float_reductions_agree_bit_for_bit() {
    // F-RESOURCE / F-AGG (execution half): a zero sink-RAM allowance moves
    // the dedup/result state to scratch; sum/mean bits are unchanged.
    let rows: Vec<(u64, u64, F64)> = (0..64)
        .map(|i| {
            (
                i % 4,
                i,
                F64::from(f64::from(u32::try_from(i).expect("64 rows")) * 0.1 - 3.0),
            )
        })
        .collect();
    let fix = readings(&rows);
    let query = Query::single(reduction_rule(true));
    let mut resident = fix.prepare(&query).unwrap();
    let expected = fix.execute(&mut resident, &[] as &[BindValue]).unwrap();
    let mut spilled = fix.prepare(&query).unwrap();
    spilled.set_sink_ram(0);
    let got = fix.execute(&mut spilled, &[] as &[BindValue]).unwrap();
    let render = |answers: &Answers| -> Vec<(u64, u64, u64, u64)> {
        let mut rows: Vec<_> = (0..answers.len())
            .map(|row| {
                let (
                    AnswerValue::U64(group),
                    AnswerValue::U64(count),
                    AnswerValue::F64(sum),
                    AnswerValue::F64(mean),
                ) = (
                    answers.get(row, 0),
                    answers.get(row, 1),
                    answers.get(row, 2),
                    answers.get(row, 3),
                )
                else {
                    panic!("typed float aggregate answers")
                };
                (group, count, sum.to_bits(), mean.to_bits())
            })
            .collect();
        rows.sort_unstable();
        rows
    };
    assert_eq!(render(&expected), render(&got));
}
