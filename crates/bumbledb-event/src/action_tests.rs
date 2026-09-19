use std::cell::Cell;

use crate::{
    ActionArena, Capacity, Control, CoordinateMap, Error, Event, FibreProduct, FixedPointLimits,
    PartitionLimits, RelationalProduct, Result, Space, SpaceId, SurjectiveMap, WorldRelation,
};

#[path = "action_transport_tests.rs"]
mod transport;

fn replay(value: crate::AdmittedActionDescriptor) -> crate::AdmittedActionDescriptor {
    let limits = crate::ActionDescriptorLimits::default();
    let data = crate::ActionDescriptor::capture(&value, limits.descriptors, &()).unwrap();
    let bytes = data.to_bytes(limits.descriptors, &()).unwrap();
    drop(value);
    let imported = crate::ActionDescriptor::import(
        &bytes,
        limits,
        &mut crate::ExactArithmetic::new(crate::ArithmeticLimits::default(), &()),
    )
    .unwrap();
    assert_eq!(
        crate::ActionDescriptor::capture(&imported, limits.descriptors, &()).unwrap(),
        data
    );
    imported
}

fn space(id: u8, bits: u8) -> Space {
    Space::new(SpaceId([id; 32]), bits, &()).unwrap()
}
fn table(space: &Space, bits: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[bits], &())
        .unwrap()
}
fn base(space: &Space, env: &Space) -> SurjectiveMap {
    CoordinateMap::new(space, env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap()
}
fn mask(event: &Event, worlds: u64) -> u64 {
    (0..worlds)
        .filter(|&w| event.contains(w).unwrap())
        .fold(0, |bits, w| bits | (1 << w))
}

fn small_products() -> (Space, FibreProduct, FibreProduct) {
    let states = space(1, 1);
    let choices = space(2, 1);
    let env = space(3, 0);
    let states_base = base(&states, &env);
    let actions =
        FibreProduct::new(SpaceId([4; 32]), &states_base, &base(&choices, &env), &()).unwrap();
    let steps = FibreProduct::new(
        SpaceId([5; 32]),
        &base(actions.space(), &env),
        &states_base,
        &(),
    )
    .unwrap();
    (states, actions, steps)
}

fn outcomes(edges: u64, state: u64, action: u64) -> u64 {
    (0..2)
        .filter(|&t| edges & (1 << (state + 2 * action + 4 * t)) != 0)
        .fold(0, |bits, t| bits | (1 << t))
}
fn good(edges: u64, selected: u64) -> u64 {
    (0..4)
        .filter(|pair| {
            let out = outcomes(edges, pair % 2, pair / 2);
            out != 0 && out & !selected == 0
        })
        .fold(0, |bits, pair| bits | (1 << pair))
}
fn pre(edges: u64, selected: u64) -> u64 {
    let allowed = good(edges, selected);
    (0..2)
        .filter(|&s| allowed & ((1 << s) | (1 << (s + 2))) != 0)
        .fold(0, |bits, s| bits | (1 << s))
}

// Independently enumerate every deterministic memoryless policy. For a fixed
// policy there is only universal environment behavior, including dead ends.
fn policy_oracle(edges: u64, goal: u64, safety: bool) -> u64 {
    let mut union = 0;
    for policy in 0..4 {
        let mut won = if safety { 3 } else { 0 };
        loop {
            let predecessor = (0..2)
                .filter(|&s| {
                    let out = outcomes(edges, s, (policy >> s) & 1);
                    out != 0 && out & !won == 0
                })
                .fold(0, |bits, s| bits | (1 << s));
            let next = if safety {
                goal & predecessor
            } else {
                goal | predecessor
            };
            if next == won {
                break;
            }
            won = next;
        }
        union |= won;
    }
    union
}

#[test]
fn every_two_state_action_arena_matches_enumerated_policy_oracles() {
    let (states, actions, steps) = small_products();
    let limits = FixedPointLimits::default();
    for edges in 0..256 {
        let transition = WorldRelation::new(&steps, &table(steps.space(), edges), &()).unwrap();
        let arena = ActionArena::new(&actions, &transition, &()).unwrap();
        let crate::AdmittedActionDescriptor::Arena(arena) =
            replay(crate::AdmittedActionDescriptor::Arena(arena))
        else {
            panic!()
        };
        assert_eq!(
            mask(arena.enabled(&()).unwrap().region(), 4),
            good(edges, 3)
        );
        let program = arena.predecessor_program(&()).unwrap();
        for goal in 0..4 {
            let event = table(&states, goal);
            assert_eq!(
                mask(arena.good(&event, &()).unwrap().region(), 4),
                good(edges, goal)
            );
            assert_eq!(
                mask(&arena.controllable_predecessor(&event, &()).unwrap(), 2),
                pre(edges, goal)
            );
            assert_eq!(
                mask(&program.evaluate(&event, &()).unwrap(), 2),
                pre(edges, goal)
            );
            let reach = arena
                .winning_reach(&event, limits, PartitionLimits::default(), &())
                .unwrap();
            let crate::AdmittedActionDescriptor::Reach(reach) =
                replay(crate::AdmittedActionDescriptor::Reach(reach))
            else {
                panic!()
            };
            assert_eq!(mask(reach.winning(), 2), policy_oracle(edges, goal, false));
            let mut earlier = 0;
            let mut allowed = 0;
            for (rank, layer) in reach.ranked().layers().cells().iter().enumerate() {
                let layer = mask(layer, 2);
                assert_ne!(layer, 0);
                assert_eq!(layer & earlier, 0);
                if rank == 0 {
                    assert_eq!(layer, goal);
                } else {
                    allowed |= good(edges, earlier) & (layer | (layer << 2));
                }
                earlier |= layer;
            }
            assert_eq!(earlier, mask(reach.winning(), 2));
            assert_eq!(mask(reach.policy().region(), 4), allowed);
            let safe = arena.winning_safe(&event, limits, &()).unwrap();
            let crate::AdmittedActionDescriptor::Safe(safe) =
                replay(crate::AdmittedActionDescriptor::Safe(safe))
            else {
                panic!()
            };
            let winning = policy_oracle(edges, goal, true);
            assert_eq!(mask(safe.winning(), 2), winning);
            assert_eq!(
                mask(safe.policy().region(), 4),
                good(edges, winning) & (winning | (winning << 2))
            );
        }
    }
}

#[test]
fn rank_witnesses_reject_stalling_and_partial_policies() {
    let (states, actions, steps) = small_products();
    // 0:a0→0, a1→1; 1:a0/a1→1. Goal state 1 stops immediately.
    let transition = WorldRelation::new(&steps, &table(steps.space(), 0b1110_0001), &()).unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let strategy = arena
        .winning_reach(
            &table(&states, 2),
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    assert!(strategy.winning().is_full());
    assert_eq!(mask(strategy.policy().region(), 4), 4); // Only action 1 at state 0.
    let stalling = arena.good(strategy.winning(), &()).unwrap();
    assert!(stalling.region().contains(0).unwrap());
    assert_eq!(
        strategy.with_policy(&stalling, &()).unwrap_err(),
        Error::UnsafePolicy
    );
    let absent = WorldRelation::new(&actions, &actions.space().empty(), &()).unwrap();
    assert_eq!(
        strategy.with_policy(&absent, &()).unwrap_err(),
        Error::IncompletePolicy
    );
    let retained = strategy.with_policy(strategy.policy(), &()).unwrap();
    drop((states, actions, steps, transition, arena, strategy));
    assert!(retained.winning().is_full());
    assert_eq!(retained.ranked().layers().locate(0, &()).unwrap(), Some(1));
    assert_eq!(retained.ranked().layers().locate(1, &()).unwrap(), Some(0));
}

#[test]
fn safety_requires_continuation_and_allows_any_remaining_safe_choice() {
    let (states, actions, steps) = small_products();
    // Both actions at 0 stay in 0; 1 is terminal even though invariant holds.
    let transition = WorldRelation::new(&steps, &table(steps.space(), 5), &()).unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let safe = arena
        .winning_safe(&states.full(), FixedPointLimits::default(), &())
        .unwrap();
    assert_eq!(mask(safe.winning(), 2), 1);
    assert_eq!(mask(safe.policy().region(), 4), 5);
    let chosen = WorldRelation::new(&actions, &table(actions.space(), 4), &()).unwrap();
    let chosen = safe.with_policy(&chosen, &()).unwrap();
    assert_eq!(mask(chosen.policy().region(), 4), 4);
    let absent = WorldRelation::new(&actions, &actions.space().empty(), &()).unwrap();
    assert_eq!(
        safe.with_policy(&absent, &()).unwrap_err(),
        Error::IncompletePolicy
    );
    let unavailable = WorldRelation::new(&actions, &table(actions.space(), 15), &()).unwrap();
    assert_eq!(
        safe.with_policy(&unavailable, &()).unwrap_err(),
        Error::UnsafePolicy
    );
}

#[test]
fn uniform_permissions_are_greatest_sound_choices_on_inhabited_cases() {
    let states = space(1, 1);
    let observations = space(2, 1);
    let actions = space(3, 1);
    let env = space(4, 0);
    let s = base(&states, &env);
    let o = base(&observations, &env);
    let a = base(&actions, &env);
    let so = FibreProduct::new(SpaceId([5; 32]), &s, &o, &()).unwrap();
    let oa = FibreProduct::new(SpaceId([6; 32]), &o, &a, &()).unwrap();
    let sa = FibreProduct::new(SpaceId([7; 32]), &s, &a, &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([8; 32]), &so, &oa, &sa, &()).unwrap();
    for cases in 0..16 {
        let reversed = WorldRelation::new(&so, &table(so.space(), cases), &()).unwrap();
        let information = reversed.converse();
        let inhabited = (0..2)
            .filter(|o| cases & (3 << (2 * o)) != 0)
            .fold(0, |bits, o| bits | (1 << o));
        for good in 0..16 {
            let good = WorldRelation::new(&sa, &table(sa.space(), good), &()).unwrap();
            let permissions = plan.uniform_permissions(&information, &good, &()).unwrap();
            for action in 0..2 {
                for observation in 0..2 {
                    let expected = inhabited & (1 << observation) != 0
                        && (0..2).all(|s| {
                            cases & (1 << (s + 2 * observation)) == 0
                                || good.region().contains(s + 2 * action).unwrap()
                        });
                    assert_eq!(
                        permissions
                            .region()
                            .contains(observation + 2 * action)
                            .unwrap(),
                        expected
                    );
                }
            }
            for candidate in 0..16 {
                let candidate =
                    WorldRelation::new(&oa, &table(oa.space(), candidate), &()).unwrap();
                let covered = mask(&candidate.domain(&()).unwrap(), 2) & !inhabited == 0;
                let sound = plan
                    .compose(&reversed, &candidate, &())
                    .unwrap()
                    .included_in(&good, &())
                    .unwrap();
                assert_eq!(
                    candidate.included_in(&permissions, &()).unwrap(),
                    covered && sound
                );
            }
        }
    }
    // Each hidden state permits a different action; the shared case permits none.
    let information = WorldRelation::new(&so, &table(so.space(), 3), &())
        .unwrap()
        .converse();
    let good = WorldRelation::new(&sa, &table(sa.space(), 9), &()).unwrap();
    assert!(good.domain(&()).unwrap().is_full());
    assert!(
        plan.uniform_permissions(&information, &good, &())
            .unwrap()
            .region()
            .is_empty()
    );
}

struct StopAfter(Cell<usize>);
impl Control for StopAfter {
    fn checkpoint(&self) -> Result<()> {
        let n = self.0.get();
        if n == 0 {
            Err(Error::Cancelled)
        } else {
            self.0.set(n - 1);
            Ok(())
        }
    }
}

#[test]
fn arena_admission_and_strategy_budgets_refuse_without_partial_results() {
    let (states, actions, steps) = small_products();
    let transition = WorldRelation::new(&steps, &table(steps.space(), 0b1110_0001), &()).unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let wrong = space(9, 1);
    assert_eq!(
        arena.good(&wrong.empty(), &()).unwrap_err(),
        Error::SpaceMismatch
    );
    assert_eq!(
        arena.reach_program(&wrong.empty(), &()).unwrap_err(),
        Error::SpaceMismatch
    );
    assert_eq!(
        ActionArena::new(&steps, &transition, &()).unwrap_err(),
        Error::SpaceMismatch
    );
    assert_eq!(
        arena
            .winning_reach(
                &table(&states, 2),
                FixedPointLimits::default(),
                PartitionLimits { cells: 1 },
                &()
            )
            .unwrap_err(),
        Error::Capacity(Capacity::PartitionCells)
    );
    assert_eq!(
        arena
            .winning_safe(
                &states.full(),
                FixedPointLimits {
                    iterations: 0,
                    program_steps: 100
                },
                &()
            )
            .unwrap_err(),
        Error::Capacity(Capacity::FixedPointIterations)
    );
    assert_eq!(
        arena
            .good(&states.full(), &StopAfter(Cell::new(0)))
            .unwrap_err(),
        Error::Cancelled
    );
    assert_eq!(
        arena
            .winning_reach(
                &states.full(),
                FixedPointLimits::default(),
                PartitionLimits::default(),
                &StopAfter(Cell::new(80))
            )
            .unwrap_err(),
        Error::Cancelled
    );
    let no_goals = arena
        .winning_reach(
            &states.empty(),
            FixedPointLimits::default(),
            PartitionLimits { cells: 0 },
            &(),
        )
        .unwrap();
    assert!(no_goals.winning().is_empty());
    assert!(no_goals.ranked().layers().cells().is_empty());
    assert!(no_goals.policy().region().is_empty());
}

#[test]
fn symbolic_twenty_bit_controlled_rotation_retains_progress_without_world_enumeration() {
    let states = space(1, 20);
    let choices = space(2, 0);
    let env = space(3, 0);
    let s = base(&states, &env);
    let actions = FibreProduct::new(SpaceId([4; 32]), &s, &base(&choices, &env), &()).unwrap();
    let steps = FibreProduct::new(SpaceId([5; 32]), &base(actions.space(), &env), &s, &()).unwrap();
    let coordinates = (1..20).chain(std::iter::once(0)).collect::<Vec<_>>();
    let rotate = CoordinateMap::coordinates(actions.space(), &states, &coordinates, &()).unwrap();
    let transition = WorldRelation::graph(&steps, &rotate, &()).unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let goal = states.coordinate(0, &()).unwrap();
    let reach = arena
        .winning_reach(
            &goal,
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    assert_eq!(reach.winning().count(&()).unwrap(), (1 << 20) - 1);
    assert_eq!(reach.ranked().layers().cells().len(), 20);
    assert_eq!(reach.ranked().result().iterations(), 21);
    assert_eq!(reach.policy().region().count(&()).unwrap(), (1 << 19) - 1);
    let safe = arena
        .winning_safe(&goal, FixedPointLimits::default(), &())
        .unwrap();
    assert_eq!(safe.winning().count(&()).unwrap(), 1);
    assert!(safe.winning().contains((1 << 20) - 1).unwrap());
}

#[test]
fn shared_environment_roles_survive_strategy_construction_and_refuse_mixing() {
    let states = space(1, 2); // environment bit 0, state bit 1
    let choices = space(2, 1); // one choice per environment
    let env = space(3, 1);
    let s = CoordinateMap::coordinates(&states, &env, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let a = CoordinateMap::coordinates(&choices, &env, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let actions = FibreProduct::new(SpaceId([4; 32]), &s, &a, &()).unwrap();
    let pair_env = actions
        .left()
        .map()
        .then(s.map(), &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let steps = FibreProduct::new(SpaceId([5; 32]), &pair_env, &s, &()).unwrap();
    let goal = states.coordinate(1, &()).unwrap();
    let edges = steps.right().map().pullback(&goal, &()).unwrap();
    let transition = WorldRelation::new(&steps, &edges, &()).unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let strategy = arena
        .winning_reach(
            &goal,
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    assert!(strategy.winning().is_full());
    assert_eq!(strategy.policy().region().count(&()).unwrap(), 2);
    for state in 0..4 {
        assert_eq!(
            strategy.ranked().layers().locate(state, &()).unwrap(),
            Some(usize::from(state < 2))
        );
        for action in 0..2 {
            let code = state + 4 * action;
            if state % 2 == action {
                assert_eq!(
                    strategy.policy().region().contains(code).unwrap(),
                    state < 2
                );
            } else {
                assert_eq!(
                    strategy.policy().region().contains(code),
                    Err(Error::IllegalWorld(code))
                );
            }
        }
    }
    // Same input space, but a different readout is claimed as its environment.
    let wrong_input_env = CoordinateMap::coordinates(actions.space(), &env, &[1], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let wrong_steps = FibreProduct::new(SpaceId([6; 32]), &wrong_input_env, &s, &()).unwrap();
    let wrong = WorldRelation::new(&wrong_steps, &wrong_steps.space().empty(), &()).unwrap();
    assert_eq!(
        ActionArena::new(&actions, &wrong, &()).unwrap_err(),
        Error::EnvironmentMismatch
    );
    // One-step output roles are valid, but they cannot be iterated as the input role.
    let wrong_output_env = CoordinateMap::coordinates(&states, &env, &[1], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let wrong_steps =
        FibreProduct::new(SpaceId([7; 32]), &pair_env, &wrong_output_env, &()).unwrap();
    let wrong = WorldRelation::new(&wrong_steps, &wrong_steps.space().empty(), &()).unwrap();
    let wrong = ActionArena::new(&actions, &wrong, &()).unwrap();
    assert!(wrong.good(&states.full(), &()).unwrap().region().is_empty());
    assert_eq!(
        wrong.reach_program(&states.empty(), &()).unwrap_err(),
        Error::EnvironmentMismatch
    );
}

#[test]
fn one_step_actions_can_change_context_but_iteration_cannot() {
    let (_, actions, _) = small_products();
    let outcome = space(9, 1);
    let env = space(3, 0);
    let pair = FibreProduct::new(
        SpaceId([8; 32]),
        &base(actions.space(), &env),
        &base(&outcome, &env),
        &(),
    )
    .unwrap();
    let relation = WorldRelation::new(&pair, &pair.space().full(), &()).unwrap();
    let arena = ActionArena::new(&actions, &relation, &()).unwrap();
    assert!(
        arena
            .controllable_predecessor(&outcome.full(), &())
            .unwrap()
            .is_full()
    );
    assert_eq!(
        arena.reach_program(&outcome.full(), &()).unwrap_err(),
        Error::SpaceMismatch
    );
    let independent_goal =
        Event::from_bytes(&arena.states().full().to_bytes(&()).unwrap(), &()).unwrap();
    assert_eq!(
        arena.safe_program(&independent_goal, &()).unwrap_err(),
        Error::SpaceMismatch
    );
}
