use bumbledb::{
    AnswerValue, BindValue, Db,
    event::{
        ArithmeticLimits, BeliefArena, BeliefArenaDescriptor, BeliefDescriptor,
        BeliefDescriptorLimits, BeliefLimits, BeliefMemory, BeliefSpaceIds, CoordinateMap,
        EventPartition, ExactArithmetic, FibreProduct, FixedPointLimits, PartitionLimits, Space,
        SpaceId, WorldRelation,
    },
};
mod common;

bumbledb::schema! {
    pub MemorySchema;
    relation Memory { game: u64, id: u64, possible: event }
    relation MemoryCode { game: u64, id: u64, when: event }
    relation MemoryDomain { game: u64, when: event }
    relation Next { game: u64, from: u64, action: u64, observation: u64, to: u64 }
    relation Winning { game: u64, when: event }
    relation Policy { game: u64, when: event }
    relation ExpectedPolicy { game: u64, when: event }
    Memory(game, id) -> Memory;
    MemoryCode(game, id) -> MemoryCode;
    MemoryCode(game, when) -> MemoryCode;
    MemoryCode(game, id) == Memory(game, id);
    MemoryDomain(game) -> MemoryDomain;
    MemoryDomain(game, when) -> MemoryDomain;
    MemoryDomain(game, when) == MemoryCode(game, when);
    Next(game, from, action, observation) -> Next;
    Next(game, from) <= Memory(game, id);
    Next(game, to) <= Memory(game, id);
    Winning(game) -> Winning;
    Winning(game, when) <= MemoryCode(game, when);
    Policy(game) -> Policy;
    ExpectedPolicy(game) -> ExpectedPolicy;
}

fn game() -> BeliefArena {
    let hidden = Space::new(SpaceId([51; 32]), 1, &()).unwrap();
    let env = Space::new(SpaceId([52; 32]), 0, &()).unwrap();
    let base = CoordinateMap::new(&hidden, &env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pairs = FibreProduct::new(SpaceId([53; 32]), &base, &base, &()).unwrap();
    // A blind actor can always reset to state 1; guessing is not required.
    let reset =
        WorldRelation::new(&pairs, &pairs.space().table(3, &[12], &()).unwrap(), &()).unwrap();
    let observation = EventPartition::on(
        &hidden.full(),
        &[hidden.full()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    let memory = BeliefMemory::new(
        &[reset],
        &observation,
        &hidden.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let limits = BeliefDescriptorLimits::default();
    let recipe = BeliefDescriptor::capture(&memory, limits.descriptors, &())
        .unwrap()
        .to_bytes(limits.descriptors, &())
        .unwrap();
    drop(memory);
    BeliefDescriptor::import(
        &recipe,
        limits,
        &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
    )
    .unwrap()
    .arena(
        BeliefSpaceIds {
            states: SpaceId([54; 32]),
            actions: SpaceId([55; 32]),
            environment: SpaceId([56; 32]),
            state_actions: SpaceId([57; 32]),
            transitions: SpaceId([58; 32]),
        },
        &(),
    )
    .unwrap()
}

fn seed(db: &Db<MemorySchema>) {
    let original = game();
    let limits = BeliefDescriptorLimits::default();
    let portable = BeliefArenaDescriptor::capture(&original, limits.descriptors, &())
        .unwrap()
        .to_bytes(limits.descriptors, &())
        .unwrap();
    drop(original);
    let compiled = BeliefArenaDescriptor::import(
        &portable,
        limits,
        &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
    )
    .unwrap();
    let hidden_goal = compiled
        .memory()
        .given()
        .space()
        .coordinate(0, &())
        .unwrap();
    let goal = compiled.known(&hidden_goal, &()).unwrap();
    let strategy = compiled
        .arena()
        .winning_reach(
            &goal,
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    let independent = game();
    let expected = independent
        .arena()
        .winning_reach(
            &independent.known(&hidden_goal, &()).unwrap(),
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&MemoryDomain {
            game: 1,
            when: compiled.arena().states().full(),
        }])?;
        for (id, state) in compiled.memory().states().iter().enumerate() {
            tx.insert([&Memory {
                game: 1,
                id: id as u64,
                possible: state.possible().clone(),
            }])?;
            tx.insert([&MemoryCode {
                game: 1,
                id: id as u64,
                when: compiled.arena().states().table(1, &[1 << id], &())?,
            }])?;
            for edge in state.transitions() {
                tx.insert([&Next {
                    game: 1,
                    from: id as u64,
                    action: edge.action as u64,
                    observation: edge.observation as u64,
                    to: edge.target as u64,
                }])?;
            }
        }
        tx.insert([&Winning {
            game: 1,
            when: strategy.winning().clone(),
        }])?;
        tx.insert([&Policy {
            game: 1,
            when: strategy.policy().region().clone(),
        }])?;
        tx.insert([&ExpectedPolicy {
            game: 1,
            when: expected.policy().region().clone(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
}

#[test]
fn belief_memory_fits_ordinary_fds_containments_and_owned_free_join_results() {
    let path = common::TempDir::new("event-belief-memory");
    let db = Db::create(path.path(), MemorySchema, common::work())
        .unwrap()
        .unwrap();
    seed(&db);
    drop(db);
    let db = Db::open(path.path(), MemorySchema, common::work()).unwrap();
    let query = bumbledb::query!(MemorySchema {
        (game, from, to, possible, policy) |
            Next(game, from, action == 0, observation == 0, to),
            Memory(game: game, id: to, possible),
            Policy(game: game, when: policy), ExpectedPolicy(game: game, when: policy);
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 2);
        retained.push(answers);
    }
    drop(db);
    for answers in retained {
        for row in 0..2 {
            let AnswerValue::Event(possible) = answers.get(row, 3) else {
                panic!("hidden possibility Event");
            };
            let AnswerValue::Event(policy) = answers.get(row, 4) else {
                panic!("memory policy Event");
            };
            assert_eq!(possible.count(&()).unwrap(), 1);
            assert!(possible.contains(1).unwrap());
            assert_eq!(policy.count(&()).unwrap(), 1);
            assert!(policy.contains(0).unwrap()); // Reset at the initially uncertain memory.
            assert!(!policy.contains(1).unwrap()); // Stop once the goal is known.
        }
    }
}
