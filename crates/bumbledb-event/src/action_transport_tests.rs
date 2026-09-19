use super::{StopAfter, base, mask, replay, small_products, space, table};
use crate::*;
use std::cell::Cell;

fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn arena(edges: u64) -> ActionArena {
    let (_, actions, steps) = small_products();
    let transition = WorldRelation::new(&steps, &table(steps.space(), edges), &()).unwrap();
    ActionArena::new(&actions, &transition, &()).unwrap()
}
fn capture(value: AdmittedActionDescriptor) -> ActionDescriptor {
    let result = ActionDescriptor::capture(&value, DescriptorLimits::default(), &()).unwrap();
    drop(value);
    result
}
fn reaching() -> ActionDescriptor {
    let arena = arena(0b1110_0001);
    capture(AdmittedActionDescriptor::Reach(
        arena
            .winning_reach(
                &arena.states().coordinate(0, &()).unwrap(),
                FixedPointLimits::default(),
                PartitionLimits::default(),
                &(),
            )
            .unwrap(),
    ))
}

#[test]
fn replay_recomputes_progress_and_checks_selected_policy_including_empty_objectives() {
    let limits = ActionDescriptorLimits::default();
    let original = reaching();
    let ActionDescriptor::Reach { arena, .. } = &original else {
        panic!()
    };
    let AdmittedActionDescriptor::Arena(admitted) = ActionDescriptor::Arena(arena.clone())
        .admit(limits, &mut work())
        .unwrap()
    else {
        panic!()
    };
    for (bits, expected) in [
        (0, Error::IncompletePolicy),
        (1, Error::UnsafePolicy),
        (15, Error::UnsafePolicy),
    ] {
        let mut data = original.clone();
        let ActionDescriptor::Reach { policy, .. } = &mut data else {
            panic!()
        };
        *policy = Some(
            table(admitted.actions().space(), bits)
                .to_bytes(&())
                .unwrap(),
        );
        assert_eq!(data.admit(limits, &mut work()).unwrap_err(), expected);
    }
    let mut compute = original.clone();
    let ActionDescriptor::Reach { policy, .. } = &mut compute else {
        panic!()
    };
    *policy = None;
    assert_eq!(
        capture(compute.admit(limits, &mut work()).unwrap()),
        original
    );
    // A forged objective is not allowed to reuse a formerly safe selected policy.
    let mut changed = original.clone();
    let ActionDescriptor::Reach { goal, .. } = &mut changed else {
        panic!()
    };
    *goal = admitted.states().full().to_bytes(&()).unwrap();
    assert_eq!(
        changed.admit(limits, &mut work()).unwrap_err(),
        Error::UnsafePolicy
    );
    let ActionDescriptor::Reach { goal, .. } = &mut changed else {
        panic!()
    };
    *goal = admitted.states().empty().to_bytes(&()).unwrap();
    assert_eq!(
        changed.admit(limits, &mut work()).unwrap_err(),
        Error::UnsafePolicy
    );
    let ActionDescriptor::Reach { policy, .. } = &mut changed else {
        panic!()
    };
    *policy = Some(b"BEVT\x01".to_vec());
    assert!(changed.admit(limits, &mut work()).is_err());

    // Retain a proper nondeterministic-policy restriction, not just the full solver result.
    let safe_arena = self::arena(5);
    let safe = safe_arena
        .winning_safe(&safe_arena.states().full(), limits.fixed_points, &())
        .unwrap();
    let selected = WorldRelation::new(
        safe_arena.actions(),
        &table(safe_arena.actions().space(), 4),
        &(),
    )
    .unwrap();
    let selected = safe.with_policy(&selected, &()).unwrap();
    let AdmittedActionDescriptor::Safe(restored) = replay(AdmittedActionDescriptor::Safe(selected))
    else {
        panic!()
    };
    assert_eq!(mask(restored.policy().region(), 4), 4);
    assert_eq!(mask(restored.winning(), 2), 1);
    // Capture also preserves a proper reach restriction with two good choices.
    let reaches = self::arena(0b1111_0000);
    let strategy = reaches
        .winning_reach(
            &reaches.states().coordinate(0, &()).unwrap(),
            limits.fixed_points,
            limits.partitions,
            &(),
        )
        .unwrap();
    let selected =
        WorldRelation::new(reaches.actions(), &table(reaches.actions().space(), 1), &()).unwrap();
    let selected = strategy.with_policy(&selected, &()).unwrap();
    let AdmittedActionDescriptor::Reach(restored) =
        replay(AdmittedActionDescriptor::Reach(selected))
    else {
        panic!()
    };
    assert_eq!(mask(restored.policy().region(), 4), 1);
}

#[test]
fn original_concatenation_and_converse_roles_survive_replay() {
    let (states, actions, steps) = small_products();
    // Author each product in the opposite concatenation order and then converse it.
    let actions = FibreProduct::new(
        SpaceId([41; 32]),
        actions.right_environment(),
        actions.left_environment(),
        &(),
    )
    .unwrap()
    .converse();
    let env = steps.left_environment().map().target();
    let trans = FibreProduct::new(
        SpaceId([42; 32]),
        &base(&states, env),
        &base(actions.space(), env),
        &(),
    )
    .unwrap()
    .converse();
    let goal = states.coordinate(0, &()).unwrap();
    let edges = trans.right().map().pullback(&goal, &()).unwrap();
    let transition = WorldRelation::new(&trans, &edges, &()).unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let AdmittedActionDescriptor::Arena(restored) = replay(AdmittedActionDescriptor::Arena(arena))
    else {
        panic!()
    };
    assert!(restored.actions().is_reversed());
    assert!(restored.transition().product().is_reversed());
    assert_eq!(restored.states().identity(), states.identity());
    let reach = restored
        .winning_reach(
            &goal,
            FixedPointLimits::default(),
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
    let AdmittedActionDescriptor::Reach(reach) = replay(AdmittedActionDescriptor::Reach(reach))
    else {
        panic!()
    };
    assert!(reach.winning().is_full());
    assert_eq!(mask(reach.policy().region(), 4), 3); // action bits precede state bits
}

#[test]
fn arena_replay_checks_role_full_markers_and_environment_even_for_empty_edges() {
    let original = capture(AdmittedActionDescriptor::Arena(arena(0)));
    let limits = ActionDescriptorLimits::default();
    let ActionDescriptor::Arena(mut data) = original.clone() else {
        panic!()
    };
    let source = Event::from_bytes(&data.actions.left.source, &())
        .unwrap()
        .space();
    data.actions.left.source = source.empty().to_bytes(&()).unwrap();
    assert_eq!(
        ActionDescriptor::Arena(data)
            .admit(limits, &mut work())
            .unwrap_err(),
        Error::InvalidEncoding
    );
    let ActionDescriptor::Arena(mut data) = original.clone() else {
        panic!()
    };
    data.actions.identity = SpaceId([99; 32]);
    assert_eq!(
        ActionDescriptor::Arena(data)
            .admit(limits, &mut work())
            .unwrap_err(),
        Error::SpaceMismatch
    );
    let ActionDescriptor::Arena(mut data) = original else {
        panic!()
    };
    data.transition.product.reversed = true;
    assert_eq!(
        ActionDescriptor::Arena(data)
            .admit(limits, &mut work())
            .unwrap_err(),
        Error::SpaceMismatch
    );
    let (_, actions, _) = small_products();
    let outcome = space(90, 1);
    let env = space(3, 0);
    let steps = FibreProduct::new(
        SpaceId([91; 32]),
        &base(actions.space(), &env),
        &base(&outcome, &env),
        &(),
    )
    .unwrap();
    let transition = WorldRelation::new(&steps, &steps.space().full(), &()).unwrap();
    let data = capture(AdmittedActionDescriptor::Arena(
        ActionArena::new(&actions, &transition, &()).unwrap(),
    ));
    let AdmittedActionDescriptor::Arena(restored) = data.admit(limits, &mut work()).unwrap() else {
        panic!()
    };
    assert!(
        restored
            .controllable_predecessor(&outcome.full(), &())
            .unwrap()
            .is_full()
    );
    let ActionDescriptor::Arena(arena) = data else {
        panic!()
    };
    assert_eq!(
        ActionDescriptor::Safe {
            arena,
            invariant: restored.states().full().to_bytes(&()).unwrap(),
            policy: None
        }
        .admit(limits, &mut work())
        .unwrap_err(),
        Error::SpaceMismatch
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn action_wire_refuses_every_truncated_prefix_unknown_tags_and_combined_budgets() {
    let data = reaching();
    let limits = ActionDescriptorLimits::default();
    let bytes = data.to_bytes(limits.descriptors, &()).unwrap();
    assert_eq!(&bytes[..6], b"BEAC\x01\x01");
    assert_eq!(
        ActionDescriptor::from_bytes(&bytes, limits.descriptors, &()).unwrap(),
        data
    );
    for end in 0..bytes.len() {
        assert!(ActionDescriptor::from_bytes(&bytes[..end], limits.descriptors, &()).is_err());
    }
    for index in [0, 4, 5, 6 + 32] {
        let mut bad = bytes.clone();
        bad[index] = 255;
        assert!(ActionDescriptor::from_bytes(&bad, limits.descriptors, &()).is_err());
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert_eq!(
        ActionDescriptor::from_bytes(&bad, limits.descriptors, &()).unwrap_err(),
        Error::InvalidEncoding
    );
    // Capture, syntax parsing and admission count the same nested roster items.
    let count = (0..200)
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
    let short = DescriptorLimits {
        items: count - 1,
        ..limits.descriptors
    };
    let error = Error::Capacity(Capacity::DescriptorItems);
    assert_eq!(data.to_bytes(short, &()).unwrap_err(), error);
    assert_eq!(
        ActionDescriptor::from_bytes(&bytes, short, &()).unwrap_err(),
        error
    );
    assert_eq!(
        data.admit(
            ActionDescriptorLimits {
                descriptors: short,
                ..limits
            },
            &mut work()
        )
        .unwrap_err(),
        error
    );
    let admitted = data.admit(limits, &mut work()).unwrap();
    assert_eq!(
        ActionDescriptor::capture(&admitted, short, &()).unwrap_err(),
        error
    );
    let exact = DescriptorLimits {
        items: count,
        bytes: bytes.len(),
        ..limits.descriptors
    };
    assert_eq!(
        ActionDescriptor::from_bytes(&bytes, exact, &()).unwrap(),
        data
    );
    let short = DescriptorLimits {
        bytes: bytes.len() - 1,
        ..exact
    };
    assert_eq!(
        data.to_bytes(short, &()).unwrap_err(),
        Error::Capacity(Capacity::DescriptorBytes)
    );
    assert_eq!(
        ActionDescriptor::from_bytes(&bytes, short, &()).unwrap_err(),
        Error::Capacity(Capacity::DescriptorBytes)
    );
    for limits in [
        ActionDescriptorLimits {
            fixed_points: FixedPointLimits {
                iterations: 0,
                ..limits.fixed_points
            },
            ..limits
        },
        ActionDescriptorLimits {
            partitions: PartitionLimits { cells: 1 },
            ..limits
        },
        ActionDescriptorLimits {
            descriptors: DescriptorLimits {
                events: Limits {
                    records: 0,
                    ..limits.descriptors.events
                },
                ..limits.descriptors
            },
            ..limits
        },
    ] {
        assert!(matches!(
            data.admit(limits, &mut work()),
            Err(Error::Capacity(_))
        ));
    }
    for count in [0, 20, 500] {
        let stop = StopAfter(Cell::new(count));
        assert_eq!(
            data.admit(
                limits,
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &stop)
            )
            .unwrap_err(),
            Error::Cancelled
        );
    }
}

#[test]
fn family_strategy_replay_preserves_one_parameter_and_charges_one_arithmetic_allowance() {
    use crate::parameter_source_tests::{c, p, sign, space as family, sub};
    let guard = ParameterGuard {
        coordinate: 0,
        region: sign(&sub(&p(), &c(1)), PolynomialSigns::NEGATIVE),
    };
    let s = family(70, 2, std::slice::from_ref(&guard));
    let env = family(71, 1, &[guard]);
    let map = CoordinateMap::coordinates(&s, &env, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let choices = CoordinateMap::coordinates(&env, &env, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let actions = FibreProduct::new(SpaceId([72; 32]), &map, &choices, &()).unwrap();
    let pair_env = actions
        .left()
        .map()
        .then(map.map(), &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let steps = FibreProduct::new(SpaceId([73; 32]), &pair_env, &map, &()).unwrap();
    let goal = s.coordinate(1, &()).unwrap();
    let transition = WorldRelation::new(
        &steps,
        &steps.right().map().pullback(&goal, &()).unwrap(),
        &(),
    )
    .unwrap();
    let arena = ActionArena::new(&actions, &transition, &()).unwrap();
    let limits = ActionDescriptorLimits::default();
    let reach = arena
        .winning_reach(&goal, limits.fixed_points, limits.partitions, &())
        .unwrap();
    let safe = arena.winning_safe(&goal, limits.fixed_points, &()).unwrap();
    for input in [
        AdmittedActionDescriptor::Arena(arena),
        AdmittedActionDescriptor::Reach(reach),
        AdmittedActionDescriptor::Safe(safe),
    ] {
        let data = capture(input);
        let mut counted = work();
        let restored = data.admit(limits, &mut counted).unwrap();
        assert_eq!(capture(restored), data);
        let cost = counted.operations();
        assert!(cost > 0);
        let mut short = ExactArithmetic::new(
            ArithmeticLimits {
                operations: cost - 1,
                ..ArithmeticLimits::default()
            },
            &(),
        );
        assert_eq!(
            data.admit(limits, &mut short).unwrap_err(),
            Error::Capacity(Capacity::ArithmeticSteps)
        );
        let mut exact = ExactArithmetic::new(
            ArithmeticLimits {
                operations: cost,
                ..ArithmeticLimits::default()
            },
            &(),
        );
        assert!(data.admit(limits, &mut exact).is_ok());
        assert_eq!(
            data.admit(limits, &mut exact).unwrap_err(),
            Error::Capacity(Capacity::ArithmeticSteps)
        );
    }
}

#[test]
fn independently_authored_beac_grammar_agrees_with_native_codec() {
    fn number(out: &mut Vec<u8>, n: usize) {
        out.extend((n as u64).to_le_bytes());
    }
    fn blob(out: &mut Vec<u8>, bytes: &[u8]) {
        number(out, bytes.len());
        out.extend(bytes);
    }
    fn map(out: &mut Vec<u8>, value: &MapDescriptor) {
        blob(out, &value.source);
        blob(out, &value.target);
        number(out, value.readouts.len());
        for event in &value.readouts {
            blob(out, event);
        }
    }
    fn fibre(out: &mut Vec<u8>, value: &FibreDescriptor) {
        out.extend(value.identity.0);
        out.push(u8::from(value.reversed));
        map(out, &value.left);
        map(out, &value.right);
    }
    let ActionDescriptor::Reach {
        arena,
        goal,
        policy,
    } = reaching()
    else {
        panic!()
    };
    for tag in 0..3 {
        for selected in [None, policy.clone()] {
            let data = match tag {
                0 => ActionDescriptor::Arena(arena.clone()),
                1 => ActionDescriptor::Reach {
                    arena: arena.clone(),
                    goal: goal.clone(),
                    policy: selected.clone(),
                },
                _ => ActionDescriptor::Safe {
                    arena: arena.clone(),
                    invariant: goal.clone(),
                    policy: selected.clone(),
                },
            };
            let mut bytes = b"BEAC\x01".to_vec();
            bytes.push(tag);
            fibre(&mut bytes, &arena.actions);
            fibre(&mut bytes, &arena.transition.product);
            blob(&mut bytes, &arena.transition.region);
            if tag != 0 {
                blob(&mut bytes, &goal);
                let flag = bytes.len();
                bytes.push(u8::from(selected.is_some()));
                if let Some(policy) = &selected {
                    blob(&mut bytes, policy);
                }
                let mut bad = bytes.clone();
                bad[flag] = 2;
                assert_eq!(
                    ActionDescriptor::from_bytes(&bad, DescriptorLimits::default(), &())
                        .unwrap_err(),
                    Error::InvalidEncoding
                );
            }
            assert_eq!(
                ActionDescriptor::from_bytes(&bytes, DescriptorLimits::default(), &()).unwrap(),
                data
            );
            assert_eq!(
                bytes,
                data.to_bytes(DescriptorLimits::default(), &()).unwrap()
            );
        }
    }
}

#[test]
fn measured_zero_mass_outcomes_remain_adversarial_possibilities_after_replay() {
    let raw = space(80, 1);
    let s = raw
        .with_density(
            &[DensityPiece {
                region: table(&raw, 1),
                density: ExactRational::one(),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let env = space(81, 0);
    let a = space(82, 0);
    let sb = base(&s, &env);
    let actions = FibreProduct::new(SpaceId([83; 32]), &sb, &base(&a, &env), &()).unwrap();
    let steps =
        FibreProduct::new(SpaceId([84; 32]), &base(actions.space(), &env), &sb, &()).unwrap();
    // Every action can reach either world, including the one of mass zero.
    let transition = WorldRelation::new(&steps, &steps.space().full(), &()).unwrap();
    let value = ActionArena::new(&actions, &transition, &()).unwrap();
    let AdmittedActionDescriptor::Arena(value) = replay(AdmittedActionDescriptor::Arena(value))
    else {
        panic!()
    };
    assert_eq!(
        value.states().full().to_bytes(&()).unwrap(),
        s.full().to_bytes(&()).unwrap()
    );
    let mass_one = table(value.states(), 1);
    assert_eq!(mass_one.mass(&mut work()).unwrap(), ExactRational::one());
    assert!(value.good(&mass_one, &()).unwrap().region().is_empty());
    let safe = value
        .winning_safe(&mass_one, FixedPointLimits::default(), &())
        .unwrap();
    let AdmittedActionDescriptor::Safe(safe) = replay(AdmittedActionDescriptor::Safe(safe)) else {
        panic!()
    };
    assert!(safe.winning().is_empty());
}
