use bumbledb::{
    Admission, AnswerValue, BindValue, Db, Error, Event, Fact, Value,
    event::{Error as EventError, Limits, Space, SpaceId},
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::{FieldId, StatementId},
};

mod common;

bumbledb::schema! {
    pub Partitions;
    relation Roster { group: u64 }
    relation Parent { group: u64, condition: event }
    relation Child { group: u64, item: u64, condition: event }

    Roster(group) -> Roster;
    Parent(group) -> Parent;
    Parent(group, condition) -> Parent;
    Child(group, condition) -> Child;
    Child(group) <= Roster(group);
    Parent(group, condition) == Child(group, condition);
}

fn space(reverse: bool) -> Space {
    Space::with_order(
        SpaceId([51; 32]),
        if reverse { &[1, 0] } else { &[0, 1] },
        Limits::default(),
        &(),
    )
    .unwrap()
}

fn mask(space: &Space, bits: u64) -> Event {
    space.table(3, &[bits], &()).unwrap()
}

fn bytes(value: &Event) -> Vec<u8> {
    value.to_bytes(&()).unwrap()
}

fn children(db: &Db<Partitions>) -> Vec<Child> {
    db.read(common::work(), |snapshot| snapshot.scan_facts()?.collect())
        .unwrap()
}

fn seed(db: &Db<Partitions>, source: &Space) {
    db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }])?;
        tx.insert([&Parent {
            group: 1,
            condition: source.full(),
        }])?;
        tx.insert([
            &Child {
                group: 1,
                item: 10,
                condition: mask(source, 0b0101),
            },
            &Child {
                group: 1,
                item: 20,
                condition: mask(source, 0b1010),
            },
        ])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
}

#[test]
fn native_partitions_match_four_world_bitset_oracle() {
    let dir = common::TempDir::new("event-partition-oracle");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    // A nonrectangular legal support also exercises support-relative completion.
    let source = space(false);
    for support in [0b1111, 0b0110, 0b0001] {
        let restricted = source.restrict(&mask(&source, support), &()).unwrap();
        for a in 0..16 {
            for b in 0..16 {
                let parent = Parent {
                    group: 1,
                    condition: mask(&restricted, a | b),
                };
                let left = Child {
                    group: 1,
                    item: 1,
                    condition: mask(&restricted, a),
                };
                let right = Child {
                    group: 1,
                    item: 2,
                    condition: mask(&restricted, b),
                };
                let result = db
                    .write(common::work(), |tx| {
                        tx.insert([&Roster { group: 1 }])?;
                        tx.insert([&parent])?;
                        tx.insert([&left, &right])?;
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(
                    matches!(result, Admission::Accepted(_)),
                    a & b & support == 0,
                    "support={support:04b}, a={a:04b}, b={b:04b}"
                );
                match result {
                    Admission::Accepted(_) => {
                        db.write(common::work(), |tx| {
                            tx.delete([&Roster { group: 1 }])?;
                            tx.delete([&parent])?;
                            tx.delete([&left, &right])?;
                            Ok(())
                        })
                        .unwrap()
                        .unwrap();
                    }
                    Admission::Rejected(violations) => {
                        assert_eq!(violations.len(), 1);
                        assert_eq!(
                            violations.get(0).unwrap().statement_id(db.schema()),
                            StatementId(3)
                        );
                        assert_eq!(violations.cited_facts(0).len(), 2);
                        assert!(children(&db).is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn empty_facts_coexist_but_scalar_roster_law_still_applies() {
    let dir = common::TempDir::new("event-empty-partition");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }])?;
        tx.insert([
            &Child {
                group: 1,
                item: 1,
                condition: source.empty(),
            },
            &Child {
                group: 1,
                item: 2,
                condition: source.empty(),
            },
        ])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    // No Parent fact is needed to cover an empty Child Event.
    assert_eq!(children(&db).len(), 2);
    let violations = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Child {
            group: 2,
            item: 3,
            condition: source.empty(),
        }])?;
        Ok(())
    }));
    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations.get(0).unwrap().statement_id(db.schema()),
        StatementId(4)
    );
    assert_eq!(children(&db).len(), 2);
}

#[test]
fn final_state_repair_and_contributor_deletion_preserve_exact_coverage() {
    let dir = common::TempDir::new("event-partition-final-state");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    seed(&db, &source);
    let old = Child {
        group: 1,
        item: 10,
        condition: mask(&source, 0b0101),
    };
    let replacement = Child {
        group: 1,
        item: 30,
        condition: mask(&source, 0b0101),
    };
    db.write(common::work(), |tx| {
        // Insertion alone overlaps; only the atomic final state is judged.
        tx.insert([&replacement])?;
        tx.delete([&old])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let before = db.generation(common::work()).unwrap();
    let violations = common::expect_rejected(db.write(common::work(), |tx| {
        tx.delete([&replacement])?;
        Ok(())
    }));
    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations.get(0).unwrap().statement_id(db.schema()),
        StatementId(5)
    );
    assert_eq!(db.generation(common::work()).unwrap(), before);
    assert_eq!(children(&db).len(), 2);
    // Deleting the contributor and reducing the required coverage is lawful.
    db.write(common::work(), |tx| {
        tx.delete([&replacement])?;
        tx.delete([&Parent {
            group: 1,
            condition: source.full(),
        }])?;
        tx.insert([&Parent {
            group: 1,
            condition: mask(&source, 0b1010),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert_eq!(children(&db).len(), 1);
}

#[test]
fn canonical_duplicates_collapse_but_distinct_whole_facts_conflict() {
    let dir = common::TempDir::new("event-partition-dedup");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    seed(&db, &source);
    let other = space(true);
    db.write(common::work(), |tx| {
        assert_eq!(
            tx.insert([&Child {
                group: 1,
                item: 10,
                condition: mask(&other, 0b0101),
            }])?
            .changed(),
            0
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    let violations = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Child {
            group: 1,
            item: 11,
            condition: mask(&other, 0b0101),
        }])?;
        Ok(())
    }));
    assert_eq!(violations.len(), 1);
    assert_eq!(violations.cited_facts(0).len(), 2);
    let items: Vec<_> = violations
        .cited_facts(0)
        .iter()
        .map(|fact| fact.values()[1].clone())
        .collect();
    assert_eq!(items, [Value::U64(10), Value::U64(11)]);
}

#[test]
fn keyed_reads_require_nonempty_event_or_an_independent_scalar_key() {
    let dir = common::TempDir::new("event-partition-get");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    seed(&db, &space(false));
    let other = space(true);
    db.read(common::work(), |snapshot| {
        let found = snapshot
            .get(ChildByGroupCondition {
                group: 1,
                condition: mask(&other, 0b0101),
            })?
            .unwrap();
        assert_eq!(found.item, 10);
        assert_eq!(bytes(&found.condition), bytes(&mask(&other, 0b0101)));
        assert!(matches!(
            snapshot.get(ChildByGroupCondition {
                group: 1,
                condition: other.empty()
            }),
            Err(Error::FactShape(
                bumbledb::error::FactShapeError::EmptyEventLookup { .. }
            ))
        ));
        Ok(())
    })
    .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Parent {
            group: 2,
            condition: other.empty(),
        }])?;
        // Parent(group) is independently unique, so its empty Event key is safe.
        assert!(
            tx.get(ParentByGroupCondition {
                group: 2,
                condition: space(false).empty()
            })?
            .is_some()
        );
        assert_eq!(
            tx.get(ChildByGroupCondition {
                group: 1,
                condition: mask(&other, 0b1010)
            })?
            .unwrap()
            .item,
            20
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    db.read(common::work(), |snapshot| {
        assert!(
            snapshot
                .get(ParentByGroupCondition {
                    group: 2,
                    condition: space(false).empty()
                })?
                .is_some()
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn free_join_deduplicates_empty_key_projections() {
    let dir = common::TempDir::new("event-partition-distinctness");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    seed(&db, &source);
    db.write(common::work(), |tx| {
        tx.insert([
            &Child {
                group: 1,
                item: 100,
                condition: source.empty(),
            },
            &Child {
                group: 1,
                item: 200,
                condition: space(true).empty(),
            },
        ])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Child::RELATION),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 3);
        let mut empty = 0;
        for row in 0..answers.len() {
            if let AnswerValue::Event(value) = answers.get(row, 1) {
                empty += usize::from(value.is_empty());
            } else {
                panic!("Event projection");
            }
        }
        assert_eq!(empty, 1);
    }
}

#[test]
fn incompatible_context_refuses_even_for_empty_values() {
    for mode in 0..3 {
        let dir = common::TempDir::new(&format!("event-partition-context-{mode}"));
        let db = Db::create(dir.path(), Partitions, common::work())
            .unwrap()
            .unwrap();
        let source = space(false);
        let foreign = Space::new(SpaceId([52; 32]), 2, &()).unwrap();
        let error = db
            .write(common::work(), |tx| {
                tx.insert([&Roster { group: 1 }])?;
                tx.insert([&Child {
                    group: 1,
                    item: 1,
                    condition: source.empty(),
                }])?;
                if mode == 0 {
                    tx.insert([&Child {
                        group: 1,
                        item: 2,
                        condition: foreign.empty(),
                    }])?;
                } else {
                    tx.insert([&Parent {
                        group: 1,
                        condition: if mode == 1 {
                            foreign.empty()
                        } else {
                            foreign.full()
                        },
                    }])?;
                }
                Ok(())
            })
            .unwrap_err();
        assert!(
            matches!(error, Error::Event(EventError::SpaceMismatch)),
            "{error:?}"
        );
        assert!(children(&db).is_empty());
    }
}

#[test]
fn rejection_citations_are_stable_and_exclude_empty_or_disjoint_facts() {
    let mut evidence = Vec::new();
    for reverse in [false, true] {
        let dir = common::TempDir::new(&format!("event-partition-citations-{reverse}"));
        let db = Db::create(dir.path(), Partitions, common::work())
            .unwrap()
            .unwrap();
        let source = space(reverse);
        let mut rows = vec![
            Child {
                group: 1,
                item: 30,
                condition: mask(&source, 0b0011),
            },
            Child {
                group: 1,
                item: 10,
                condition: mask(&source, 0b0010),
            },
            Child {
                group: 1,
                item: 20,
                condition: mask(&source, 0b0101),
            },
            Child {
                group: 1,
                item: 40,
                condition: mask(&source, 0b1000),
            },
            Child {
                group: 1,
                item: 50,
                condition: source.empty(),
            },
        ];
        if reverse {
            rows.reverse();
        }
        let violations = common::expect_rejected(db.write(common::work(), |tx| {
            tx.insert([&Roster { group: 1 }])?;
            tx.insert([&Parent {
                group: 1,
                condition: source.full(),
            }])?;
            for row in &rows {
                tx.insert([row])?;
            }
            Ok(())
        }));
        assert_eq!(violations.len(), 1);
        let cited: Vec<_> = violations
            .cited_facts(0)
            .iter()
            .map(|fact| {
                let [Value::U64(group), Value::U64(item), Value::Event(condition)] = fact.values()
                else {
                    panic!("Child citation");
                };
                (*group, *item, bytes(condition))
            })
            .collect();
        assert_eq!(
            cited.iter().map(|(_, item, _)| *item).collect::<Vec<_>>(),
            [10, 20, 30]
        );
        evidence.push(cited);
    }
    assert_eq!(evidence[0], evidence[1]);
}

#[test]
fn reference_state_deduplicates_event_meaning_and_refuses_cancelled_insertion() {
    use bumbledb::schema::judge::{
        CandidateFacts, JudgeBudget, Judgment, MapState, judge_complete,
    };
    let dir = common::TempDir::new("event-reference-identity");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    let mut state = MapState::new();
    state.insert(Roster::RELATION, vec![Value::U64(1)]).unwrap();
    state
        .insert(
            Parent::RELATION,
            vec![Value::U64(1), Value::Event(source.full())],
        )
        .unwrap();
    assert!(
        state
            .insert(
                Child::RELATION,
                vec![Value::U64(1), Value::U64(1), Value::Event(source.full())]
            )
            .unwrap()
    );
    assert!(
        !state
            .insert(
                Child::RELATION,
                vec![
                    Value::U64(1),
                    Value::U64(1),
                    Value::Event(space(true).full())
                ]
            )
            .unwrap()
    );
    assert_eq!(
        judge_complete(db.schema(), &state, &common::work(), JudgeBudget::default()).unwrap(),
        Judgment::Admitted
    );
    let cancelled = common::work();
    cancelled.cancel();
    assert_eq!(
        state.insert_with_control(
            Child::RELATION,
            vec![Value::U64(1), Value::U64(2), Value::Event(source.full())],
            &cancelled
        ),
        Err(EventError::Cancelled)
    );
    let mut count = 0;
    state
        .visit_rows(Child::RELATION, &mut |_| {
            count += 1;
            Ok(true)
        })
        .unwrap();
    assert_eq!(count, 1);
    // Same region, different whole-fact identity: now the pointwise law fails.
    state
        .insert(
            Child::RELATION,
            vec![Value::U64(1), Value::U64(2), Value::Event(source.full())],
        )
        .unwrap();
    assert!(matches!(
        judge_complete(db.schema(), &state, &common::work(), JudgeBudget::default()).unwrap(),
        Judgment::Rejected(_)
    ));
}

#[test]
fn groups_route_independently_and_same_name_does_not_erase_support() {
    let dir = common::TempDir::new("event-partition-groups");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    let restricted = source.restrict(&mask(&source, 0b0110), &()).unwrap();
    db.write(common::work(), |tx| {
        for (group, domain) in [(1, &source), (2, &restricted)] {
            tx.insert([&Roster { group }])?;
            tx.insert([&Parent {
                group,
                condition: domain.full(),
            }])?;
            tx.insert([&Child {
                group,
                item: 1,
                condition: domain.full(),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let error = db
        .write(common::work(), |tx| {
            tx.insert([&Child {
                group: 1,
                item: 2,
                condition: restricted.empty(),
            }])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(error, Error::Event(EventError::SpaceMismatch)));
    assert_eq!(children(&db).len(), 2);
}

#[test]
fn cancellation_before_admission_preserves_the_lawful_parent() {
    let dir = common::TempDir::new("event-partition-cancel");
    let db = Db::create(dir.path(), Partitions, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    seed(&db, &source);
    let before = db.generation(common::work()).unwrap();
    let work = common::work();
    let result = db.write(work.clone(), |tx| {
        tx.insert([&Child {
            group: 1,
            item: 3,
            condition: source.full(),
        }])?;
        work.cancel();
        Ok(())
    });
    assert!(
        result.is_err(),
        "cancellation is a refusal, never a partial verdict"
    );
    assert_eq!(db.generation(common::work()).unwrap(), before);
    assert_eq!(children(&db).len(), 2);
}

#[test]
fn empty_uncovered_source_groups_still_require_a_common_context() {
    bumbledb::schema! {
        pub OneWay;
        relation Demand { id: u64, group: u64, condition: event }
        relation Supply { group: u64, condition: event }
        Demand(id) -> Demand;
        Supply(group, condition) -> Supply;
        Demand(group, condition) <= Supply(group, condition);
    }
    let dir = common::TempDir::new("event-empty-source-context");
    let db = Db::create(dir.path(), OneWay, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    let foreign = Space::new(SpaceId([61; 32]), 2, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            &Demand {
                id: 1,
                group: 1,
                condition: source.empty(),
            },
            &Demand {
                id: 2,
                group: 1,
                condition: space(true).empty(),
            },
        ])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let error = db
        .write(common::work(), |tx| {
            tx.insert([&Demand {
                id: 3,
                group: 1,
                condition: foreign.empty(),
            }])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(error, Error::Event(EventError::SpaceMismatch)));
}

#[test]
fn event_keys_keep_exact_target_and_region_shape_requirements() {
    use bumbledb::error::{SchemaError, StatementErrorKind};
    use bumbledb::schema::{StatementDescriptor, Theory as _, ValidateDescriptor as _};
    let mut wrong_order = Partitions.descriptor();
    let StatementDescriptor::Functionality { projection, .. } = &mut wrong_order.statements[3]
    else {
        unreachable!()
    };
    let bumbledb::schema::Projection::Fields(fields) = projection else {
        unreachable!()
    };
    fields.reverse();
    assert!(matches!(
        wrong_order.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::FunctionalityEventNotLast { .. },
            ..
        })
    ));
    let mut missing_key = Partitions.descriptor();
    missing_key.statements.remove(3);
    assert!(matches!(
        missing_key.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::NoPointwiseTargetKey { .. },
            ..
        })
    ));
    let mut two_regions = Partitions.descriptor();
    two_regions.relations[2].fields[1].value_type = bumbledb::schema::ValueType::Event;
    let StatementDescriptor::Functionality { projection, .. } = &mut two_regions.statements[3]
    else {
        unreachable!()
    };
    *projection = vec![FieldId(0), FieldId(1), FieldId(2)].into();
    assert!(matches!(
        two_regions.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::FunctionalityMultipleRegions { .. },
            ..
        })
    ));
}
