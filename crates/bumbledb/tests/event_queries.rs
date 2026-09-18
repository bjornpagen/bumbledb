//! Whole-binding query semantics checked against small explicit legal worlds.
use bumbledb::{
    AnswerValue, BindValue, Db, Error, Event, EventExpr, EventTest, FindTerm, VarId,
    event::{BoolOp4, Space, SpaceId},
    query,
};

mod common;

bumbledb::schema! {
    pub EventQueries;
    relation Region { id: u64, region: event }
    relation Pair { id: u64, left: event, right: event, unused: event }
    relation Gate { id: u64 }
    Region(id) -> Region;
    Pair(id) -> Pair;
    Gate(id) -> Gate;
}

fn mask(event: &Event) -> u64 {
    (0..4)
        .filter(|&world| event.contains(world).unwrap_or(false))
        .fold(0, |bits, world| bits | (1 << world))
}

#[test]
fn macro_event_heads_match_explicit_worlds_on_complete_and_coupled_supports() {
    for support in [15, 7, 5] {
        let directory = common::TempDir::new(&format!("event-heads-{support}"));
        let db = Db::create(directory.path(), EventQueries, common::work())
            .unwrap()
            .unwrap();
        let original = Space::new(SpaceId([1; 32]), 2, &()).unwrap();
        let source = original
            .restrict(&original.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        db.write(common::work(), |tx| {
            for bits in 0..16 {
                if bits & !support == 0 {
                    tx.insert([&Region {
                        id: bits,
                        region: source.table(3, &[bits], &())?,
                    }])?;
                }
            }
            Ok(())
        })
        .unwrap()
        .unwrap();
        let template = query!(EventQueries {
            (x, y,
             neither: Event(!(a | b)),
             exclusive: Event(a ^ b),
             both: Event(a & b),
             choice: Event(Ite(a, b, !b)),
             count: Event(Exactly(2, a, a, b)),
             low: Event(AtMost(1, a, b)),
             high: Event(AtLeast(1, a, b)),
             precedence: Event(a | b ^ a & b),
             bottom: Event(Empty(a)), top: Event(Full(a)),
             empty: Test(IsEmpty(a)), full: Test(IsFull(a)),
             subset: Test(Subset(a, b)), equal: Test(Equal(a, b)),
             separate: Test(Disjoint(a, b)), cover: Test(Covers(a, b))) |
                Region(id: x, region: a), Region(id: y, region: b);
        });
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        for fallback in [false, true] {
            prepared.force_cursor_fallback(fallback);
            let answers = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            let count = 1usize << support.count_ones();
            assert_eq!(answers.len(), count * count);
            for row in 0..answers.len() {
                let (AnswerValue::U64(a), AnswerValue::U64(b)) =
                    (answers.get(row, 0), answers.get(row, 1))
                else {
                    panic!("scalar ids");
                };
                let regions = [
                    !(a | b) & support,
                    a ^ b,
                    a & b,
                    (a & b) | (!a & !b & support),
                    a & !b,
                    !(a & b) & support,
                    a | b,
                    a | (b ^ (a & b)),
                    0,
                    support,
                ];
                for (column, expected) in regions.into_iter().enumerate() {
                    let AnswerValue::Event(value) = answers.get(row, column + 2) else {
                        panic!("Event column");
                    };
                    assert_eq!(
                        mask(value),
                        expected,
                        "support={support}, a={a}, b={b}, column={column}"
                    );
                }
                for (column, expected) in [
                    a == 0,
                    a == support,
                    a & !b == 0,
                    a == b,
                    a & b == 0,
                    a | b == support,
                ]
                .into_iter()
                .enumerate()
                {
                    assert_eq!(answers.get(row, 12 + column), AnswerValue::Bool(expected));
                }
            }
        }
    }
}

#[test]
fn empty_values_survive_interiors_and_can_be_complemented_and_joined() {
    let directory = common::TempDir::new("event-empty-staging");
    let db = Db::create(directory.path(), EventQueries, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            region: source.coordinate(0, &())?,
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(EventQueries {
        interior empty(id, zero: Event(a & !a)) | Region(id, region: a);
        interior full(id, all: Event(!zero), yes: Test(IsEmpty(zero))) | empty(id, zero);
        (id, all) | full(id, all, yes), yes == true, Region(id: id);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(answers.len(), 1);
    drop(prepared);
    drop(db);
    let AnswerValue::Event(all) = answers.get(0, 1) else {
        panic!("owned Event result");
    };
    assert_eq!(mask(all), 15);
    assert!(all.is_full());
}

#[test]
fn complete_fault_sets_ignore_physical_order_and_unmatched_operands() {
    let source = Space::new(SpaceId([3; 32]), 2, &()).unwrap();
    let foreign = Space::new(SpaceId([4; 32]), 2, &()).unwrap();
    let coupled = source
        .restrict(&source.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let mut baseline = None;
    for reverse in [false, true] {
        let directory = common::TempDir::new(&format!("event-fault-order-{reverse}"));
        let db = Db::create(directory.path(), EventQueries, common::work())
            .unwrap()
            .unwrap();
        let mut rows = vec![
            Pair {
                id: 1,
                left: source.empty(),
                right: foreign.full(),
                unused: coupled.full(),
            },
            Pair {
                id: 2,
                left: source.full(),
                right: coupled.empty(),
                unused: foreign.empty(),
            },
            Pair {
                id: 3,
                left: source.full(),
                right: source.empty(),
                unused: foreign.full(),
            },
            Pair {
                id: 4,
                left: foreign.empty(),
                right: coupled.full(),
                unused: source.full(),
            },
        ];
        if reverse {
            rows.reverse();
        }
        db.write(common::work(), |tx| {
            for row in &rows {
                tx.insert([row])?;
            }
            for id in [1, 2, 3] {
                tx.insert([&Gate { id }])?;
            }
            Ok(())
        })
        .unwrap()
        .unwrap();
        let template = query!(EventQueries {
            (result: Event(Empty(a) & b), branch: Event(Ite(Full(a), a, b)),
             saturated: Event(AtLeast(0, a, b)), test: Test(Covers(Full(a), b))) |
                Pair(id: id, left: a, right: b), Gate(id: id);
            (result: Event(Empty(a) & b), branch: Event(Ite(Full(a), a, b)),
             saturated: Event(AtLeast(0, a, b)), test: Test(Covers(Full(a), b))) |
                Pair(id: id, left: a, right: b), Gate(id: id), or(id == 1, id == 1);
            (result: Event(Empty(a) & b), branch: Event(Ite(Full(a), a, b)),
             saturated: Event(AtLeast(0, a, b)), test: Test(Covers(Full(a), b))) |
                Pair(id: id, left: a, right: b), Gate(id: id);
        });
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        for fallback in [false, true] {
            prepared.force_cursor_fallback(fallback);
            let error = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap_err();
            let Error::EventFaults(faults) = error else {
                panic!("complete fault set: {error:?}");
            };
            assert_eq!(
                faults.len(),
                20,
                "two bad rows in each of rules 0 and 2, one in rule 1, four heads"
            );
            assert!(faults.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(faults.iter().all(|f| f.stage.is_none()));
            if let Some(expected) = &baseline {
                assert_eq!(&faults, expected);
            } else {
                baseline = Some(faults);
            }
        }
    }
}

#[test]
fn a_consumer_filter_cannot_hide_a_fault_in_its_producer() {
    let directory = common::TempDir::new("event-producer-fault");
    let db = Db::create(directory.path(), EventQueries, common::work())
        .unwrap()
        .unwrap();
    let a = Space::new(SpaceId([5; 32]), 1, &()).unwrap();
    let b = Space::new(SpaceId([6; 32]), 1, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Pair {
            id: 1,
            left: a.full(),
            right: b.empty(),
            unused: a.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(EventQueries {
        interior produced(id, result: Event(a | b)) | Pair(id, left: a, right: b);
        (result) | produced(0 == 999, 1: result);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    for fallback in [false, true] {
        prepared.force_cursor_fallback(fallback);
        let error = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err();
        let Error::EventFaults(faults) = error else {
            panic!("producer must fail");
        };
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].stage, Some(0));
    }
}

#[test]
fn every_truth_function_checks_context_even_when_constant_or_one_sided() {
    let directory = common::TempDir::new("event-truth-function-admission");
    let db = Db::create(directory.path(), EventQueries, common::work())
        .unwrap()
        .unwrap();
    let a = Space::new(SpaceId([7; 32]), 1, &()).unwrap();
    let b = Space::new(SpaceId([8; 32]), 1, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Pair {
            id: 1,
            left: a.empty(),
            right: b.empty(),
            unused: a.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(EventQueries { (result: Event(a & b)) | Pair(left: a, right: b); });
    for bits in 0..16 {
        let mut ir = (*template).clone();
        ir.rules[0].finds = vec![FindTerm::Event(EventExpr::Apply {
            op: BoolOp4::new(bits).unwrap(),
            left: Box::new(EventExpr::Var(VarId(0))),
            right: Box::new(EventExpr::Var(VarId(1))),
        })];
        let mut prepared = db.prepare(&ir, common::work()).unwrap();
        let error = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err();
        assert!(matches!(error, Error::EventFaults(faults) if faults.len() == 1));
    }
}

#[test]
fn static_shape_and_type_refusals_precede_query_normalization() {
    let directory = common::TempDir::new("event-query-shapes");
    let db = Db::create(directory.path(), EventQueries, common::work())
        .unwrap()
        .unwrap();
    let template = query!(EventQueries { (id) | Region(id); });
    let mut ir = (*template).clone();
    for term in [
        FindTerm::Event(EventExpr::Var(VarId(0))),
        FindTerm::Test(EventTest::IsEmpty(EventExpr::Var(VarId(0)))),
        FindTerm::Event(EventExpr::Cardinality {
            minimum: 0,
            maximum: 0,
            events: vec![],
        }),
    ] {
        ir.rules[0].finds = vec![term];
        ir.head = ir.rules[0].head();
        assert!(db.prepare(&ir, common::work()).is_err());
    }
    let mut deep = EventExpr::Var(VarId(0));
    for _ in 0..129 {
        deep = EventExpr::Not(Box::new(deep));
    }
    ir.rules[0].finds = vec![FindTerm::Event(deep)];
    assert!(matches!(
        db.prepare(&ir, common::work()),
        Err(Error::Validation(
            bumbledb::ValidationError::EventExpression {
                source: bumbledb::EventExprError::TooDeep,
                ..
            }
        ))
    ));
}
