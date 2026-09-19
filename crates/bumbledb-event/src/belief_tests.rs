#![allow(clippy::too_many_lines)]
use crate::*;
use std::cell::Cell;

fn space(id: u8, bits: u8) -> Space {
    Space::new(SpaceId([id; 32]), bits, &()).unwrap()
}
fn table(s: &Space, bits: u64) -> Event {
    s.table((1 << s.dimensions()) - 1, &[bits], &()).unwrap()
}
fn base(s: &Space, env: &Space) -> SurjectiveMap {
    CoordinateMap::new(s, env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap()
}
fn pairs(s: &Space, id: u8) -> FibreProduct {
    let e = space(190, 0);
    FibreProduct::new(SpaceId([id; 32]), &base(s, &e), &base(s, &e), &()).unwrap()
}
fn relation(p: &FibreProduct, mask: u64) -> WorldRelation {
    WorldRelation::new(p, &table(p.space(), mask), &()).unwrap()
}
fn partition(s: &Space, cells: &[u64]) -> EventPartition {
    EventPartition::on(
        &s.full(),
        &cells.iter().map(|&b| table(s, b)).collect::<Vec<_>>(),
        PartitionLimits::default(),
        &(),
    )
    .unwrap()
}
fn mask(event: &Event, worlds: u64) -> u64 {
    (0..worlds)
        .filter(|&s| event.contains(s).unwrap())
        .fold(0, |m, s| m | (1 << s))
}
fn ids() -> BeliefSpaceIds {
    BeliefSpaceIds {
        states: SpaceId([181; 32]),
        actions: SpaceId([182; 32]),
        environment: SpaceId([183; 32]),
        state_actions: SpaceId([184; 32]),
        transitions: SpaceId([185; 32]),
    }
}

fn replay(memory: &BeliefMemory) -> BeliefMemory {
    let limits = BeliefDescriptorLimits::default();
    let data = BeliefDescriptor::capture(memory, limits.descriptors, &()).unwrap();
    let bytes = data.to_bytes(limits.descriptors, &()).unwrap();
    let result = BeliefDescriptor::import(
        &bytes,
        limits,
        &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
    )
    .unwrap();
    assert_eq!(
        BeliefDescriptor::capture(&result, limits.descriptors, &()).unwrap(),
        data
    );
    assert_eq!(result.initial(), memory.initial());
    assert_eq!(result.states().len(), memory.states().len());
    for (a, b) in result.states().iter().zip(memory.states()) {
        assert_eq!(
            a.possible().to_bytes(&()).unwrap(),
            b.possible().to_bytes(&()).unwrap()
        );
        assert_eq!(a.observation(), b.observation());
        assert_eq!(a.transitions(), b.transitions());
    }
    result
}

// A separate dense graph oracle; it knows no Event operations or product maps.
fn next(edges: u64, belief: u64, observation: u64, worlds: u64) -> Option<u64> {
    let mut post = 0;
    for s in 0..worlds {
        if belief & (1 << s) == 0 {
            continue;
        }
        let mut successors = 0;
        for t in 0..worlds {
            if edges & (1 << (s + worlds * t)) != 0 {
                successors |= 1 << t;
            }
        }
        if successors == 0 {
            return None;
        }
        post |= successors;
    }
    Some(post & observation).filter(|&m| m != 0)
}

#[test]
fn every_two_state_game_matches_dense_reachable_memory_and_history_updates() {
    let s = space(170, 1);
    let p = pairs(&s, 171);
    for a in 0..16 {
        for b in 0..16 {
            let actions = [relation(&p, a), relation(&p, b)];
            for cells in [vec![3, 0], vec![1, 2]] {
                let observed = partition(&s, &cells);
                for initial in 0..4 {
                    let memory = BeliefMemory::new(
                        &actions,
                        &observed,
                        &table(&s, initial),
                        BeliefLimits::default(),
                        &(),
                    )
                    .unwrap();
                    let memory = replay(&memory);
                    let mut expected: Vec<u64> = cells
                        .iter()
                        .map(|o| o & initial)
                        .filter(|&m| m != 0)
                        .collect();
                    let mut i = 0;
                    while i < expected.len() {
                        for action in [a, b] {
                            for &cell in &cells {
                                if let Some(dest) = next(action, expected[i], cell, 2)
                                    && !expected.contains(&dest)
                                {
                                    expected.push(dest);
                                }
                            }
                        }
                        i += 1;
                    }
                    assert_eq!(
                        memory
                            .states()
                            .iter()
                            .map(|v| mask(v.possible(), 2))
                            .collect::<Vec<_>>(),
                        expected
                    );
                    for (o, index) in memory.initial().iter().enumerate() {
                        assert_eq!(
                            index.map(|i| expected[i]),
                            Some(initial & cells[o]).filter(|&v| v != 0)
                        );
                    }
                    for (state, &belief) in expected.iter().enumerate() {
                        for (action, edge) in [a, b].into_iter().enumerate() {
                            let enabled = next(edge, belief, 3, 2).is_some();
                            assert_eq!(memory.enabled(state, action).unwrap(), enabled);
                            for (observation, &cell) in cells.iter().enumerate() {
                                let result = memory.update(state, action, observation).unwrap();
                                assert_eq!(
                                    result.map(|i| expected[i]),
                                    next(edge, belief, cell, 2)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn compiled_memory_preserves_all_edges_support_and_modal_objectives() {
    let s = space(170, 1);
    let p = pairs(&s, 171);
    for a in 0..16 {
        for b in 0..16 {
            let memory = BeliefMemory::new(
                &[relation(&p, a), relation(&p, b)],
                &partition(&s, &[3]),
                &s.full(),
                BeliefLimits::default(),
                &(),
            )
            .unwrap();
            let compiled = memory.arena(ids(), &()).unwrap();
            let count = memory.states().len();
            let arena = compiled.arena();
            assert_eq!(arena.states().full().count(&()).unwrap(), count as u64);
            assert_eq!(arena.choices().full().count(&()).unwrap(), 2);
            assert_eq!(compiled.initial().count(&()).unwrap(), 1);
            let sbits = arena.states().dimensions();
            for (source, state) in memory.states().iter().enumerate() {
                for action in 0..2 {
                    for target in 0..count {
                        let code = source as u64
                            | ((action as u64) << sbits)
                            | ((target as u64) << (sbits + 1));
                        assert_eq!(
                            arena.transition().region().contains(code).unwrap(),
                            state
                                .transitions()
                                .iter()
                                .any(|e| e.action == action && e.target == target)
                        );
                    }
                }
            }
            for goal in 0..4 {
                let known = compiled.known(&table(&s, goal), &()).unwrap();
                let possible = compiled.possible(&table(&s, goal), &()).unwrap();
                for (i, state) in memory.states().iter().enumerate() {
                    let worlds = mask(state.possible(), 2);
                    assert_eq!(known.contains(i as u64).unwrap(), worlds & !goal == 0);
                    assert_eq!(possible.contains(i as u64).unwrap(), worlds & goal != 0);
                }
                let reach = arena
                    .winning_reach(
                        &known,
                        FixedPointLimits::default(),
                        PartitionLimits::default(),
                        &(),
                    )
                    .unwrap();
                let mut won = mask(&known, count as u64);
                loop {
                    let mut new = won;
                    for (i, state) in memory.states().iter().enumerate() {
                        if (0..2).any(|a| {
                            let targets: Vec<_> = state
                                .transitions()
                                .iter()
                                .filter(|e| e.action == a)
                                .collect();
                            !targets.is_empty()
                                && targets.iter().all(|e| won & (1 << e.target) != 0)
                        }) {
                            new |= 1 << i;
                        }
                    }
                    if new == won {
                        break;
                    }
                    won = new;
                }
                assert_eq!(mask(reach.winning(), count as u64), won);
            }
        }
    }
}

fn edges(worlds: u64, pairs: &[(u64, u64)]) -> u64 {
    pairs
        .iter()
        .fold(0, |m, &(s, t)| m | (1 << (s + worlds * t)))
}

#[test]
fn remembered_observation_selects_the_right_action_after_the_display_is_erased() {
    let s = space(172, 3);
    let p = pairs(&s, 173);
    // Probe distinguishes the secret, then conceal returns both possibilities
    // to the same visible screen. Winning needs the remembered probe result.
    let actions = [
        relation(&p, edges(8, &[(0, 2), (1, 3)])),
        relation(&p, edges(8, &[(2, 4), (3, 5)])),
        relation(&p, edges(8, &[(4, 6), (5, 7)])),
        relation(&p, edges(8, &[(4, 7), (5, 6)])),
    ];
    let observation = partition(&s, &[0b0011_0011, 4, 8, 64, 128, 0]);
    let memory = BeliefMemory::new(
        &actions,
        &observation,
        &table(&s, 3),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let memory = replay(&memory);
    let start = memory.initial()[0].unwrap();
    assert!(memory.initial()[5].is_none());
    let left = memory.update(start, 0, 1).unwrap().unwrap();
    let right = memory.update(start, 0, 2).unwrap().unwrap();
    let left = memory.update(left, 1, 0).unwrap().unwrap();
    let right = memory.update(right, 1, 0).unwrap().unwrap();
    assert_ne!(left, right);
    assert_eq!(
        memory.states()[left].observation(),
        memory.states()[right].observation()
    );
    assert_eq!(mask(memory.states()[left].possible(), 8), 16);
    assert_eq!(mask(memory.states()[right].possible(), 8), 32);
    let compiled = memory.arena(ids(), &()).unwrap();
    let goal = compiled.known(&table(&s, 64), &()).unwrap();
    let strategy = compiled
        .arena()
        .winning_reach(
            &goal,
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    assert!(strategy.winning().contains(start as u64).unwrap());
    assert_eq!(
        strategy
            .ranked()
            .layers()
            .locate(start as u64, &())
            .unwrap(),
        Some(3)
    );
    let bits = compiled.arena().states().dimensions();
    assert!(
        strategy
            .policy()
            .region()
            .contains(left as u64 | (2 << bits))
            .unwrap()
    );
    assert!(
        !strategy
            .policy()
            .region()
            .contains(left as u64 | (3 << bits))
            .unwrap()
    );
    assert!(
        strategy
            .policy()
            .region()
            .contains(right as u64 | (3 << bits))
            .unwrap()
    );
    // Forgetting the remembered signal really loses this game.
    let forgotten = BeliefMemory::new(
        &actions,
        &observation,
        &table(&s, 48),
        BeliefLimits::default(),
        &(),
    )
    .unwrap()
    .arena(ids(), &())
    .unwrap();
    let cannot_win = forgotten
        .arena()
        .winning_reach(
            &forgotten.known(&table(&s, 64), &()).unwrap(),
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    assert!(!cannot_win.winning().contains(0).unwrap());
    drop((s, p, memory, actions, observation, compiled));
    assert!(strategy.winning().contains(start as u64).unwrap());
}

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> Result<()> {
        let n = self.0.get();
        if n == 0 {
            return Err(Error::Cancelled);
        }
        self.0.set(n - 1);
        Ok(())
    }
}

#[test]
fn admission_ownership_unavailable_actions_and_limits_are_explicit() {
    let s = space(170, 1);
    let p = pairs(&s, 171);
    let actions = [relation(&p, 1), relation(&p, 8)];
    let observations = partition(&s, &[3]);
    let memory = BeliefMemory::new(
        &actions,
        &observations,
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    // Each world can act, but no one action is enabled throughout the belief.
    assert!(!memory.enabled(0, 0).unwrap());
    assert!(!memory.enabled(0, 1).unwrap());
    assert_eq!(memory.update(0, 0, 0).unwrap(), None);
    for (state, action, obs) in [(1, 0, 0), (0, 2, 0), (0, 0, 1)] {
        assert_eq!(
            memory.update(state, action, obs).unwrap_err(),
            Error::BeliefIndex
        );
    }
    let partial =
        EventPartition::on(&table(&s, 1), &[s.full()], PartitionLimits::default(), &()).unwrap();
    assert_eq!(
        BeliefMemory::new(&actions, &partial, &s.empty(), BeliefLimits::default(), &())
            .unwrap_err(),
        Error::PartitionGap
    );
    let foreign = space(175, 1);
    assert_eq!(
        BeliefMemory::new(
            &actions,
            &observations,
            &foreign.empty(),
            BeliefLimits::default(),
            &()
        )
        .unwrap_err(),
        Error::SpaceMismatch
    );
    let wrong = relation(&pairs(&foreign, 176), 0);
    assert_eq!(
        BeliefMemory::new(
            &[wrong],
            &observations,
            &s.empty(),
            BeliefLimits::default(),
            &()
        )
        .unwrap_err(),
        Error::SpaceMismatch
    );
    let empty = BeliefMemory::new(
        &actions,
        &observations,
        &s.empty(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    assert!(empty.states().is_empty());
    assert_eq!(empty.arena(ids(), &()).unwrap_err(), Error::EmptySpace);
    let no_actions =
        BeliefMemory::new(&[], &observations, &s.full(), BeliefLimits::default(), &()).unwrap();
    assert_eq!(no_actions.states().len(), 1);
    assert_eq!(no_actions.arena(ids(), &()).unwrap_err(), Error::EmptySpace);
    let limits = BeliefLimits::default();
    for (limit, expected) in [
        (
            BeliefLimits {
                states: 0,
                ..limits
            },
            Capacity::BeliefStates,
        ),
        (
            BeliefLimits {
                transitions: 0,
                ..limits
            },
            Capacity::BeliefTransitions,
        ),
        (BeliefLimits { steps: 0, ..limits }, Capacity::BeliefSteps),
    ] {
        assert_eq!(
            BeliefMemory::new(&[relation(&p, 9)], &observations, &s.full(), limit, &())
                .unwrap_err(),
            Error::Capacity(expected)
        );
    }
    for n in [0, 10, 40] {
        assert_eq!(
            BeliefMemory::new(
                &[relation(&p, 15)],
                &observations,
                &s.full(),
                limits,
                &Stop(Cell::new(n))
            )
            .unwrap_err(),
            Error::Cancelled
        );
    }
    assert_eq!(
        memory.arena(ids(), &Stop(Cell::new(0))).unwrap_err(),
        Error::Cancelled
    );
    // Independently decoded endpoint owners must align before symbolic splitting.
    let decoded = Event::from_bytes(&s.full().to_bytes(&()).unwrap(), &())
        .unwrap()
        .space();
    let independent = relation(&pairs(&decoded, 171), 9);
    let retained =
        BeliefMemory::new(&[independent], &observations, &s.full(), limits, &()).unwrap();
    assert_eq!(retained.update(0, 0, 0).unwrap(), Some(0));
}

#[test]
fn symbolic_hidden_state_does_not_require_world_enumeration() {
    let s = space(174, 20);
    let p = pairs(&s, 175);
    let identity = WorldRelation::identity(&p, &()).unwrap();
    let observed =
        EventPartition::on(&s.full(), &[s.full()], PartitionLimits::default(), &()).unwrap();
    let memory = BeliefMemory::new(
        &[identity],
        &observed,
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    assert_eq!(memory.states().len(), 1);
    assert_eq!(memory.states()[0].possible().count(&()).unwrap(), 1 << 20);
    assert_eq!(memory.update(0, 0, 0).unwrap(), Some(0));
    let compiled = memory.arena(ids(), &()).unwrap();
    assert_eq!(compiled.arena().states().dimensions(), 0);
    assert!(
        compiled
            .arena()
            .winning_safe(
                &compiled.arena().states().full(),
                FixedPointLimits::default(),
                &()
            )
            .unwrap()
            .winning()
            .is_full()
    );
}

#[test]
fn shared_unknown_parameter_survives_belief_updates_without_becoming_visible() {
    use crate::parameter_source_tests::{c, p, sign, space as family, sub};
    let guard = ParameterGuard {
        coordinate: 0,
        region: sign(&sub(&p(), &c(1)), PolynomialSigns::NEGATIVE),
    };
    let s = family(176, 2, std::slice::from_ref(&guard));
    let environment = family(177, 1, &[guard]);
    let env = CoordinateMap::coordinates(&s, &environment, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let product = FibreProduct::new(SpaceId([178; 32]), &env, &env, &()).unwrap();
    let g = s.coordinate(0, &()).unwrap();
    let outcome = s.coordinate(1, &()).unwrap();
    let flip = CoordinateMap::new(&s, &s, &[g.clone(), outcome.complement()], &()).unwrap();
    let transition = WorldRelation::graph(&product, &flip, &()).unwrap();
    let observation =
        EventPartition::on(&s.full(), &[s.full()], PartitionLimits::default(), &()).unwrap();
    let given = g.apply(BoolOp4::AND, &outcome.complement(), &()).unwrap();
    let memory = BeliefMemory::new(
        &[transition],
        &observation,
        &given,
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let memory = replay(&memory);
    let given = given.align_to(&memory.given().space(), &()).unwrap();
    let g = g.align_to(&memory.given().space(), &()).unwrap();
    let outcome = outcome.align_to(&memory.given().space(), &()).unwrap();
    assert_eq!(memory.states().len(), 2);
    assert_eq!(memory.states()[0].possible(), &given);
    assert_eq!(
        memory.states()[1].possible(),
        &g.apply(BoolOp4::AND, &outcome, &()).unwrap()
    );
    assert_eq!(memory.update(1, 0, 0).unwrap(), Some(0));
    // The memory node has no actual parameter coordinate. Possibilities retain
    // the original family; no scalar p was observed or drawn at a transition.
    assert!(
        memory.states()[1]
            .possible()
            .space()
            .parameter_domain()
            .is_some()
    );
    assert!(
        memory
            .arena(ids(), &())
            .unwrap()
            .arena()
            .states()
            .parameter_domain()
            .is_none()
    );
}

#[test]
fn zero_mass_worlds_still_participate_and_mixed_environments_refuse() {
    let raw = space(179, 1);
    let s = raw
        .with_density(
            &[DensityPiece {
                region: table(&raw, 1),
                density: ExactRational::one(),
            }],
            LawLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
        .unwrap();
    let p = pairs(&s, 180);
    let actions = [relation(&p, 1)]; // Disabled only in possible zero-mass world 1.
    let memory = BeliefMemory::new(
        &actions,
        &partition(&s, &[3]),
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let memory = replay(&memory);
    assert!(!memory.enabled(0, 0).unwrap());
    assert!(memory.states()[0].possible().contains(1).unwrap());
    let dead = BeliefMemory::new(
        &actions,
        &partition(&s, &[3]),
        &table(&s, 2),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    assert_eq!(dead.states().len(), 1); // Nonempty evidence of mass zero survives.

    let s = space(186, 2);
    let e = space(187, 1);
    let first = CoordinateMap::coordinates(&s, &e, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let second = CoordinateMap::coordinates(&s, &e, &[1], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let wrong = FibreProduct::new(SpaceId([188; 32]), &first, &second, &()).unwrap();
    let wrong = WorldRelation::new(&wrong, &wrong.space().empty(), &()).unwrap();
    let observed = partition(&s, &[15]);
    assert_eq!(
        BeliefMemory::new(
            &[wrong],
            &observed,
            &s.empty(),
            BeliefLimits::default(),
            &()
        )
        .unwrap_err(),
        Error::EnvironmentMismatch
    );
    let one = FibreProduct::new(SpaceId([188; 32]), &first, &first, &()).unwrap();
    let two = FibreProduct::new(SpaceId([189; 32]), &second, &second, &()).unwrap();
    let actions = [
        WorldRelation::identity(&one, &()).unwrap(),
        WorldRelation::identity(&two, &()).unwrap(),
    ];
    assert_eq!(
        BeliefMemory::new(
            &actions,
            &observed,
            &s.empty(),
            BeliefLimits::default(),
            &()
        )
        .unwrap_err(),
        Error::EnvironmentMismatch
    );
}

#[test]
fn memory_recipe_preserves_reversal_indices_and_compilation() {
    let s = space(191, 1);
    let p = pairs(&s, 192);
    // The first authored product is reversed; retaining its roles is essential.
    let actions = [relation(&p, 6).converse(), relation(&pairs(&s, 193), 9)];
    let memory = BeliefMemory::new(
        &actions,
        &partition(&s, &[0, 1, 2]),
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let data = BeliefDescriptor::capture(&memory, DescriptorLimits::default(), &()).unwrap();
    assert!(data.actions.iter().all(|a| a.product.reversed));
    assert!(
        data.actions
            .iter()
            .all(|a| a.product.identity == SpaceId([192; 32]))
    );
    let old = memory.arena(ids(), &()).unwrap();
    let restored = replay(&memory);
    drop(memory);
    drop(actions);
    drop(p);
    let new = restored.arena(ids(), &()).unwrap();
    assert_eq!(
        old.initial().to_bytes(&()).unwrap(),
        new.initial().to_bytes(&()).unwrap()
    );
    assert_eq!(
        old.arena().transition().region().to_bytes(&()).unwrap(),
        new.arena().transition().region().to_bytes(&()).unwrap()
    );
    assert_eq!(
        old.known(&s.coordinate(0, &()).unwrap(), &())
            .unwrap()
            .to_bytes(&())
            .unwrap(),
        new.known(&s.coordinate(0, &()).unwrap(), &())
            .unwrap()
            .to_bytes(&())
            .unwrap()
    );
    assert_eq!(restored.initial(), &[None, Some(0), Some(1)]);
    assert_eq!(restored.update(0, 0, 2).unwrap(), Some(1));
    assert_eq!(restored.update(0, 1, 1).unwrap(), Some(0));
    // Non-symmetric converse is observable even without a fully enabled action.
    let one_way = BeliefMemory::new(
        &[relation(&pairs(&s, 194), 2).converse()],
        &partition(&s, &[1, 2]),
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let restored = replay(&one_way);
    assert!(restored.enabled(0, 0).unwrap());
    assert!(!restored.enabled(1, 0).unwrap());
    assert_eq!(restored.update(0, 0, 1).unwrap(), Some(1));
}

#[test]
fn memory_recipe_checks_every_context_even_with_no_reachable_states() {
    let s = space(194, 1);
    let memory = BeliefMemory::new(
        &[relation(&pairs(&s, 195), 9)],
        &partition(&s, &[3, 0]),
        &s.empty(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let limits = BeliefDescriptorLimits::default();
    let data = BeliefDescriptor::capture(&memory, limits.descriptors, &()).unwrap();
    let restored = replay(&memory);
    assert!(restored.states().is_empty());
    assert!(matches!(restored.arena(ids(), &()), Err(Error::EmptySpace)));
    let admit = |d: &BeliefDescriptor| {
        d.admit(
            limits,
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
    };
    let mut bad = data.clone();
    bad.source = s.empty().to_bytes(&()).unwrap();
    assert!(matches!(admit(&bad), Err(Error::InvalidEncoding)));
    bad = data.clone();
    bad.actions[0].product.left.source = s.coordinate(0, &()).unwrap().to_bytes(&()).unwrap();
    assert!(matches!(admit(&bad), Err(Error::InvalidEncoding)));
    bad = data.clone();
    bad.observations = vec![s.empty().to_bytes(&()).unwrap()];
    assert!(matches!(admit(&bad), Err(Error::PartitionGap)));
    bad = data.clone();
    bad.observations[1] = s.full().to_bytes(&()).unwrap();
    assert!(matches!(admit(&bad), Err(Error::PartitionOverlap)));
    bad = data.clone();
    bad.actions[0].product.identity = SpaceId([196; 32]);
    assert!(admit(&bad).is_err());
    bad = data.clone();
    bad.given = space(197, 1).empty().to_bytes(&()).unwrap();
    assert!(admit(&bad).is_err());
    bad = data.clone();
    bad.actions[0]
        .product
        .left
        .readouts
        .push(s.empty().to_bytes(&()).unwrap());
    assert!(matches!(admit(&bad), Err(Error::MapArity)));
    let no_actions = BeliefMemory::new(
        &[],
        &partition(&s, &[3]),
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    assert_eq!(replay(&no_actions).states().len(), 1);
}

#[test]
fn memory_recipe_envelope_and_combined_budgets_refuse_atomically() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let s = space(198, 1);
    let memory = BeliefMemory::new(
        &[relation(&pairs(&s, 199), 9)],
        &partition(&s, &[3, 0]),
        &s.full(),
        BeliefLimits::default(),
        &(),
    )
    .unwrap();
    let limits = BeliefDescriptorLimits::default();
    let data = BeliefDescriptor::capture(&memory, limits.descriptors, &()).unwrap();
    let bytes = data.to_bytes(limits.descriptors, &()).unwrap();
    for end in 0..bytes.len() {
        assert!(BeliefDescriptor::from_bytes(&bytes[..end], limits.descriptors, &()).is_err());
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(BeliefDescriptor::from_bytes(&bad, limits.descriptors, &()).is_err());
    let mut bad = bytes.clone();
    bad[4] = 2;
    assert_eq!(
        BeliefDescriptor::from_bytes(&bad, limits.descriptors, &()).unwrap_err(),
        Error::UnsupportedVersion(2)
    );
    let minimum = (0..100)
        .find(|&items| {
            data.to_bytes(
                DescriptorLimits {
                    items,
                    ..limits.descriptors
                },
                &(),
            )
            .is_ok()
        })
        .unwrap();
    // One shared allowance includes maps and all their source/readout blobs.
    let mut repeated = data.clone();
    repeated.actions.push(data.actions[0].clone());
    let bounded = DescriptorLimits {
        items: minimum,
        ..limits.descriptors
    };
    assert!(BeliefDescriptor::from_bytes(&bytes, bounded, &()).is_ok());
    assert!(matches!(
        repeated.to_bytes(bounded, &()),
        Err(Error::Capacity(Capacity::DescriptorItems))
    ));
    assert!(matches!(
        repeated.admit(
            BeliefDescriptorLimits {
                descriptors: bounded,
                ..limits
            },
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &())
        ),
        Err(Error::Capacity(Capacity::DescriptorItems))
    ));
    let repeated_bytes = repeated.to_bytes(limits.descriptors, &()).unwrap();
    assert!(matches!(
        BeliefDescriptor::from_bytes(&repeated_bytes, bounded, &()),
        Err(Error::Capacity(Capacity::DescriptorItems))
    ));
    let bounded = DescriptorLimits {
        bytes: bytes.len() - 1,
        ..limits.descriptors
    };
    assert!(matches!(
        data.to_bytes(bounded, &()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    assert!(matches!(
        BeliefDescriptor::from_bytes(&bytes, bounded, &()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    for (policy, expected) in [
        (
            BeliefDescriptorLimits {
                partitions: PartitionLimits { cells: 1 },
                ..limits
            },
            Capacity::PartitionCells,
        ),
        (
            BeliefDescriptorLimits {
                beliefs: BeliefLimits {
                    states: 0,
                    ..limits.beliefs
                },
                ..limits
            },
            Capacity::BeliefStates,
        ),
        (
            BeliefDescriptorLimits {
                beliefs: BeliefLimits {
                    transitions: 0,
                    ..limits.beliefs
                },
                ..limits
            },
            Capacity::BeliefTransitions,
        ),
    ] {
        assert!(
            matches!(data.admit(policy, &mut ExactArithmetic::new(ArithmeticLimits::default(), &())), Err(Error::Capacity(c)) if c == expected)
        );
    }
    assert!(matches!(
        data.admit(
            limits,
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        BeliefDescriptor::from_bytes(&bytes, limits.descriptors, &Cancel),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        BeliefDescriptor::capture(&memory, limits.descriptors, &Cancel),
        Err(Error::Cancelled)
    ));
}

#[test]
fn parameter_memory_replay_charges_one_counter_including_normalization() {
    use crate::parameter_source_tests::{c, p, sign, space as family, sub};
    let guard = ParameterGuard {
        coordinate: 0,
        region: sign(&sub(&p(), &c(1)), PolynomialSigns::NEGATIVE),
    };
    let s = family(200, 2, std::slice::from_ref(&guard));
    let env = family(201, 1, &[guard]);
    let map = CoordinateMap::coordinates(&s, &env, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let product = FibreProduct::new(SpaceId([202; 32]), &map, &map, &()).unwrap();
    let action = WorldRelation::identity(&product, &()).unwrap();
    let obs = EventPartition::on(&s.full(), &[s.full()], PartitionLimits::default(), &()).unwrap();
    let memory =
        BeliefMemory::new(&[action], &obs, &s.full(), BeliefLimits::default(), &()).unwrap();
    let limits = BeliefDescriptorLimits::default();
    let data = BeliefDescriptor::capture(&memory, limits.descriptors, &()).unwrap();
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
    let restored = data.admit(limits, &mut work).unwrap();
    let cost = work.operations();
    assert!(cost > 0);
    assert_eq!(
        restored.states()[0].possible().to_bytes(&()).unwrap(),
        s.full().to_bytes(&()).unwrap()
    );
    let mut exact = ExactArithmetic::new(
        ArithmeticLimits {
            operations: cost,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(data.admit(limits, &mut exact).is_ok());
    assert!(matches!(
        data.admit(limits, &mut exact),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let mut short = ExactArithmetic::new(
        ArithmeticLimits {
            operations: cost - 1,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(matches!(
        data.admit(limits, &mut short),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
    BeliefMemory::new_with_parameters(
        memory.actions(),
        &obs,
        &s.full(),
        limits.beliefs,
        limits.parameters,
        &mut work,
    )
    .unwrap();
    // Canonically identical guards can normalize without arithmetic. Fresh
    // products must still charge guard attachment/projection admission.
    let mut no_work = ExactArithmetic::new(
        ArithmeticLimits {
            operations: 0,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(matches!(
        FibreProduct::with_order_and_parameters(
            SpaceId([203; 32]),
            &map,
            &map,
            &[0, 1, 2, 3],
            Limits::default(),
            limits.parameters,
            &mut no_work,
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
}
