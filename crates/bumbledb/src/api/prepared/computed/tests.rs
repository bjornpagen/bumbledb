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
            expression: FindTerm::Compute(E::Divide(
                Box::new(E::Literal(Value::U64(1))),
                Box::new(E::Var(VarId(2))),
            )),
            inputs: vec![(VarId(2), 0, ValueType::U64)],
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
