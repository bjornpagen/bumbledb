use super::*;
use crate::error::FindIndex;
use crate::ir::FoldOp;

#[test]
fn expectation_finalization_preserves_the_initialized_prefix_on_failed_append() {
    use crate::event::{
        ArithmeticLimits, DensityPiece, EventPartition, ExactArithmetic, ExactRational, LawLimits,
        PartitionLimits, Space, SpaceId,
    };
    use crate::exec::run::{Bindings, Sink};
    use crate::ir::validate::SignatureColumn;
    use crate::observation::ExpectationInput;
    let work = crate::WorkContext::new();
    let generation = crate::image::test_generation();
    let interner = crate::image::intern::InternerHandle::new(&generation, &work);
    let raw = Space::new(SpaceId([207; 32]), 0, &work).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.full(),
                density: ExactRational::one(),
            }],
            LawLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
        )
        .unwrap();
    let input = |source: &Space, value: i64| ExpectationInput {
        partition: EventPartition::on(
            &source.full(),
            &[source.full()],
            PartitionLimits::default(),
            &work,
        )
        .unwrap(),
        values: vec![ExactRational::from(value)],
    };
    for spill in [false, true] {
        let append = |inputs: Vec<ExpectationInput>, out: &mut Answers| {
            let mut sink = ProjectionSink::new(vec![0]);
            let mut row = Bindings::new(1);
            for n in 0..inputs.len() {
                row.set(0, n as u64);
                assert!(!sink.emit(&row).is_terminal());
            }
            if spill {
                sink.force_spill().unwrap();
            }
            out.expectation_inputs = inputs;
            super::super::finalize::finalize(
                &mut EitherSink::Projection(sink),
                &mut Vec::new(),
                &mut ResolveMemo::new(),
                &interner,
                &[SignatureColumn::Expectation],
                out,
                &work,
            )
        };
        let mut out = Answers::new();
        out.begin(1);
        append(vec![input(&source, 7)], &mut out).unwrap();
        let AnswerValue::Expectation(prior) = out.get(0, 0) else {
            panic!("expectation");
        };
        let prior = prior.clone();
        assert!(matches!(
            append(vec![input(&source, 9), input(&raw, 0)], &mut out),
            Err(Error::Event(crate::event::Error::MissingLaw))
        ));
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), AnswerValue::Expectation(&prior));
        assert!(out.expectation_inputs.is_empty());
        append(vec![input(&source, -3)], &mut out).unwrap();
        let AnswerValue::Expectation(next) = out.get(1, 0) else {
            panic!("expectation");
        };
        let crate::ExpectationValue::Fixed { value, .. } = next.value() else {
            panic!("fixed");
        };
        assert_eq!(value, &Some(ExactRational::from(-3i64)));
    }
}

#[test]
fn probability_finalization_rolls_back_only_failed_appends_in_ram_and_spill() {
    use crate::event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational, LawLimits, Space, SpaceId,
    };
    use crate::exec::run::{Bindings, Sink};
    use crate::ir::validate::SignatureColumn;
    use crate::work::{GenerationHandle, GenerationState};
    let work = crate::WorkContext::new();
    let generation = GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
    ));
    let interner = crate::image::intern::InternerHandle::new(&generation, &work);
    let raw = Space::new(SpaceId([191; 32]), 1, &work).unwrap();
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &work);
    let half = ExactRational::fraction("1", "2", &mut arithmetic).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.full(),
                density: half.clone(),
            }],
            LawLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
    let full = interner.intern_event(&source.full()).unwrap().key().words();
    let first = interner
        .intern_event(&source.coordinate(0, &work).unwrap())
        .unwrap()
        .key()
        .words();
    let missing = interner.intern_event(&raw.full()).unwrap().key().words();
    let pair = |a: [u64; 2], b: [u64; 2]| [a[0], a[1], b[0], b[1]];
    for spill in [false, true] {
        let append = |rows: &[[u64; 4]], out: &mut Answers| {
            let mut sink = ProjectionSink::new(vec![0, 1, 2, 3]);
            let mut bindings = Bindings::new(4);
            for row in rows {
                for (slot, word) in row.iter().enumerate() {
                    bindings.set(slot, *word);
                }
                assert!(!sink.emit(&bindings).is_terminal());
            }
            if spill {
                sink.force_spill().unwrap();
            }
            super::super::finalize::finalize(
                &mut EitherSink::Projection(sink),
                &mut Vec::new(),
                &mut ResolveMemo::new(),
                &interner,
                &[SignatureColumn::Probability],
                out,
                &work,
            )
        };
        let mut out = Answers::new();
        out.begin(1);
        append(&[pair(full, full)], &mut out).unwrap();
        let AnswerValue::Probability(prior) = out.get(0, 0) else {
            panic!("probability")
        };
        let prior = prior.clone();
        assert!(matches!(
            append(&[pair(first, full), pair(missing, missing)], &mut out),
            Err(Error::Event(crate::event::Error::MissingLaw))
        ));
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), AnswerValue::Probability(&prior));
        append(&[pair(first, full)], &mut out).unwrap();
        assert_eq!(out.len(), 2);
        let AnswerValue::Probability(next) = out.get(1, 0) else {
            panic!("probability")
        };
        let crate::ProbabilityValue::Fixed { value, .. } = next.value() else {
            panic!("fixed")
        };
        assert_eq!(value, &Some(half.clone()));
        assert_ne!(next, &prior);
        // A repeated pair may reuse the old observation pool entry on append.
        append(&[pair(full, full)], &mut out).unwrap();
        assert_eq!(out.get(2, 0), AnswerValue::Probability(&prior));
    }
}

#[test]
fn event_results_resolve_spilled_keys_and_reject_stale_keys() {
    use crate::event::{Space, SpaceId};
    use crate::exec::run::{Bindings, Sink};
    use crate::ir::validate::SignatureColumn;
    use crate::work::{GenerationHandle, GenerationState};

    let work = crate::WorkContext::new();
    let generation = GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
    ));
    let interner = crate::image::intern::InternerHandle::new(&generation, &work);
    let space = Space::new(SpaceId([2; 32]), 3, &work).unwrap();
    let original = [
        space.empty(),
        space.full(),
        space.coordinate(1, &work).unwrap(),
    ];
    let mut sink = ProjectionSink::new(vec![0, 1]);
    let mut binding = Bindings::new(2);
    for value in &original {
        let words = interner.intern_event(value).unwrap().key().words();
        binding.set(0, words[0]);
        binding.set(1, words[1]);
        assert!(!sink.emit(&binding).is_terminal());
    }
    sink.force_spill().unwrap();
    let columns = [SignatureColumn::Project {
        ty: ValueType::Event,
    }];
    let mut out = Answers::new();
    out.begin(1);
    super::super::finalize::finalize(
        &mut EitherSink::Projection(sink),
        &mut Vec::new(),
        &mut ResolveMemo::new(),
        &interner,
        &columns,
        &mut out,
        &work,
    )
    .unwrap();
    drop(generation);
    let mut counts: Vec<_> = (0..out.len())
        .map(|row| {
            let AnswerValue::Event(value) = out.get(row, 0) else {
                panic!("Event")
            };
            value.count(&work).unwrap()
        })
        .collect();
    counts.sort_unstable();
    assert_eq!(counts, [0, 4, 8]);

    let fresh = GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
    ));
    let interner = crate::image::intern::InternerHandle::new(&fresh, &work);
    let mut sink = ProjectionSink::new(vec![0, 1]);
    assert!(!sink.emit(&binding).is_terminal());
    let mut refused = Answers::new();
    refused.begin(1);
    assert!(matches!(
        super::super::finalize::finalize(
            &mut EitherSink::Projection(sink),
            &mut Vec::new(),
            &mut ResolveMemo::new(),
            &interner,
            &columns,
            &mut refused,
            &work,
        ),
        Err(crate::Error::Event(crate::event::Error::UnknownKey))
    ));
    assert!(refused.is_empty());
}

fn finalize_mixed_test_rows(
    rows: &[[u64; 4]],
    out: &mut Answers,
    dense: bool,
) -> crate::error::Result<()> {
    use crate::exec::run::{Bindings, Sink};
    use crate::ir::validate::SignatureColumn;

    let mut projection = ProjectionSink::new(vec![0, 1, 2, 3]);
    if dense {
        projection.elide_output_hashing(mixed_output_witness());
    }
    let mut bindings = Bindings::new(4);
    for row in rows {
        for (slot, word) in row.iter().enumerate() {
            bindings.set(slot, *word);
        }
        assert!(!projection.emit(&bindings).is_terminal());
    }
    let work = crate::api::db::test_operation();
    let interner = crate::image::intern::InternerHandle::without_text(&work);
    let columns = [
        ValueType::U64,
        ValueType::FixedBytes { len: 9 },
        ValueType::F64,
    ]
    .map(|ty| SignatureColumn::Project { ty });
    super::super::finalize::finalize(
        &mut EitherSink::Projection(projection),
        &mut Vec::new(),
        &mut ResolveMemo::new(),
        &interner,
        &columns,
        out,
        &work,
    )
}

fn mixed_output_witness() -> crate::plan::fj::ProjectionDistinctWitness {
    use crate::schema::ValidateDescriptor as _;
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "mixed".into(),
            fields: [
                ValueType::U64,
                ValueType::FixedBytes { len: 9 },
                ValueType::F64,
            ]
            .into_iter()
            .enumerate()
            .map(|(field, value_type)| FieldDescriptor {
                name: format!("field{field}").into(),
                value_type,
            })
            .collect(),
            extension: None,
        }],
        statements: vec![],
    }
    .validate()
    .unwrap();
    let query = Query::single(Rule {
        finds: (0..3).map(|var| FindTerm::Var(VarId(var))).collect(),
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: (0..3)
                .map(|field| (FieldId(field), Term::Var(VarId(field))))
                .collect(),
        }],
        negated: vec![],
        conditions: vec![],
    });
    let validated = crate::ir::validate::validate(&schema, &query).unwrap();
    let normalized = crate::ir::normalize::normalize_rules(&schema, &[], validated.rules());
    crate::plan::fj::provably_distinct_projection(&normalized[0], &schema, &query.rules()[0].finds)
        .unwrap()
}

#[test]
fn bulk_finalize_keeps_initialized_prefix_on_error_and_matches_rowwise_retry() {
    for dense in [false, true] {
        assert_bulk_finalize_prefix_and_retry(dense);
    }
}

fn assert_bulk_finalize_prefix_and_retry(dense: bool) {
    let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    let rows = [
        [
            7,
            0x0102_0304_0506_0708,
            0x0900_0000_0000_0000,
            0xbff0_0000_0000_0000,
        ],
        [
            8,
            0x0102_0304_0506_0708,
            0x0900_0000_0000_0000,
            0xc000_0000_0000_0000,
        ],
    ];
    let mut bad = rows;
    bad[1][3] = 0; // Noncanonical F64 order key, after the first float succeeds.
    let mut out = Answers::new();
    out.begin(3);
    out.push_value(&AnswerValue::U64(99));
    out.push_value(&AnswerValue::FixedBytes(&bytes));
    out.push_value(&AnswerValue::F64(crate::F64::from(3.0)));
    let failed = finalize_mixed_test_rows(&bad, &mut out, dense);
    assert!(matches!(failed, Err(Error::Corruption(_))));
    assert_eq!(
        out.len(),
        1,
        "no partially initialized appended row is published"
    );
    assert_eq!(out.get(0, 0), AnswerValue::U64(99));
    assert_eq!(out.get(0, 1), AnswerValue::FixedBytes(&bytes));
    assert_eq!(out.get(0, 2), AnswerValue::F64(crate::F64::from(3.0)));

    // Both layouts must agree with literals, including the two-word byte
    // find between single-word finds and reuse after a failed bulk fill.
    for _ in 0..3 {
        out.begin(3);
        finalize_mixed_test_rows(&rows, &mut out, dense).unwrap();
        assert_eq!(out.len(), 2);
        for (row, id, value) in [(0, 7, 1.0), (1, 8, 2.0)] {
            assert_eq!(out.get(row, 0), AnswerValue::U64(id));
            assert_eq!(out.get(row, 1), AnswerValue::FixedBytes(&bytes));
            assert_eq!(out.get(row, 2), AnswerValue::F64(crate::F64::from(value)));
        }
    }
    out.begin(3);
    finalize_mixed_test_rows(&[], &mut out, dense).unwrap();
    assert!(out.is_empty());
}

#[test]
fn overflow_errors_leave_answers_reusable() {
    let fix = postings(&[(1, 7, "a", i64::MAX), (2, 7, "b", 1), (3, 8, "c", 4)]);

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepares");
    let mut out = Answers::new();
    for _ in 0..2 {
        let err = fix
            .execute_into(&mut prepared, &[] as &[BindValue], &mut out)
            .expect_err("account 7 overflows");
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: FindIndex(1) })
            ),
            "{err:?}"
        );
    }

    let ok_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(crate::ir::Value::U64(8)),
        })],
    });
    let mut ok = fix.prepare(&ok_query).expect("prepares");
    fix.execute_into(&mut ok, &[] as &[BindValue], &mut out)
        .expect("executes");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::I64(4));
}

#[test]
fn answer_reuse_retains_capacity_and_answers_stay_identical() {
    let fix = postings(&[(1, 7, "one", 1), (2, 7, "two", 2), (3, 7, "three", 3)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();
    let params = [BindValue::U64(7), BindValue::I64(0)];

    fix.execute_into(&mut prepared, &params, &mut out)
        .expect("execute");
    let first = answers_of(&out);
    let (cells_cap, text_cap) = (out.cells.capacity(), out.text.capacity());
    assert!(cells_cap > 0 && text_cap > 0);

    fix.execute_into(&mut prepared, &params, &mut out)
        .expect("execute");
    assert_eq!(answers_of(&out), first);

    assert!(out.cells.capacity() >= cells_cap);
    assert!(out.text.capacity() >= text_cap);
    assert_eq!(first.len(), 3);
}

#[test]
fn completed_results_own_shared_text_across_prepared_reuse() {
    use super::super::result::{CompleteResult, ResultIdentity};
    use super::super::source::{PinnedSource, QuerySource};

    let texts = ["shared", "a-much-longer-shared-text", ""];
    let facts: Vec<_> = (0..514u64)
        .map(|id| {
            (
                id,
                7,
                texts[usize::try_from(id % 3).unwrap()],
                id.cast_signed(),
            )
        })
        .collect();
    let fix = posting_store("result-batch-text", &facts);
    let mut prepared = fix.prepare(&by_account_query()).unwrap();
    let mut expected: Vec<_> = facts
        .iter()
        .map(|(_, _, text, n)| ((*text).to_owned(), *n))
        .collect();
    expected.sort();
    let params = [BindValue::U64(7), BindValue::I64(-1)];
    for fallback in [false, true] {
        prepared.force_cursor_fallback(fallback);
        for _ in 0..3 {
            let complete = fix
                .db
                .read(crate::api::db::test_operation(), |instance| {
                    let context = instance.work();
                    let source = QuerySource::store(instance.snapshot(), context);
                    let mut carrier = Answers::new();
                    prepared.execute_source(&source, &params, &mut carrier)?;
                    CompleteResult::seal(
                        carrier,
                        ResultIdentity {
                            source: PinnedSource::Store(instance.snapshot().identity()),
                            generation: Some(instance.snapshot().generation()),
                        },
                        context,
                    )
                })
                .unwrap();
            assert_eq!(complete.len(), 514);
            prepared.release_memory();
            let rows = complete.into_answers();
            assert_eq!(answers_of(&rows), expected, "fallback={fallback}");
        }
    }
}

#[test]
fn finalize_materializes_each_distinct_intern_once() {
    let facts: Vec<(u64, u64, String, i64)> = (0..64)
        .map(|id| {
            (
                id,
                1,
                "shared-memo".to_owned(),
                i64::try_from(id).expect("fits"),
            )
        })
        .chain((0..16).map(|i| (64 + i, 2, format!("m{i}"), i64::try_from(i).expect("fits"))))
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = facts
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let fix = postings(&borrowed);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");

    let resolves = |prepared: &mut PreparedQuery<T>, account: u64| {
        let out = fix
            .execute(prepared, &[BindValue::U64(account), BindValue::I64(-1)])
            .expect("execute");
        let count = prepared.resolve_memo.ranges.len();
        (out, count)
    };

    let (out, count) = resolves(&mut prepared, 1);
    assert_eq!(out.len(), 64);
    assert_eq!(count, 1, "one memo entry for the shared intern");
    assert_eq!(out.byte_len(), "shared-memo".len(), "bytes stored once");

    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(out.len(), 16);
    assert_eq!(count, 16);

    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(count, 16, "each finalize re-resolves into the answer heap");
    assert_eq!(out.len(), 16);
    let mut memos: Vec<String> = (0..out.len())
        .map(|answer| {
            let AnswerValue::String(memo) = out.get(answer, 0) else {
                panic!("column 0 is a string");
            };
            memo.to_owned()
        })
        .collect();
    memos.sort();
    let expected: Vec<String> = {
        let mut memos: Vec<String> = (0..16).map(|i| format!("m{i}")).collect();
        memos.sort();
        memos
    };
    assert_eq!(memos, expected, "answer heap materializes the same text");
}

/// Forced work refusal during text compare fails the query. It does
/// not return a successful empty or wrong answer.
#[test]
fn text_compare_refusal_fails_the_query() {
    let fix = postings(&[(1, 7, "alpha", 10), (2, 7, "beta", 20)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    prepared.force_cursor_fallback(true);
    let work = crate::work::WorkContext::new();
    work.cancel();
    let source = crate::api::prepared::source::QuerySource::heap(&fix.instance, 1, work);
    let mut out = Answers::new();
    let err = prepared.execute_source(&source, &[BindValue::U64(7), BindValue::I64(0)], &mut out);
    assert!(err.is_err(), "refusal must fail the query");
}
