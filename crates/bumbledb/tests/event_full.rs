use bumbledb::{
    Db, Event, Value,
    event::{Space, SpaceId},
    schema::StatementId,
};

mod common;

bumbledb::schema! {
    pub FullPartitions;
    relation Roster { group: u64 }
    relation Branch { group: u64, choice: u64, when: event }
    Roster(group) -> Roster;
    Roster(group, true) -> Roster;
    Branch(group, when) -> Branch;
    Branch(group) <= Roster(group);
    Roster(group, true) == Branch(group, when);
}

fn source() -> Space {
    Space::new(SpaceId([83; 32]), 2, &()).unwrap()
}
fn mask(space: &Space, bits: u64) -> Event {
    space.table(3, &[bits], &()).unwrap()
}

#[test]
fn roster_constants_require_full_coverage_and_survive_reopen() {
    let dir = common::TempDir::new("event-full-roster");
    let db = Db::create(dir.path(), FullPartitions, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    let missing = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }])?;
        Ok(())
    }));
    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing.get(0).unwrap().statement_id(db.schema()),
        StatementId(4)
    );
    let gap = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }])?;
        tx.insert([&Branch {
            group: 1,
            choice: 1,
            when: mask(&source, 0b0111),
        }])?;
        Ok(())
    }));
    assert_eq!(gap.len(), 1);
    db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }])?;
        tx.insert([
            &Branch {
                group: 1,
                choice: 1,
                when: mask(&source, 0b0101),
            },
            &Branch {
                group: 1,
                choice: 2,
                when: mask(&source, 0b1010),
            },
            &Branch {
                group: 1,
                choice: 3,
                when: source.empty(),
            },
        ])?;
        assert!(tx.get(RosterByGroupTrue { group: 1 })?.is_some());
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);
    let reopened = Db::open(dir.path(), FullPartitions, common::work()).unwrap();
    reopened
        .read(common::work(), |snapshot| {
            assert!(snapshot.get(RosterByGroupTrue { group: 1 })?.is_some());
            let branches: Vec<Branch> = snapshot.scan_facts()?.collect::<bumbledb::Result<_>>()?;
            assert_eq!(branches.len(), 3);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        bumbledb::schema::render::render(reopened.schema(), StatementId(1)),
        "Roster(group, true) -> Roster"
    );
}

#[test]
fn a_full_key_gives_scalar_uniqueness_without_a_stored_event_column() {
    bumbledb::schema! {
        pub FullKey;
        relation Slot { owner: u64, payload: u64 }
        Slot(owner, true) -> Slot;
    }
    let dir = common::TempDir::new("event-full-key");
    let db = Db::create(dir.path(), FullKey, common::work())
        .unwrap()
        .unwrap();
    let rejected = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([
            &Slot {
                owner: 7,
                payload: 1,
            },
            &Slot {
                owner: 7,
                payload: 2,
            },
        ])?;
        Ok(())
    }));
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected.cited_facts(0).len(), 2);
}

#[test]
fn two_full_sides_preserve_actual_roster_presence() {
    bumbledb::schema! {
        pub FullMirror;
        relation Left { id: u64 }
        relation Right { id: u64 }
        Left(id, true) -> Left;
        Right(id, true) -> Right;
        Left(id, true) == Right(id, true);
    }
    let dir = common::TempDir::new("event-full-mirror");
    let db = Db::create(dir.path(), FullMirror, common::work())
        .unwrap()
        .unwrap();
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Left { id: 1 }])?;
        Ok(())
    }));
    db.write(common::work(), |tx| {
        tx.insert([&Left { id: 1 }])?;
        tx.insert([&Right { id: 1 }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
}

#[test]
fn full_only_projection_is_a_global_singleton_key() {
    bumbledb::schema! {
        pub Singleton;
        relation Config { mode: u64 }
        Config(true) -> Config;
    }
    let dir = common::TempDir::new("event-full-only");
    let db = Db::create(dir.path(), Singleton, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Config { mode: 1 }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    db.read(common::work(), |snapshot| {
        assert_eq!(snapshot.get(ConfigByTrue {})?.unwrap().mode, 1);
        Ok(())
    })
    .unwrap();
    let rejected = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Config { mode: 2 }])?;
        Ok(())
    }));
    assert_eq!(rejected.cited_facts(0)[0].values(), &[Value::U64(1)]);
}

#[test]
fn full_partitions_match_bitsets_on_arbitrary_nonempty_supports() {
    use bumbledb::Admission;
    let dir = common::TempDir::new("event-full-oracle");
    let db = Db::create(dir.path(), FullPartitions, common::work())
        .unwrap()
        .unwrap();
    let base = source();
    for support in [0b1111, 0b0110, 0b0001] {
        let space = base.restrict(&mask(&base, support), &()).unwrap();
        for a in 0..16 {
            for b in 0..16 {
                let roster = Roster { group: 1 };
                let left = Branch {
                    group: 1,
                    choice: 1,
                    when: mask(&space, a),
                };
                let right = Branch {
                    group: 1,
                    choice: 2,
                    when: mask(&space, b),
                };
                let admitted = db
                    .write(common::work(), |tx| {
                        tx.insert([&roster])?;
                        tx.insert([&left, &right])?;
                        Ok(())
                    })
                    .unwrap();
                let expected = (a & b & support) == 0 && ((a | b) & support) == support;
                assert_eq!(
                    matches!(admitted, Admission::Accepted(_)),
                    expected,
                    "support={support:04b}, a={a:04b}, b={b:04b}"
                );
                if expected {
                    db.write(common::work(), |tx| {
                        tx.delete([&roster])?;
                        tx.delete([&left, &right])?;
                        Ok(())
                    })
                    .unwrap()
                    .unwrap();
                }
            }
        }
    }
}

#[test]
fn contextual_full_checks_all_event_owners_even_behind_empty_operands() {
    bumbledb::schema! {
        pub Contextual;
        relation Roster { group: u64 }
        relation Claim { group: u64, item: u64, when: event }
        Roster(group, true) -> Roster;
        Claim(group, when) <= Roster(group, true);
    }
    let dir = common::TempDir::new("event-full-context");
    let db = Db::create(dir.path(), Contextual, common::work())
        .unwrap()
        .unwrap();
    let base = source();
    let foreign = Space::new(SpaceId([84; 32]), 2, &()).unwrap();
    for has_roster in [false, true] {
        let error = db
            .write(common::work(), |tx| {
                if has_roster {
                    tx.insert([&Roster { group: 1 }])?;
                }
                tx.insert([
                    &Claim {
                        group: 1,
                        item: 1,
                        when: base.empty(),
                    },
                    &Claim {
                        group: 1,
                        item: 2,
                        when: foreign.empty(),
                    },
                ])?;
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(
            error,
            bumbledb::Error::Event(bumbledb::event::Error::SpaceMismatch)
        ));
    }
    // Different scalar groups need no common world space.
    db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }, &Roster { group: 2 }])?;
        tx.insert([
            &Claim {
                group: 1,
                item: 1,
                when: base.full(),
            },
            &Claim {
                group: 1,
                item: 2,
                when: base.empty(),
            },
            &Claim {
                group: 2,
                item: 3,
                when: foreign.full(),
            },
        ])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
}

#[test]
fn full_coverage_observes_atomic_repairs_and_contributor_deletion() {
    let dir = common::TempDir::new("event-full-repair");
    let db = Db::create(dir.path(), FullPartitions, common::work())
        .unwrap()
        .unwrap();
    let space = source();
    let old = Branch {
        group: 1,
        choice: 1,
        when: space.full(),
    };
    db.write(common::work(), |tx| {
        tx.insert([&Roster { group: 1 }])?;
        tx.insert([&old])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let left = Branch {
        group: 1,
        choice: 2,
        when: mask(&space, 0b0101),
    };
    let right = Branch {
        group: 1,
        choice: 3,
        when: mask(&space, 0b1010),
    };
    db.write(common::work(), |tx| {
        tx.insert([&left, &right])?;
        tx.delete([&old])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let generation = db.generation(common::work()).unwrap();
    let rejected = common::expect_rejected(db.write(common::work(), |tx| {
        tx.delete([&left])?;
        Ok(())
    }));
    assert_eq!(rejected.len(), 1);
    assert_eq!(
        rejected.get(0).unwrap().statement_id(db.schema()),
        StatementId(4)
    );
    assert_eq!(db.generation(common::work()).unwrap(), generation);
}

#[test]
fn exact_full_keys_and_typed_prefixes_are_required() {
    use bumbledb::error::{SchemaError, StatementErrorKind};
    use bumbledb::schema::{
        FieldId, Projection, StatementDescriptor, Theory as _, ValidateDescriptor as _,
    };
    let descriptor = FullPartitions.descriptor();
    let mut ordinary_key_only = descriptor.clone();
    ordinary_key_only.statements.remove(1);
    let error = ordinary_key_only.validate().unwrap_err();
    assert!(matches!(
        error,
        SchemaError::Statement {
            kind: StatementErrorKind::NoPointwiseTargetKey { .. },
            ..
        }
    ));
    assert!(error.to_string().contains("true"));
    let mut non_scalar_prefix = descriptor;
    non_scalar_prefix.statements[2] = StatementDescriptor::Functionality {
        relation: FullPartitions::BRANCH,
        projection: Projection::EventFull(Box::new([FieldId(2)])),
    };
    assert!(matches!(
        non_scalar_prefix.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::FullProjectionNonScalar { .. },
            ..
        })
    ));
}

#[test]
fn full_constants_preserve_scalar_permutations_and_empty_lookup_uniqueness() {
    bumbledb::schema! {
        pub Permuted;
        relation Parent { a: u64, b: u64 }
        relation Child { x: u64, y: u64, when: event }
        Parent(b, a, true) -> Parent;
        Child(x, y, true) -> Child;
        Child(x, y, when) -> Child;
        Child(x, y, when) <= Parent(a, b, true);
    }
    let dir = common::TempDir::new("event-full-permuted");
    let db = Db::create(dir.path(), Permuted, common::work())
        .unwrap()
        .unwrap();
    let space = source();
    db.write(common::work(), |tx| {
        tx.insert([&Parent { a: 5, b: 7 }])?;
        tx.insert([&Child {
            x: 5,
            y: 7,
            when: space.empty(),
        }])?;
        assert!(tx.get(ParentByBATrue { b: 7, a: 5 })?.is_some());
        assert!(
            tx.get(ChildByXYWhen {
                x: 5,
                y: 7,
                when: space.empty()
            })?
            .is_some()
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Child {
            x: 7,
            y: 5,
            when: space.full(),
        }])?;
        Ok(())
    }));
}

#[test]
fn ordinary_event_containments_keep_permuted_region_positions() {
    bumbledb::schema! {
        pub Reordered;
        relation Parent { id: u64, when: event }
        relation Child { id: u64, when: event }
        Parent(id, when) -> Parent;
        Child(when, id) <= Parent(when, id);
    }
    let dir = common::TempDir::new("event-full-field-order");
    let db = Db::create(dir.path(), Reordered, common::work())
        .unwrap()
        .unwrap();
    let space = source();
    db.write(common::work(), |tx| {
        tx.insert([&Parent {
            id: 7,
            when: space.full(),
        }])?;
        tx.insert([&Child {
            id: 7,
            when: mask(&space, 5),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Child {
            id: 8,
            when: space.full(),
        }])?;
        Ok(())
    }));
}

#[test]
fn closed_scalar_full_targets_and_ground_full_laws_work() {
    use bumbledb::schema::{Theory as _, ValidateDescriptor as _};
    use closed_fixtures::{Claim, Closed, Ground};

    let dir = common::TempDir::new("event-full-closed");
    let db = Db::create(dir.path(), Closed, common::work())
        .unwrap()
        .unwrap();
    let space = source();
    db.write(common::work(), |tx| {
        tx.insert([&Claim {
            label: 7,
            when: space.full(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Claim {
            label: 8,
            when: space.full(),
        }])?;
        Ok(())
    }));

    Ground.descriptor().validate().unwrap();
    let mut refuted = Ground.descriptor();
    refuted.relations[1].extension.as_mut().unwrap()[0].values[0] = Value::U64(8);
    assert!(matches!(
        refuted.validate(),
        Err(bumbledb::error::SchemaError::Statement {
            kind: bumbledb::error::StatementErrorKind::ClosedStatementRefuted { .. },
            ..
        })
    ));
}

#[allow(dead_code)]
mod closed_fixtures {
    bumbledb::schema! {
        pub Closed;
        closed relation Allowed as AllowedId { label: u64 } = {
            First { label: 7 },
            Second { label: 9 },
        };
        relation Claim { label: u64, when: event }
        Allowed(label, true) -> Allowed;
        Claim(label, when) <= Allowed(label, true);
    }
    bumbledb::schema! {
        pub Ground;
        closed relation A as AId { label: u64 } = { One { label: 7 } };
        closed relation B as BId { label: u64 } = { One { label: 7 } };
        B(label, true) -> B;
        A(label, true) <= B(label, true);
    }
}
