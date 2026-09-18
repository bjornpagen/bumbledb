use bumbledb::{
    AnswerValue, BindValue, Db, Event, Fact, WorkContext,
    event::{CoordinateMap, Error, FibreProduct, FixedPointLimits, Space, SpaceId, WorldRelation},
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::FieldId,
};

mod common;

bumbledb::schema! {
    pub FixedPointSchema;
    relation Answer { id: u64, when: event }
    relation Expected { id: u64, when: event }
    Answer(id) -> Answer;
    Expected(id) -> Expected;
}

fn equality_join() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: [Answer::RELATION, Expected::RELATION]
            .into_iter()
            .map(|source| Atom {
                source: AtomSource::Edb(source),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            })
            .collect(),
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn fixed_point_results_persist_join_and_retain_exact_owned_values() {
    let directory = common::TempDir::new("event-fixed-point-results");
    let db = Db::create(directory.path(), FixedPointSchema, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([1; 32]), 2, &()).unwrap();
    let environment = Space::new(SpaceId([2; 32]), 0, &()).unwrap();
    let base = CoordinateMap::new(&source, &environment, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pairs = FibreProduct::new(SpaceId([3; 32]), &base, &base, &()).unwrap();
    // 0→1; 1→1 or 2; 2→3; 3 terminates. The cycle can postpone the goal forever.
    let edges = (1 << 4) | (1 << 5) | (1 << 9) | (1 << 14);
    let relation = WorldRelation::new(
        &pairs,
        &pairs.space().table(15, &[edges], &()).unwrap(),
        &(),
    )
    .unwrap();
    let goal = source.table(3, &[8], &()).unwrap();
    let safe = source.table(3, &[14], &()).unwrap();
    let limits = FixedPointLimits::default();
    let answers = [
        relation.can_reach(&goal, limits, &common::work()).unwrap(),
        relation
            .inevitably_reach(&goal, limits, &common::work())
            .unwrap(),
        relation
            .safe_throughout(&safe, limits, &common::work())
            .unwrap(),
    ];
    db.write(common::work(), |transaction| {
        for (index, (answer, expected)) in answers.iter().zip([15, 12, 14]).enumerate() {
            let independent = Event::from_bytes(
                &source
                    .table(3, &[expected], &())
                    .unwrap()
                    .to_bytes(&())
                    .unwrap(),
                &(),
            )
            .unwrap();
            transaction.insert([&Answer {
                id: index as u64,
                when: answer.event().clone(),
            }])?;
            transaction.insert([&Expected {
                id: index as u64,
                when: independent,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let cancelled = WorkContext::new();
    cancelled.cancel();
    assert_eq!(
        relation.can_reach(&goal, limits, &cancelled).unwrap_err(),
        Error::Cancelled
    );
    drop((
        db,
        source,
        environment,
        base,
        pairs,
        relation,
        goal,
        safe,
        answers,
    ));
    let db = Db::open(directory.path(), FixedPointSchema, common::work()).unwrap();
    let query = equality_join();
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 3);
        retained.push(answers);
    }
    drop(db);
    for answers in retained {
        for row in 0..answers.len() {
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("answer id")
            };
            let AnswerValue::Event(event) = answers.get(row, 1) else {
                panic!("owned Event")
            };
            let expected = [15, 12, 14][usize::try_from(id).unwrap()];
            for world in 0..4 {
                assert_eq!(event.contains(world).unwrap(), expected & (1 << world) != 0);
            }
        }
    }
}
