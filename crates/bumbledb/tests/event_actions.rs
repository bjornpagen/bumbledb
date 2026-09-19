use bumbledb::{
    AnswerValue, BindValue, Db, Fact, WorkContext,
    event::{
        ActionArena, ActionDescriptor, ActionDescriptorLimits, AdmittedActionDescriptor,
        ArithmeticLimits, CoordinateMap, Error, ExactArithmetic, FibreProduct, FixedPointLimits,
        PartitionLimits, Space, SpaceId, WorldRelation,
    },
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::FieldId,
};

mod common;

bumbledb::schema! {
    pub StrategySchema;
    relation Winning { id: u64, when: event }
    relation Rank { id: u64, rank: u64, when: event }
    relation Policy { id: u64, when: event }
    relation Expected { id: u64, when: event }
    Winning(id) -> Winning;
    Winning(id, when) -> Winning;
    Rank(id, rank) -> Rank;
    Rank(id, when) -> Rank;
    Winning(id, when) == Rank(id, when);
    Policy(id) -> Policy;
    Expected(id) -> Expected;
}

fn arena() -> ActionArena {
    let states = Space::new(SpaceId([1; 32]), 1, &()).unwrap();
    let choices = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let env = Space::new(SpaceId([3; 32]), 0, &()).unwrap();
    let s = CoordinateMap::new(&states, &env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let a = CoordinateMap::new(&choices, &env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let sa = FibreProduct::new(SpaceId([4; 32]), &s, &a, &()).unwrap();
    let base = CoordinateMap::new(sa.space(), &env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let step = FibreProduct::new(SpaceId([5; 32]), &base, &s, &()).unwrap();
    let relation = WorldRelation::new(
        &step,
        &step.space().table(7, &[0b1110_0001], &()).unwrap(),
        &(),
    )
    .unwrap();
    ActionArena::new(&sa, &relation, &()).unwrap()
}

fn policy_join() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: [Policy::RELATION, Expected::RELATION]
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
fn strategy_regions_ranks_and_policy_persist_as_ordinary_event_facts() {
    let directory = common::TempDir::new("event-strategy-results");
    let db = Db::create(directory.path(), StrategySchema, common::work())
        .unwrap()
        .unwrap();
    let arena = arena();
    let goal = arena.states().coordinate(0, &()).unwrap();
    let strategy = arena
        .winning_reach(
            &goal,
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &common::work(),
        )
        .unwrap();
    let limits = ActionDescriptorLimits::default();
    let recipe = ActionDescriptor::capture(
        &AdmittedActionDescriptor::Reach(strategy),
        limits.descriptors,
        &(),
    )
    .unwrap();
    let bytes = recipe.to_bytes(limits.descriptors, &()).unwrap();
    drop(recipe);
    let AdmittedActionDescriptor::Reach(strategy) = ActionDescriptor::import(
        &bytes,
        limits,
        &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
    )
    .unwrap() else {
        panic!("checked reach strategy")
    };
    db.write(common::work(), |tx| {
        tx.insert([&Winning {
            id: 1,
            when: strategy.winning().clone(),
        }])?;
        for (rank, when) in strategy.ranked().layers().cells().iter().enumerate() {
            tx.insert([&Rank {
                id: 1,
                rank: rank as u64,
                when: when.clone(),
            }])?;
        }
        tx.insert([&Policy {
            id: 1,
            when: strategy.policy().region().clone(),
        }])?;
        tx.insert([&Expected {
            id: 1,
            when: arena.actions().space().table(3, &[4], &())?,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let cancelled = WorkContext::new();
    cancelled.cancel();
    assert_eq!(
        arena
            .winning_safe(&goal, FixedPointLimits::default(), &cancelled)
            .unwrap_err(),
        Error::Cancelled
    );
    drop((db, arena, goal, strategy));
    let db = Db::open(directory.path(), StrategySchema, common::work()).unwrap();
    let mut ranks: Vec<Rank> = db
        .read(common::work(), |snapshot| snapshot.scan_facts()?.collect())
        .unwrap();
    ranks.sort_by_key(|row| row.rank);
    assert_eq!(ranks.len(), 2);
    assert!(ranks[0].when.contains(1).unwrap());
    assert!(ranks[1].when.contains(0).unwrap());
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&policy_join(), common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 1);
        retained.push(answers);
    }
    drop(db);
    for answers in retained {
        let AnswerValue::Event(policy) = answers.get(0, 1) else {
            panic!("owned policy region")
        };
        assert_eq!(policy.count(&()).unwrap(), 1);
        assert!(policy.contains(2).unwrap());
        assert!(!policy.contains(0).unwrap());
    }
}
