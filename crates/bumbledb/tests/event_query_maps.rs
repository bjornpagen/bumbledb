//! Readout query heads compared with explicit legal fibres, not the native map oracle.
use bumbledb::{
    AnswerValue, BindValue, Db, Error, Event, EventExpr, EventExprError, EventImport, FindTerm,
    VarId,
    event::{
        AdmittedDescriptor, CoordinateMap, DescriptorLimits, FibreProduct, MapOp, Space, SpaceId,
    },
    query,
};

mod common;

bumbledb::schema! {
    pub ReadoutQueries;
    relation Source { id: u64, when: event }
    relation Target { id: u64, when: event }
    Source(id) -> Source;
    Target(id) -> Target;
}

fn capture(map: CoordinateMap) -> EventImport {
    EventImport::capture(
        &AdmittedDescriptor::Map(map),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap()
}

fn mask(value: &Event, worlds: u64) -> u64 {
    (0..worlds)
        .filter(|&w| value.contains(w).unwrap_or(false))
        .fold(0, |bits, w| bits | (1 << w))
}

#[test]
fn imported_readouts_match_each_legal_fibre_and_keep_independent_owners() {
    for support in [15u64, 7, 5] {
        let directory = common::TempDir::new("event-readout-heads");
        let db = Db::create(directory.path(), ReadoutQueries, common::work())
            .unwrap()
            .unwrap();
        let raw = Space::new(SpaceId([1; 32]), 2, &()).unwrap();
        let source = raw
            .restrict(&raw.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        let target = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
        let map = capture(CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap());
        let independent =
            EventImport::from_bytes(map.bytes(), DescriptorLimits::default(), &()).unwrap();
        db.write(common::work(), |tx| {
            for bits in 0..16 {
                if bits & !support == 0 {
                    tx.insert([&Source {
                        id: bits,
                        when: source.table(3, &[bits], &())?,
                    }])?;
                }
            }
            for bits in 0..4 {
                tx.insert([&Target {
                    id: bits,
                    when: target.table(1, &[bits], &())?,
                }])?;
            }
            Ok(())
        })
        .unwrap()
        .unwrap();
        let template = query!(ReadoutQueries {
            use map observation = &map;
            use map restored = &independent;
            (s, t,
             image: Event(Image(a, observation)),
             all: Event(UniversalImage(a, observation)),
             must: Event(NonvacuousImage(a, observation)),
             possible: Event(Possible(a, observation)),
             guaranteed: Event(Guaranteed(a, observation)),
             back: Event(Pullback(b, observation)),
             selected: Event(Possible(a, observation) & Pullback(b, observation)),
             duplicate: Event(Exactly(1, Image(a, observation), Image(a, restored))),
             choice: Event(Ite(Image(a, observation), b, !b)),
             subset: Test(Subset(Image(a, observation), b))) |
                Source(id: s, when: a), Target(id: t, when: b);
        });
        drop((map, independent, source, target, raw));
        let mut retained = Vec::new();
        for cursor in [false, true] {
            let mut prepared = db.prepare(&template, common::work()).unwrap();
            prepared.force_cursor_fallback(cursor);
            retained.push(
                db.read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap(),
            );
        }
        drop((template, db));
        for answers in retained {
            assert_eq!(answers.len(), (1 << support.count_ones()) * 4);
            for row in 0..answers.len() {
                let (AnswerValue::U64(a), AnswerValue::U64(b)) =
                    (answers.get(row, 0), answers.get(row, 1))
                else {
                    panic!("scalar ids");
                };
                let (image, expected) = fibre_oracle(a, b, support);
                for (column, (expected, worlds)) in expected.into_iter().enumerate() {
                    let AnswerValue::Event(value) = answers.get(row, column + 2) else {
                        panic!("Event");
                    };
                    assert_eq!(
                        mask(value, worlds),
                        expected,
                        "support={support}, a={a}, b={b}, output={column}"
                    );
                }
                assert_eq!(answers.get(row, 11), AnswerValue::Bool(image & !b == 0));
            }
        }
    }
}

#[test]
fn typed_boundaries_validate_each_occurrence_even_when_one_variable_changes_role() {
    let directory = common::TempDir::new("event-readout-faults");
    let db = Db::create(directory.path(), ReadoutQueries, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([3; 32]), 1, &()).unwrap();
    let target = Space::new(SpaceId([4; 32]), 1, &()).unwrap();
    let map = capture(CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap());
    db.write(common::work(), |tx| {
        tx.insert([&Source {
            id: 1,
            when: source.empty(),
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(ReadoutQueries {
        use map observation = &map;
        (output: Event(Image(a, observation) & Full(a))) | Source(when: a);
    });
    let mut faults = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let Error::EventFaults(values) = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err()
        else {
            panic!("context fault");
        };
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].operand, 1);
        assert_eq!(
            values[0].source,
            bumbledb::EventOperandSource::Variable(VarId(0))
        );
        assert_eq!(
            &*values[0].expected_space,
            target.full().to_bytes(&()).unwrap()
        );
        assert_eq!(
            &*values[0].offending_value,
            source.empty().to_bytes(&()).unwrap()
        );
        faults.push(values);
    }
    assert_eq!(faults[0], faults[1]);
}

#[test]
fn incompatible_static_outputs_refuse_before_empty_body_execution() {
    let directory = common::TempDir::new("event-readout-shape");
    let db = Db::create(directory.path(), ReadoutQueries, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([5; 32]), 1, &()).unwrap();
    let other = Space::new(SpaceId([6; 32]), 1, &()).unwrap();
    let one = capture(CoordinateMap::coordinates(&source, &source, &[0], &()).unwrap());
    let two = capture(CoordinateMap::coordinates(&source, &other, &[0], &()).unwrap());
    let template = query!(ReadoutQueries {
        use map one = &one;
        use map two = &two;
        (output: Event(Image(a, one) | Image(a, two))) | Source(when: a);
    });
    assert!(matches!(template.rules()[0].finds[0], FindTerm::Event(_)));
    let FindTerm::Event(expr) = &template.rules()[0].finds[0] else {
        unreachable!()
    };
    assert_eq!(
        expr.validate_shape(),
        Err(EventExprError::IncompatibleContexts)
    );
    assert!(db.prepare(&template, common::work()).is_err());
    let expression = EventExpr::Map {
        operation: MapOp::Pullback,
        map: one,
        input: Box::new(EventExpr::Var(VarId(0))),
    };
    assert!(expression.validate_shape().is_ok());
}

#[test]
fn map_imports_compose_with_template_imports_interiors_and_event_pack() {
    let directory = common::TempDir::new("event-readout-staging");
    let db = Db::create(directory.path(), ReadoutQueries, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([7; 32]), 1, &()).unwrap();
    let target = Space::new(SpaceId([8; 32]), 1, &()).unwrap();
    let observation = capture(CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap());
    db.write(common::work(), |tx| {
        tx.insert([
            &Source {
                id: 1,
                when: source.coordinate(0, &())?,
            },
            &Source {
                id: 2,
                when: source.coordinate(0, &())?.complement(),
            },
        ])
    })
    .unwrap()
    .unwrap();
    let seed = query!(ReadoutQueries { (a) | Source(when: a); });
    let template = query!(ReadoutQueries {
        use map observation = &observation;
        use map = &seed;
        interior viewed(out: Event(Image(a, observation))) | map(a);
        interior combined(out: Pack(a)) | viewed(a);
        (out: Event(!a), sure: Test(IsFull(a))) | combined(a);
    });
    // Capture/query identity and diagnostics do not print arena internals.
    let restored =
        EventImport::from_bytes(observation.bytes(), DescriptorLimits::default(), &()).unwrap();
    assert_eq!(restored, observation);
    assert_eq!(format!("{restored:?}"), format!("{observation:?}"));
    assert!(format!("{observation:?}").len() < 150);
    drop((seed, observation, restored, source));
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    drop((template, prepared, db));
    assert_eq!(answers.len(), 1);
    let AnswerValue::Event(value) = answers.get(0, 0) else {
        panic!("Event");
    };
    assert!(value.is_empty());
    assert_eq!(value.space().identity(), target.identity());
    assert_eq!(answers.get(0, 1), AnswerValue::Bool(true));
}

#[test]
fn map_heads_accept_onto_maps_and_refuse_other_checked_descriptor_kinds() {
    let source = Space::new(SpaceId([9; 32]), 1, &()).unwrap();
    let environment = Space::new(SpaceId([10; 32]), 0, &()).unwrap();
    let onto = CoordinateMap::coordinates(&source, &environment, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = FibreProduct::new(SpaceId([11; 32]), &onto, &onto, &()).unwrap();
    for (descriptor, expected) in [
        (AdmittedDescriptor::Surjective(onto), Ok(())),
        (
            AdmittedDescriptor::Fibre(pair),
            Err(EventExprError::ImportKind),
        ),
    ] {
        let map = EventImport::capture(&descriptor, DescriptorLimits::default(), &()).unwrap();
        let expr = EventExpr::Map {
            operation: MapOp::Possible,
            map,
            input: Box::new(EventExpr::Var(VarId(0))),
        };
        assert_eq!(expr.validate_shape(), expected);
    }
}

fn fibre_oracle(a: u64, b: u64, support: u64) -> (u64, [(u64, u64); 9]) {
    let mut image = 0;
    let mut all = 0;
    let mut inhabited = 0;
    for target in 0..2 {
        let fibre = (0..4)
            .filter(|w| w & 1 == target && support & (1 << w) != 0)
            .fold(0, |bits, w| bits | (1 << w));
        if fibre & a != 0 {
            image |= 1 << target;
        }
        if fibre & !a == 0 {
            all |= 1 << target;
        }
        if fibre != 0 {
            inhabited |= 1 << target;
        }
    }
    let pullback = |bits| {
        (0..4)
            .filter(|w| support & (1 << w) != 0 && bits & (1 << (w & 1)) != 0)
            .fold(0, |out, w| out | (1 << w))
    };
    let expected = [
        (image, 2),
        (all, 2),
        (all & inhabited, 2),
        (pullback(image), 4),
        (pullback(all), 4),
        (pullback(b), 4),
        (pullback(image) & pullback(b), 4),
        (0, 2),
        ((image & b) | (!image & !b & 3), 2),
    ];
    (image, expected)
}
