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
                    rules: vec![0],
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
                            }
                            .into(),
                        ),
                        (
                            VarId(1),
                            2,
                            ValueType::Interval {
                                element: IntervalElement::U64,
                            }
                            .into(),
                        ),
                    ],
                }),
            )
        })
        .collect();
    let mut sink = ComputedSink::new(EitherSink::Projection(projection), programs, 4, total, &[]);
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
fn products_survive_projection_deduplication_spill_and_reuse() {
    for project_one in [false, true] {
        let mut baseline = None;
        for spill in [false, true] {
            let mut sink = fixture(10, project_one);
            for _ in 0..3 {
                sink.reset();
                projection(&mut sink).begin(Some(WorkContext::new()));
                if spill {
                    projection(&mut sink).force_spill().unwrap();
                }
                sink.row();
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
        sink.row();
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
        sink.row();
        assert!(!sink.flow_after_row().is_terminal());
        assert_eq!(rows(&mut sink).len(), if project_one { 2 } else { 1024 });
    }
}

#[test]
fn batch_stops_after_product_cancellation_before_evaluating_later_scalars() {
    use crate::ScalarExpr as E;

    let mut sink = fixture(10, false);
    sink.bindings.resize(25);
    sink.programs.push((
        24,
        Arc::new(OutputProgram {
            find: 10,
            rules: vec![0],
            expression: FindTerm::Compute(E::Divide(
                Box::new(E::Literal(Value::U64(1))),
                Box::new(E::Var(VarId(2))),
            )),
            inputs: vec![(VarId(2), 0, ValueType::U64.into())],
        }),
    ));
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

fn event_fixture(
    expression: crate::EventExpr,
) -> (
    ComputedSink,
    crate::event::Space,
    crate::work::GenerationHandle,
) {
    let source = crate::event::Space::new(crate::event::SpaceId([31; 32]), 2, &()).unwrap();
    let generation = crate::image::test_generation();
    let program = Arc::new(OutputProgram {
        find: 0,
        rules: vec![0],
        expression: FindTerm::Event(expression),
        inputs: vec![
            (VarId(0), 0, ValueType::Event.into()),
            (VarId(1), 2, ValueType::Event.into()),
        ],
    });
    let projection = ProjectionSink::new(vec![4, 5]);
    let mut sink = ComputedSink::new(
        EitherSink::Projection(projection),
        vec![(4, program)],
        4,
        6,
        &[],
    );
    sink.work = Some(WorkContext::new());
    sink.bind_events(&generation, None);
    (sink, source, generation)
}

fn set_event(
    sink: &mut ComputedSink,
    generation: &crate::work::GenerationHandle,
    slot: usize,
    value: &crate::Event,
) {
    let value = generation
        .lock_resolver()
        .events
        .intern(value, &())
        .unwrap();
    let words = value.key().words();
    sink.bindings.set(slot, words[0]);
    sink.bindings.set(slot + 1, words[1]);
}

#[test]
fn constructed_events_survive_spill_and_empty_values_do_not_eliminate_bindings() {
    use crate::{EventExpr as E, event::BoolOp4};
    let (mut sink, source, generation) = event_fixture(E::Apply {
        op: BoolOp4::AND,
        left: Box::new(E::Var(VarId(0))),
        right: Box::new(E::Not(Box::new(E::Var(VarId(1))))),
    });
    for spill in [false, true] {
        sink.reset();
        projection(&mut sink).begin(Some(WorkContext::new()));
        if spill {
            projection(&mut sink).force_spill().unwrap();
        }
        for bits in 0..16 {
            let a = source.table(3, &[bits], &()).unwrap();
            set_event(&mut sink, &generation, 0, &a);
            set_event(&mut sink, &generation, 2, &source.empty());
            sink.row();
        }
        sink.finish_events().unwrap();
        let rows = rows(&mut sink);
        assert_eq!(rows.len(), 16);
        let mut masks = Vec::new();
        for row in rows {
            let event = generation
                .lock_resolver()
                .events
                .resolve([row[0], row[1]], &())
                .unwrap();
            masks.push(
                (0..4)
                    .filter(|&w| event.contains(w).unwrap())
                    .fold(0u64, |bits, w| bits | (1 << w)),
            );
        }
        masks.sort_unstable();
        assert_eq!(masks, (0..16).collect::<Vec<_>>());
    }
}

#[test]
fn cancellation_during_fault_collection_is_not_reported_as_a_complete_set() {
    use crate::{EventExpr as E, event::BoolOp4};
    let (mut sink, source, generation) = event_fixture(E::Apply {
        op: BoolOp4::TRUE,
        left: Box::new(E::Var(VarId(0))),
        right: Box::new(E::Var(VarId(1))),
    });
    let foreign = crate::event::Space::new(crate::event::SpaceId([32; 32]), 2, &()).unwrap();
    set_event(&mut sink, &generation, 0, &source.full());
    set_event(&mut sink, &generation, 2, &foreign.empty());
    sink.row();
    assert!(!sink.faults.is_empty());
    assert!(
        !sink.flow_after_row().is_terminal(),
        "semantic faults must keep collecting"
    );
    sink.work.as_ref().unwrap().cancel();
    sink.row();
    assert!(sink.flow_after_row().is_terminal());
    assert!(!matches!(
        sink.finish_events(),
        Ok(()) | Err(Error::EventFaults(_))
    ));
    sink.reset();
    sink.work = Some(WorkContext::new());
    set_event(&mut sink, &generation, 2, &source.empty());
    sink.row();
    sink.finish_events().unwrap();
    assert_eq!(rows(&mut sink).len(), 1);
}

#[test]
fn numerical_sink_canonicalizes_before_spilled_projection_and_shares_contraction_work() {
    use crate::event::{
        ArithmeticBudget, ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational,
        LawLimits, Space, SpaceId,
    };
    let control = WorkContext::new();
    let make_sink = || {
        let program = Arc::new(OutputProgram {
            find: 0,
            rules: vec![0],
            expression: FindTerm::Number(crate::NumberExpr::Integer(VarId(0))),
            inputs: vec![(VarId(0), 0, ValueType::U64.into())],
        });
        let mut sink = ComputedSink::new(
            EitherSink::Projection(ProjectionSink::new(vec![1])),
            vec![(1, program)],
            1,
            2,
            &[],
        );
        sink.work = Some(control.clone());
        projection(&mut sink).begin(Some(control.clone()));
        sink.bindings.set(0, 7);
        sink
    };
    let mut reference = make_sink();
    reference.row();
    reference.finish_events().unwrap();
    let number_steps = reference.arithmetic.operations();
    assert!(number_steps > 0);
    let raw = Space::new(SpaceId([222; 32]), 0, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.full(),
                density: ExactRational::one(),
            }],
            LawLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
        .unwrap();
    let mut observed = ExactArithmetic::new(ArithmeticLimits::default(), &());
    crate::ProbabilityAnswer::new(source.full(), source.full(), &mut observed).unwrap();
    let contraction_steps = observed.operations();
    assert!(contraction_steps > 0);
    for spill in [false, true] {
        let mut sink = make_sink();
        if spill {
            projection(&mut sink).force_spill().unwrap();
        }
        for _ in 0..3 {
            sink.row();
        }
        sink.finish_events().unwrap();
        assert_eq!(rows(&mut sink), vec![vec![0]]);
        assert_eq!(sink.observations.checkpoint(), (0, 0, 1, 0));
        let mut owners = super::super::observations::ObservationRegistry::default();
        let mut budget = ArithmeticBudget::new(ArithmeticLimits {
            operations: number_steps + contraction_steps - 1,
            ..ArithmeticLimits::default()
        });
        let mut sink = EitherSink::Computed(Box::new(make_sink()));
        sink.swap_numbers(&mut owners, &mut budget);
        let EitherSink::Computed(computed) = &mut sink else {
            unreachable!()
        };
        computed.row();
        computed.finish_events().unwrap();
        sink.swap_numbers(&mut owners, &mut budget);
        assert_eq!(budget.operations(), number_steps);
        assert_eq!(owners.checkpoint(), (0, 0, 1, 0));
        let result = crate::ProbabilityAnswer::new(
            source.full(),
            source.full(),
            &mut ExactArithmetic::borrow(&mut budget, &control),
        );
        assert!(matches!(
            result,
            Err(crate::Error::Event(crate::event::Error::Capacity(
                crate::event::Capacity::ArithmeticSteps
            )))
        ));
    }
}

#[test]
fn predicate_sink_canonicalizes_before_spilled_projection_and_shares_contraction_work() {
    use crate::event::{
        ArithmeticBudget, ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational,
        LawLimits, Space, SpaceId,
    };
    let control = WorkContext::new();
    let make_sink = || {
        let program = Arc::new(OutputProgram {
            find: 0,
            rules: vec![0],
            expression: FindTerm::Predicate(crate::PredicateExpr::Sign {
                number: crate::NumberExpr::Integer(VarId(0)),
                signs: crate::event::PolynomialSigns::POSITIVE,
            }),
            inputs: vec![(VarId(0), 0, ValueType::U64.into())],
        });
        let mut sink = ComputedSink::new(
            EitherSink::Projection(ProjectionSink::new(vec![1])),
            vec![(1, program)],
            1,
            2,
            &[],
        );
        sink.work = Some(control.clone());
        projection(&mut sink).begin(Some(control.clone()));
        sink.bindings.set(0, 7);
        sink
    };
    let mut reference = make_sink();
    reference.row();
    reference.finish_events().unwrap();
    let number_steps = reference.arithmetic.operations();
    assert!(number_steps > 0);
    let raw = Space::new(SpaceId([222; 32]), 0, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.full(),
                density: ExactRational::one(),
            }],
            LawLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
        .unwrap();
    let mut observed = ExactArithmetic::new(ArithmeticLimits::default(), &());
    crate::ProbabilityAnswer::new(source.full(), source.full(), &mut observed).unwrap();
    let contraction_steps = observed.operations();
    assert!(contraction_steps > 0);
    for spill in [false, true] {
        let mut sink = make_sink();
        if spill {
            projection(&mut sink).force_spill().unwrap();
        }
        for _ in 0..3 {
            sink.row();
        }
        sink.finish_events().unwrap();
        assert_eq!(rows(&mut sink), vec![vec![0]]);
        assert_eq!(sink.observations.checkpoint(), (0, 0, 0, 1));
        let mut owners = super::super::observations::ObservationRegistry::default();
        let mut budget = ArithmeticBudget::new(ArithmeticLimits {
            operations: number_steps + contraction_steps - 1,
            ..ArithmeticLimits::default()
        });
        let mut sink = EitherSink::Computed(Box::new(make_sink()));
        sink.swap_numbers(&mut owners, &mut budget);
        let EitherSink::Computed(computed) = &mut sink else {
            unreachable!()
        };
        computed.row();
        computed.finish_events().unwrap();
        sink.swap_numbers(&mut owners, &mut budget);
        assert_eq!(budget.operations(), number_steps);
        assert_eq!(owners.checkpoint(), (0, 0, 0, 1));
        let result = crate::ProbabilityAnswer::new(
            source.full(),
            source.full(),
            &mut ExactArithmetic::borrow(&mut budget, &control),
        );
        assert!(matches!(
            result,
            Err(crate::Error::Event(crate::event::Error::Capacity(
                crate::event::Capacity::ArithmeticSteps
            )))
        ));
    }
}

#[test]
fn guard_sink_retains_empty_events_through_spill_and_shares_execution_work() {
    use crate::event::{ArithmeticBudget, ArithmeticLimits, ExactArithmetic, PolynomialSigns};
    let (mut sink, source, generation) = event_fixture(crate::EventExpr::Full(VarId(0)));
    let limits = crate::ObservationNumberCodecLimits::default();
    let plan = crate::PredicateGuardPlan::capture(
        &source,
        None,
        limits,
        &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
    )
    .unwrap();
    sink.programs[0].1 = Arc::new(OutputProgram {
        find: 0,
        rules: vec![0],
        inputs: vec![(VarId(0), 0, ValueType::U64.into())],
        expression: FindTerm::Guard(crate::GuardExpr {
            plan,
            companions: vec![crate::PredicateExpr::Sign {
                number: crate::NumberExpr::Integer(VarId(0)),
                signs: PolynomialSigns::POSITIVE,
            }],
            predicate: crate::PredicateExpr::Sign {
                number: crate::NumberExpr::Integer(VarId(0)),
                signs: PolynomialSigns::POSITIVE,
            },
            operation: crate::GuardOp::Holds,
        }),
    });
    let mut steps = 0;
    for spill in [false, true] {
        sink.reset();
        sink.arithmetic = ArithmeticBudget::new(ArithmeticLimits::default());
        projection(&mut sink).begin(Some(WorkContext::new()));
        if spill {
            projection(&mut sink).force_spill().unwrap();
        }
        for value in [0, 1, 1] {
            sink.bindings.set(0, value);
            sink.row();
        }
        sink.finish_events().unwrap();
        let answers = rows(&mut sink);
        assert_eq!(
            answers.len(),
            2,
            "empty guard is a value, not an absent row"
        );
        let events: Vec<_> = answers
            .iter()
            .map(|row| {
                generation
                    .lock_resolver()
                    .events
                    .resolve([row[0], row[1]], &())
                    .unwrap()
            })
            .collect();
        assert!(events.iter().any(crate::Event::is_empty));
        assert!(events.iter().any(crate::Event::is_full));
        steps = sink.arithmetic.operations();
    }
    assert!(steps > 0);
    sink.reset();
    projection(&mut sink).begin(Some(WorkContext::new()));
    sink.arithmetic = ArithmeticBudget::new(ArithmeticLimits {
        operations: steps - 1,
        ..ArithmeticLimits::default()
    });
    for value in [0, 1, 1] {
        sink.bindings.set(0, value);
        sink.row();
    }
    assert!(matches!(
        sink.finish_events(),
        Err(Error::Event(crate::event::Error::Capacity(
            crate::event::Capacity::ArithmeticSteps
        )))
    ));
    sink.reset();
    sink.arithmetic = ArithmeticBudget::new(ArithmeticLimits::default());
    sink.work.as_ref().unwrap().cancel();
    sink.row();
    assert!(sink.finish_events().is_err());
    sink.reset();
    sink.work = Some(WorkContext::new());
    let control = sink.work.clone();
    projection(&mut sink).begin(control);
    sink.row();
    sink.finish_events().unwrap();
    assert_eq!(rows(&mut sink).len(), 1);
}
