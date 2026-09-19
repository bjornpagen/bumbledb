#![allow(clippy::too_many_lines)]
use bumbledb::{
    AnswerValue, BindValue, Db, Error, EventOperandSource, ExpectationFunction, ExpectationPayoff,
    ExpectationValue, ImportedPayoff, PayoffImport,
    event::{
        ArithmeticLimits, BoolOp4, DensityPiece, ExactArithmetic, ExactPolynomial, ExactRational,
        FamilyFunction, FiniteFunction, FunctionLimits, FunctionPiece, GuardedRationalFunction,
        LawLimits, ParameterDensityPiece, ParameterDomain, ParameterGuard, ParameterId,
        ParameterRegion, ParameterSourceLimits, PolynomialSigns, SourceDescriptorLimits, Space,
        SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub LocalPayoffs;
    relation Patch { id: u64, group: u64, expression: u64, value: i64, when: event, given: event }
    Patch(id) -> Patch;
}
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn import(value: ImportedPayoff) -> PayoffImport {
    let captured =
        PayoffImport::capture(value, SourceDescriptorLimits::default(), &mut work()).unwrap();
    PayoffImport::from_bytes(
        captured.bytes(),
        SourceDescriptorLimits::default(),
        &mut work(),
    )
    .unwrap()
}
fn finite(source: &Space, values: [i64; 2]) -> FiniteFunction {
    FiniteFunction::new(
        source,
        &values
            .into_iter()
            .enumerate()
            .map(|(i, n)| FunctionPiece {
                region: source.table(1, &[1 << i], &()).unwrap(),
                value: q(n, 1),
            })
            .collect::<Vec<_>>(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap()
}
fn fixed(answer: AnswerValue<'_>) -> Option<ExactRational> {
    let AnswerValue::Expectation(answer) = answer else {
        panic!("expectation")
    };
    let ExpectationValue::Fixed { value, .. } = answer.value() else {
        panic!("fixed")
    };
    value.clone()
}

#[test]
fn imported_functions_match_all_two_world_covers_on_both_join_paths() {
    let path = common::TempDir::new("query-function-oracle");
    let db = Db::create(path.path(), LocalPayoffs, common::work())
        .unwrap()
        .unwrap();
    let raw = Space::new(SpaceId([238; 32]), 1, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.table(1, &[1], &()).unwrap(),
                density: q(1, 1),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let a = import(ImportedPayoff::Finite(finite(&source, [0, 2])));
    let b = import(ImportedPayoff::Finite(finite(&source, [0, 3])));
    db.write(common::work(), |tx| {
        for parent in 0..4 {
            for left in 0..4 {
                for right in 0..4 {
                    let group = parent * 16 + left * 4 + right;
                    for (expression, region) in [(0, left), (1, right)] {
                        tx.insert([&Patch {
                            id: group * 2 + expression,
                            group,
                            expression,
                            value: 0,
                            when: source.table(1, &[region], &())?,
                            given: source.table(1, &[parent], &())?,
                        }])?;
                    }
                }
            }
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);
    let db = Db::open(path.path(), LocalPayoffs, common::work()).unwrap();
    let template = query!(LocalPayoffs {
        use payoff left = &a;
        use payoff right = &b;
        (mean: Expectation(Payoff(left), when, given)) | Patch(group, expression, when, given), group == ?case, expression == 0;
        (mean: Expectation(Payoff(right), when, given)) | Patch(group, expression, when, given), group == ?case, expression == 1;
    });
    let mut retained = Vec::new();
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        for parent in 0..4 {
            for left in 0..4 {
                for right in 0..4 {
                    let group = parent * 16 + left * 4 + right;
                    let result = db.read(common::work(), |snapshot| {
                        snapshot.execute_collect(&mut prepared, &[BindValue::U64(group)])
                    });
                    let covered = (left | right) & parent == parent;
                    let agrees = left & right & parent & 2 == 0;
                    assert_eq!(
                        result.is_ok(),
                        covered && agrees,
                        "parent {parent} left {left} right {right}"
                    );
                    if let Ok(answers) = result {
                        assert_eq!(answers.len(), 1);
                        assert_eq!(
                            fixed(answers.get(0, 0)),
                            if parent & 1 == 0 { None } else { Some(q(0, 1)) }
                        );
                        let AnswerValue::Expectation(answer) = answers.get(0, 0) else {
                            panic!("expectation")
                        };
                        let ExpectationPayoff::Finite(cover) = answer.payoff() else {
                            panic!("finite cover")
                        };
                        assert_eq!(cover.patches().len(), 2);
                        assert!(answer.partition().is_none());
                        for world in 0..2 {
                            let expected = if parent & (1 << world) == 0 || world == 0 {
                                0
                            } else if left & 2 != 0 {
                                2
                            } else {
                                3
                            };
                            assert_eq!(cover.function().at(world, &()).unwrap(), q(expected, 1));
                        }
                        retained.push(answers);
                    }
                }
            }
        }
    }
    drop((db, template, a, b, source));
    assert!(!retained.is_empty());
    for answers in retained {
        let AnswerValue::Expectation(answer) = answers.get(0, 0) else {
            panic!("expectation")
        };
        let ExpectationFunction::Finite(function) = answer.function() else {
            panic!("finite")
        };
        assert_eq!(function.at(0, &()).unwrap(), q(0, 1));
    }
}

#[test]
fn arbitrary_precision_literals_merge_with_column_values_and_survive_owner_release() {
    let path = common::TempDir::new("query-exact-literals");
    let db = Db::create(path.path(), LocalPayoffs, common::work())
        .unwrap()
        .unwrap();
    let raw = Space::new(SpaceId([239; 32]), 0, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.full(),
                density: q(1, 1),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let big = ExactRational::fraction(
        "1234567890123456789012345678901234567890123456789",
        "7",
        &mut work(),
    )
    .unwrap();
    let huge = import(ImportedPayoff::Rational(big.clone()));
    let small = import(ImportedPayoff::Rational(q(4, 1)));
    db.write(common::work(), |tx| {
        tx.insert([&Patch {
            id: 0,
            group: 0,
            expression: 0,
            value: 4,
            when: source.full(),
            given: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(LocalPayoffs {
        use payoff huge = &huge;
        use payoff small = &small;
        (big: Expectation(Payoff(huge), when, given), four: Expectation(Payoff(small), when, given)) | Patch(when, given);
        (big: Expectation(Payoff(huge), when, given), four: Expectation(value, when, given)) | Patch(value, when, given);
    });
    let mut retained = Vec::new();
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, template, huge, small, source));
    for answers in retained {
        assert_eq!(answers.len(), 1);
        assert_eq!(fixed(answers.get(0, 0)), Some(big.clone()));
        assert_eq!(fixed(answers.get(0, 1)), Some(q(4, 1)));
        let AnswerValue::Expectation(answer) = answers.get(0, 1) else {
            panic!("expectation")
        };
        assert_eq!(answer.values().unwrap(), [q(4, 1)]);
    }
}

fn parameter_fixture() -> (Space, FamilyFunction, FamilyFunction) {
    let limits = ParameterSourceLimits::default();
    let name = ParameterId([240; 32]);
    let p = ExactPolynomial::parameter(name);
    let one = ExactPolynomial::constant(q(1, 1));
    let tail = one
        .sub(&p, limits.parameters.region.polynomial, &mut work())
        .unwrap();
    let sign = |poly: &ExactPolynomial, signs| {
        ParameterRegion::from_polynomial(name, poly, signs, limits.parameters.region, &mut work())
            .unwrap()
    };
    let domain = ParameterDomain::new(
        sign(&p, PolynomialSigns::NON_NEGATIVE)
            .apply(
                BoolOp4::AND,
                &sign(&tail, PolynomialSigns::NON_NEGATIVE),
                limits.parameters.region,
                &mut work(),
            )
            .unwrap(),
    )
    .unwrap();
    let threshold = p
        .sub(
            &ExactPolynomial::constant(q(1, 2)),
            limits.parameters.region.polynomial,
            &mut work(),
        )
        .unwrap();
    let raw = Space::new(SpaceId([241; 32]), 3, &())
        .unwrap()
        .with_parameters(
            domain.clone(),
            &[
                ParameterGuard {
                    coordinate: 0,
                    region: sign(&threshold, PolynomialSigns::NON_POSITIVE),
                },
                ParameterGuard {
                    coordinate: 1,
                    region: sign(&threshold, PolynomialSigns::NON_NEGATIVE),
                },
                ParameterGuard {
                    coordinate: 2,
                    region: sign(&p, PolynomialSigns::ZERO),
                },
            ],
            limits,
            &mut work(),
        )
        .unwrap();
    let rational = |polynomial| {
        GuardedRationalFunction::new(
            domain.clone(),
            polynomial,
            one.clone(),
            limits.parameters.region,
            &mut work(),
        )
        .unwrap()
    };
    let source = raw
        .with_parameter_density(
            &[ParameterDensityPiece {
                region: raw.full(),
                density: rational(one.clone()),
            }],
            limits,
            &mut work(),
        )
        .unwrap();
    let rising = FamilyFunction::constant(&source, rational(p), limits, &mut work()).unwrap();
    let falling = FamilyFunction::constant(&source, rational(tail), limits, &mut work()).unwrap();
    (source, rising, falling)
}

#[test]
fn family_queries_glue_at_an_exact_boundary_and_promote_finite_and_scalar_patches() {
    let (source, rising, falling) = parameter_fixture();
    let a = import(ImportedPayoff::Family(rising));
    let b = import(ImportedPayoff::Family(falling));
    let zero = import(ImportedPayoff::Finite(
        FiniteFunction::constant(&source, q(0, 1), FunctionLimits::default(), &mut work()).unwrap(),
    ));
    let half = import(ImportedPayoff::Rational(q(1, 2)));
    let left = source.coordinate(0, &()).unwrap();
    let right = source.coordinate(1, &()).unwrap();
    let path = common::TempDir::new("query-family-patches");
    let db = Db::create(path.path(), LocalPayoffs, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        for (expression, when) in [
            (0, left.clone()),
            (1, right.clone()),
            (2, source.coordinate(2, &())?),
            (3, left.apply(BoolOp4::AND, &right, &())?),
        ] {
            tx.insert([&Patch {
                id: expression,
                group: 0,
                expression,
                value: 0,
                when,
                given: source.full(),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(LocalPayoffs {
        use payoff a = &a; use payoff b = &b; use payoff zero = &zero; use payoff half = &half;
        (mean: Expectation(Payoff(a), when, given)) | Patch(expression, when, given), expression == 0;
        (mean: Expectation(Payoff(b), when, given)) | Patch(expression, when, given), expression == 1;
        (mean: Expectation(Payoff(zero), when, given)) | Patch(expression, when, given), expression == 2;
        (mean: Expectation(Payoff(half), when, given)) | Patch(expression, when, given), expression == 3;
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        let AnswerValue::Expectation(answer) = answers.get(0, 0) else {
            panic!("expectation")
        };
        let ExpectationPayoff::Family(cover) = answer.payoff() else {
            panic!("family cover")
        };
        assert_eq!(cover.patches().len(), 4);
        let ExpectationValue::Family(observed) = answer.value() else {
            panic!("family observation")
        };
        let limits = ParameterSourceLimits::default();
        for (n, expected) in [(0, 0), (1, 1), (2, 2), (3, 1), (4, 0)] {
            assert_eq!(
                observed
                    .conditional()
                    .value_at(
                        &q(n, 4),
                        limits.parameters.region,
                        limits.functions,
                        &mut work()
                    )
                    .unwrap(),
                Some(q(expected, 4))
            );
        }
    }
    // An isolated parameter point has no prior mass, but is still a real world.
    let bad = import(ImportedPayoff::Rational(q(7, 1)));
    let conflict = query!(LocalPayoffs {
        use payoff a = &a; use payoff b = &b; use payoff bad = &bad;
        (mean: Expectation(Payoff(a), when, given)) | Patch(expression, when, given), expression == 0;
        (mean: Expectation(Payoff(b), when, given)) | Patch(expression, when, given), expression == 1;
        (mean: Expectation(Payoff(bad), when, given)) | Patch(expression, when, given), expression == 3;
    });
    let mut prepared = db.prepare(&conflict, common::work()).unwrap();
    assert!(matches!(
        db.read(common::work(), |snapshot| snapshot
            .execute_collect(&mut prepared, &[] as &[BindValue])),
        Err(Error::Event(bumbledb::event::Error::FunctionCoverConflict))
    ));
}

#[test]
fn foreign_empty_function_imports_have_honest_complete_operand_diagnostics() {
    let path = common::TempDir::new("query-payoff-faults");
    let db = Db::create(path.path(), LocalPayoffs, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([1; 32]), 0, &()).unwrap();
    let foreign = Space::new(SpaceId([252; 32]), 0, &()).unwrap();
    let foreign = import(ImportedPayoff::Finite(
        FiniteFunction::constant(&foreign, q(0, 1), FunctionLimits::default(), &mut work())
            .unwrap(),
    ));
    db.write(common::work(), |tx| {
        tx.insert([&Patch {
            id: 0,
            group: 0,
            expression: 0,
            value: 0,
            when: source.empty(),
            given: source.empty(),
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(LocalPayoffs {
        use payoff foreign = &foreign;
        (mean: Expectation(Payoff(foreign), when, given)) | Patch(when, given);
        (mean: Expectation(Payoff(foreign), when, given)) | Patch(when, given);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let error = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err();
        assert!(error.to_string().contains("payoff import"));
        let Error::EventFaults(faults) = error else {
            panic!("context faults")
        };
        assert_eq!(faults.len(), 2);
        for (i, fault) in faults.iter().enumerate() {
            assert_eq!(usize::from(fault.rule), i);
            assert_eq!(fault.operand, 2);
            assert_eq!(fault.source, EventOperandSource::PayoffImport);
            assert_eq!(&*fault.offending_value, foreign.bytes());
            assert_eq!(&*fault.expected_space, source.full().to_bytes(&()).unwrap());
        }
    }
}
