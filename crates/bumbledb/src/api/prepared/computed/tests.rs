use super::*;
use crate::exec::sink::ProjectionSink;
use crate::schema::IntervalElement;
use crate::{SegmentOp, WorkContext};

fn fixture(producers: usize, project_one: bool) -> ComputedSink {
    let total = 4 + producers * 2;
    let projection = ProjectionSink::new((4..if project_one { 6 } else { total }).collect());
    let programs = (0..producers)
        .map(|find| {
            (
                4 + 2 * find,
                Arc::new(OutputProgram {
                    find,
                    expression: FindTerm::Segments {
                        op: SegmentOp::Difference,
                        left: VarId(0),
                        right: VarId(1),
                    },
                    inputs: vec![
                        (
                            VarId(0),
                            0,
                            ValueType::Interval {
                                element: IntervalElement::U64,
                            },
                        ),
                        (
                            VarId(1),
                            2,
                            ValueType::Interval {
                                element: IntervalElement::U64,
                            },
                        ),
                    ],
                }),
            )
        })
        .collect();
    let mut sink = ComputedSink::new(EitherSink::Projection(projection), programs, 4, total);
    sink.bindings.set(0, 0);
    sink.bindings.set(1, 10);
    sink.bindings.set(2, 3);
    sink.bindings.set(3, 7);
    sink
}

fn projection(sink: &mut ComputedSink) -> &mut ProjectionSink {
    let EitherSink::Projection(projection) = &mut sink.inner else {
        unreachable!()
    };
    projection
}

fn rows(sink: &mut ComputedSink) -> Vec<Vec<u64>> {
    let mut rows = Vec::new();
    projection(sink)
        .for_each_answer(&mut |row| {
            rows.push(row.to_vec());
            Ok(())
        })
        .unwrap();
    rows.sort();
    rows
}

#[test]
fn products_survive_projection_deduplication_and_reuse() {
    for project_one in [false, true] {
        let mut sink = fixture(10, project_one);
        let mut baseline = None;
        for _ in 0..3 {
            sink.reset();
            projection(&mut sink).begin(Some(WorkContext::new()));
            sink.row(0);
            assert!(!sink.flow_after_row().is_terminal());
            let result = rows(&mut sink);
            assert_eq!(result.len(), if project_one { 2 } else { 1024 });
            for row in &result {
                assert!(
                    row.as_chunks::<2>()
                        .0
                        .iter()
                        .all(|p| *p == [0, 3] || *p == [7, 10])
                );
            }
            if let Some(expected) = &baseline {
                assert_eq!(&result, expected);
            } else {
                baseline = Some(result);
            }
        }
        sink.release_memory();
    }
}

#[test]
fn product_cancellation_polls_even_when_projection_deduplicates_every_later_row() {
    for project_one in [false, true] {
        let mut sink = fixture(10, project_one);
        let work = WorkContext::new();
        projection(&mut sink).begin(Some(work.clone()));
        // Enter the kernel directly: its own 256-output polling quantum must
        // stop the product, including duplicate projections, without an outer join poll.
        work.cancel();
        sink.row(0);
        assert!(sink.flow_after_row().is_terminal());
        assert_eq!(
            projection(&mut sink).answers().count(),
            if project_one { 1 } else { 255 }
        );
        assert!(
            projection(&mut sink)
                .for_each_answer(&mut |_| panic!("failed output must not drain"))
                .is_err()
        );
        sink.reset();
        projection(&mut sink).begin(Some(WorkContext::new()));
        sink.row(0);
        assert!(!sink.flow_after_row().is_terminal());
        assert_eq!(rows(&mut sink).len(), if project_one { 2 } else { 1024 });
    }
}

#[test]
fn batch_stops_after_product_cancellation_before_evaluating_later_scalars() {
    use crate::ScalarExpr as E;

    let mut sink = fixture(10, false);
    sink.bindings.resize(25);
    let mut programs: Vec<_> = sink
        .outputs
        .iter()
        .map(|output| (output.slot, Arc::clone(&output.program)))
        .collect();
    programs.push((
        24,
        Arc::new(OutputProgram {
            find: 10,
            expression: FindTerm::Compute(E::Divide(
                Box::new(E::Literal(Value::U64(1))),
                Box::new(E::Var(VarId(2))),
            )),
            inputs: vec![(VarId(2), 0, ValueType::U64)],
        }),
    ));
    sink.install(programs);
    let work = WorkContext::new();
    projection(&mut sink).begin(Some(work.clone()));
    let mut outer = Bindings::new(4);
    outer.set(1, 10);
    outer.set(2, 3);
    outer.set(3, 7);
    let batch = LeafBatch {
        keys: &[1, 0],
        arity: 1,
        survivors: &[0, 1],
        key_slots: &[0],
        bindings: &outer,
    };
    work.cancel();
    assert!(sink.emit_batch(&batch).is_terminal());
    assert!(
        sink.error.is_none(),
        "the later division by zero must not replace the latched cancellation: {:?}",
        sink.error
    );
    assert!(
        projection(&mut sink)
            .for_each_answer(&mut |_| panic!("cancelled output must not drain"))
            .is_err()
    );
}

mod differential {
    //! Compiled programs against the recursive evaluator: random typed
    //! expression trees over random bindings, every lane, every SIMD level.
    use super::super::program::{Errors, LANES, Program, Register};
    use crate::exec::kernel::numeric::DefaultFloatEnvironment;
    use crate::scalar::{NumericCast, Rounding, ScalarError, ScalarExpr as E, evaluate_binding};
    use crate::schema::{IntervalElement, ValueType};
    use crate::{F64, Interval, Value, VarId};

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 ^ (self.0 >> 29)
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum T {
        U64,
        I64,
        F64,
        Bool,
    }

    /// Variable `i` reads columns starting at `COLUMNS[i]`.
    const TYPES: [ValueType; 7] = [
        ValueType::U64,
        ValueType::I64,
        ValueType::F64,
        ValueType::Interval {
            element: IntervalElement::U64,
        },
        ValueType::Interval {
            element: IntervalElement::I64,
        },
        ValueType::Interval {
            element: IntervalElement::F64,
        },
        ValueType::Bool,
    ];
    const COLUMNS: [usize; 7] = [0, 1, 2, 3, 5, 7, 9];

    fn float_pool(rng: &mut Rng) -> F64 {
        const POOL: [f64; 14] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            3.0,
            1e308,
            -1e308,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            5e-324,
            9_007_199_254_740_992.0,
            -4_611_686_018_427_387_904.0,
        ];
        if rng.below(4) == 0 {
            F64::from_bits(rng.next())
        } else {
            F64::from(POOL[rng.below(14) as usize])
        }
    }

    fn u64_pool(rng: &mut Rng) -> u64 {
        match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => u64::MAX,
            3 => rng.below(10),
            4 => 1 << rng.below(64),
            _ => rng.next(),
        }
    }

    fn i64_pool(rng: &mut Rng) -> i64 {
        match rng.below(6) {
            0 => 0,
            1 => -1,
            2 => i64::MIN,
            3 => i64::MAX,
            4 => rng.below(21).cast_signed() - 10,
            _ => rng.next().cast_signed(),
        }
    }

    fn literal(rng: &mut Rng, t: T) -> E {
        E::Literal(match t {
            T::U64 => Value::U64(u64_pool(rng)),
            T::I64 => Value::I64(i64_pool(rng)),
            T::F64 => Value::F64(float_pool(rng)),
            T::Bool => Value::Bool(rng.below(2) == 0),
        })
    }

    fn var(t: T) -> E {
        E::Var(VarId(match t {
            T::U64 => 0,
            T::I64 => 1,
            T::F64 => 2,
            T::Bool => 6,
        }))
    }

    fn rounding(rng: &mut Rng) -> Rounding {
        [
            Rounding::TowardZero,
            Rounding::NearestTiesAwayFromZero,
            Rounding::NearestTiesToEven,
        ][rng.below(3) as usize]
    }

    fn arithmetic(rng: &mut Rng, a: E, b: E) -> E {
        let (a, b) = (Box::new(a), Box::new(b));
        match rng.below(4) {
            0 => E::Add(a, b),
            1 => E::Subtract(a, b),
            2 => E::Multiply(a, b),
            _ => E::Divide(a, b),
        }
    }

    fn any_numeric(rng: &mut Rng) -> T {
        [T::U64, T::I64, T::F64][rng.below(3) as usize]
    }

    fn generate(rng: &mut Rng, t: T, depth: u32) -> E {
        if depth == 0 || rng.below(4) == 0 {
            return if rng.below(2) == 0 {
                var(t)
            } else {
                literal(rng, t)
            };
        }
        let d = depth - 1;
        let cast = |rng: &mut Rng, kind| E::Cast {
            kind,
            expr: Box::new({
                let from = any_numeric(rng);
                generate(rng, from, d)
            }),
        };
        match (t, rng.below(5)) {
            (T::Bool, 0 | 1) => E::IsNaN(Box::new(generate(rng, T::F64, d))),
            (T::Bool, _) => E::IsFinite(Box::new(generate(rng, T::F64, d))),
            (T::U64, 0) => E::Measure(Box::new(E::Var(VarId(3 + u16::from(rng.below(2) == 1))))),
            (T::F64, 0) => E::Measure(Box::new(E::Var(VarId(5)))),
            (T::U64, 1) => cast(rng, NumericCast::ToU64Exact),
            (T::I64, 1) => cast(rng, NumericCast::ToI64Exact),
            (T::F64, 1) => {
                let kind = if rng.below(2) == 0 {
                    NumericCast::ToF64
                } else {
                    NumericCast::ToF64Exact
                };
                cast(rng, kind)
            }
            (T::U64 | T::I64, 2) => E::MulDiv {
                a: Box::new(generate(rng, t, d)),
                b: Box::new(generate(rng, t, d)),
                divisor: Box::new(generate(rng, t, d)),
                rounding: rounding(rng),
            },
            (T::I64 | T::F64, 3) => E::Negate(Box::new(generate(rng, t, d))),
            _ => {
                let a = generate(rng, t, d);
                let b = generate(rng, t, d);
                arithmetic(rng, a, b)
            }
        }
    }

    /// Valid binding words for every input column, per lane.
    fn bindings(rng: &mut Rng) -> Vec<Register> {
        let mut columns = vec![[0; LANES]; 10];
        for lane in 0..LANES {
            columns[0][lane] = u64_pool(rng);
            columns[1][lane] = i64_pool(rng).cast_unsigned() ^ (1 << 63);
            columns[2][lane] = float_pool(rng).to_order_key();
            let start = rng.below(1000);
            let end = if rng.below(4) == 0 {
                u64::MAX
            } else {
                start + 1 + rng.below(1000)
            };
            columns[3][lane] = start;
            columns[4][lane] = end;
            let start = rng.below(2000).cast_signed() - 1000;
            let end = if rng.below(4) == 0 {
                i64::MAX
            } else {
                start + 1 + rng.below(1000).cast_signed()
            };
            columns[5][lane] = start.cast_unsigned() ^ (1 << 63);
            columns[6][lane] = end.cast_unsigned() ^ (1 << 63);
            let (start, end) = loop {
                let (a, b) = (float_pool(rng), float_pool(rng));
                if let Some(span) = Interval::new(a.min(b), a.max(b)) {
                    if !a.is_nan() && !b.is_nan() {
                        break (span.start(), span.end());
                    }
                }
            };
            columns[7][lane] = start.to_order_key();
            columns[8][lane] = end.to_order_key();
            columns[9][lane] = rng.below(2);
        }
        columns
    }

    fn decode(columns: &[Register], lane: usize, var: VarId) -> Result<Value, ScalarError> {
        let index = var.0 as usize;
        let ty = *TYPES.get(index).ok_or(ScalarError::UnboundVariable(var))?;
        let word = columns[COLUMNS[index]][lane];
        let float = |key| F64::from_order_key(key).expect("canonical");
        let signed = |word: u64| (word ^ (1 << 63)).cast_signed();
        Ok(match ty {
            ValueType::U64 => Value::U64(word),
            ValueType::I64 => Value::I64(signed(word)),
            ValueType::F64 => Value::F64(float(word)),
            ValueType::Bool => Value::Bool(word != 0),
            _ => {
                let end = columns[COLUMNS[index] + 1][lane];
                match ty.interval_element().expect("interval") {
                    IntervalElement::U64 => Value::IntervalU64(Interval::new(word, end).unwrap()),
                    IntervalElement::I64 => {
                        Value::IntervalI64(Interval::new(signed(word), signed(end)).unwrap())
                    }
                    IntervalElement::F64 => {
                        Value::IntervalF64(Interval::new(float(word), float(end)).unwrap())
                    }
                }
            }
        })
    }

    fn word(value: Value) -> u64 {
        match value {
            Value::U64(v) => v,
            Value::I64(v) => v.cast_unsigned() ^ (1 << 63),
            Value::F64(v) => v.to_order_key(),
            Value::Bool(v) => u64::from(v),
            other => panic!("scalar output {other:?}"),
        }
    }

    fn check(expression: &E, columns: &[Register], float: Option<DefaultFloatEnvironment>) {
        let program = Program::compile(expression, |var| {
            let index = var.0 as usize;
            TYPES.get(index).map(|ty| (COLUMNS[index], *ty))
        });
        let expected: Vec<_> = (0..LANES)
            .map(|lane| {
                evaluate_binding(expression, float, |var| decode(columns, lane, var)).map(word)
            })
            .collect();
        for level in crate::exec::kernel::every_level() {
            let mut stack = vec![[0; LANES]; program.depth()];
            let mut out = [0; LANES];
            let mut errors = Errors::new();
            fearless_simd::dispatch!(level, simd => program.run(
                simd,
                LANES,
                columns,
                float,
                &mut stack,
                &mut out,
                &mut errors,
            ));
            for (lane, expected) in expected.iter().enumerate() {
                let got = errors.of(lane).map_or(Ok(out[lane]), Err);
                assert_eq!(&got, expected, "lane {lane} of {expression:?}");
            }
        }
    }

    #[test]
    fn compiled_programs_match_the_recursive_evaluator_at_every_level() {
        let mut rng = Rng(0x00C0_FFEE);
        let float = DefaultFloatEnvironment::check().ok();
        assert!(float.is_some());
        for round in 0..1500 {
            let columns = bindings(&mut rng);
            let t = [T::U64, T::I64, T::F64, T::Bool][round % 4];
            let expression = generate(&mut rng, t, 5);
            check(&expression, &columns, float);
            check(&expression, &columns, None);
        }
    }

    #[test]
    fn negative_zero_canonicalizes_after_every_float_op() {
        let mut rng = Rng(7);
        let columns = bindings(&mut rng);
        let literal = |x: f64| Box::new(E::Literal(Value::F64(F64::from(x))));
        // 1 / (0 * -1) is +Inf only if the product canonicalizes to +0.
        let expression = E::Divide(
            literal(1.0),
            Box::new(E::Multiply(literal(0.0), literal(-1.0))),
        );
        check(&expression, &columns, DefaultFloatEnvironment::check().ok());
        let program = Program::compile(&expression, |_| None);
        let mut out = [0; LANES];
        let mut errors = Errors::new();
        let mut stack = vec![[0; LANES]; program.depth()];
        fearless_simd::dispatch!(crate::exec::kernel::level(), simd => program.run(
            simd,
            1,
            &columns,
            DefaultFloatEnvironment::check().ok(),
            &mut stack,
            &mut out,
            &mut errors,
        ));
        assert_eq!(out[0], F64::INFINITY.to_order_key());
    }

    #[test]
    fn untyped_and_unbound_inputs_fail_every_lane_in_program_order() {
        let mut rng = Rng(11);
        let columns = bindings(&mut rng);
        let float = DefaultFloatEnvironment::check().ok();
        for expression in [
            E::Add(Box::new(E::Var(VarId(0))), Box::new(E::Var(VarId(1)))),
            E::Add(Box::new(E::Var(VarId(0))), Box::new(E::Var(VarId(42)))),
            E::Divide(
                Box::new(E::Divide(
                    Box::new(E::Var(VarId(0))),
                    Box::new(E::Literal(Value::U64(0))),
                )),
                Box::new(E::Var(VarId(42))),
            ),
            E::Negate(Box::new(E::Var(VarId(0)))),
            E::IsNaN(Box::new(E::Var(VarId(1)))),
            E::Measure(Box::new(E::Var(VarId(0)))),
            E::Cast {
                kind: NumericCast::ToF64,
                expr: Box::new(E::Var(VarId(6))),
            },
        ] {
            check(&expression, &columns, float);
        }
    }
}
