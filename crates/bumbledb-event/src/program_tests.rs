use std::cell::Cell;

use crate::{
    BoolOp4, Capacity, Control, CoordinateMap, Error, Event, EventProgramBuilder, FibreProduct,
    FixedPointLimits, MapOp, ModalOp, ProgramOp, RelationalProduct, Result, Space, SpaceId,
    Variance, WorldRelation,
};

fn space(bits: u8) -> Space {
    Space::new(SpaceId([1; 32]), bits, &()).unwrap()
}
fn table(space: &Space, mask: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[mask], &())
        .unwrap()
}
fn members(event: &Event, support: u64, n: u64) -> u64 {
    (0..n)
        .filter(|&w| support & (1 << w) != 0 && event.contains(w).unwrap())
        .fold(0, |m, w| m | (1 << w))
}

fn pair(space: &Space) -> FibreProduct {
    let unit = Space::new(SpaceId([2; 32]), 0, &()).unwrap();
    let base = CoordinateMap::new(space, &unit, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    FibreProduct::new(SpaceId([3; 32]), &base, &base, &()).unwrap()
}

#[test]
fn all_truth_functions_and_variances_match_exhaustive_event_evaluation() {
    let space = space(2);
    for code in 0..16 {
        for left_kind in 0..3 {
            for right_kind in 0..3 {
                let op = BoolOp4::new(code).unwrap();
                let mut builder = EventProgramBuilder::new(&space, &()).unwrap();
                let input = builder.input();
                let neg = builder.complement(&input, &()).unwrap();
                let constant = builder.constant(&table(&space, 5), &()).unwrap();
                let values = [input, neg, constant];
                let root = builder
                    .apply(op, &values[left_kind], &values[right_kind], &())
                    .unwrap();
                let program = builder.finish(&root, &()).unwrap();
                let outputs = (0..16)
                    .map(|mask| {
                        let expected = (0..4).fold(0, |out, w| {
                            let values = [
                                mask & (1 << w) != 0,
                                mask & (1 << w) == 0,
                                5 & (1 << w) != 0,
                            ];
                            out | (u64::from(op.evaluate(values[left_kind], values[right_kind]))
                                << w)
                        });
                        let actual = program.evaluate(&table(&space, mask), &()).unwrap();
                        assert_eq!(members(&actual, 15, 4), expected);
                        expected
                    })
                    .collect::<Vec<_>>();
                for a in 0..16 {
                    for b in 0..16 {
                        if a & b == a {
                            for w in 0..4 {
                                assert!(program.variance().allows(
                                    outputs[a] & (1 << w) != 0,
                                    outputs[b] & (1 << w) != 0
                                ));
                            }
                        }
                    }
                }
                assert_eq!(
                    program.fixed_points(&()).is_ok(),
                    matches!(
                        program.variance(),
                        Variance::Independent | Variance::Increasing
                    )
                );
            }
        }
    }
}

#[test]
fn finite_original_world_rank_controls_least_and_greatest_iteration() {
    let raw = space(3);
    let space = raw.restrict(&table(&raw, 0b0001_0110), &()).unwrap(); // one-hot 001,010,100
    let rotate = CoordinateMap::coordinates(&space, &space, &[1, 2, 0], &()).unwrap();
    for (op, least_expected, greatest_expected) in [(BoolOp4::OR, 22, 22), (BoolOp4::AND, 0, 0)] {
        let mut builder = EventProgramBuilder::new(&space, &()).unwrap();
        let input = builder.input();
        let seed = builder
            .constant(&space.coordinate(0, &()).unwrap(), &())
            .unwrap();
        let next = builder.map(MapOp::Pullback, &rotate, &input, &()).unwrap();
        let root = builder.apply(op, &seed, &next, &()).unwrap();
        let program = builder
            .finish(&root, &())
            .unwrap()
            .fixed_points(&())
            .unwrap();
        assert_eq!(program.carrier().worlds(), 3);
        let least = program.least(FixedPointLimits::default(), &()).unwrap();
        let greatest = program.greatest(FixedPointLimits::default(), &()).unwrap();
        assert_eq!(members(least.event(), 22, 8), least_expected);
        assert_eq!(members(greatest.event(), 22, 8), greatest_expected);
        assert_eq!(least.program_steps(), least.iterations() * 4);
        assert_eq!(greatest.program_steps(), greatest.iterations() * 4);
        if op == BoolOp4::OR {
            assert_eq!(least.iterations(), 4);
        }
        assert!(greatest.iterations() <= 4);
        assert_eq!(
            program.program().evaluate(least.event(), &()).unwrap(),
            *least.event()
        );
        assert_eq!(
            program.program().evaluate(greatest.event(), &()).unwrap(),
            *greatest.event()
        );
    }
}

fn modal_mask(
    edges: u64,
    selected: u64,
    support: u64,
    n: u64,
    universal: bool,
    enabled: bool,
) -> u64 {
    (0..n)
        .filter(|&s| {
            if support & (1 << s) == 0 {
                return false;
            }
            let successors = (0..n)
                .filter(|&t| edges & (1 << (s + n * t)) != 0)
                .fold(0, |m, t| m | (1 << t));
            if universal {
                (!enabled || successors != 0) && successors & !selected == 0
            } else {
                successors & selected != 0
            }
        })
        .fold(0, |m, s| m | (1 << s))
}

fn iterate(mut value: u64, step: impl Fn(u64) -> u64) -> u64 {
    loop {
        let next = step(value);
        if next == value {
            return value;
        }
        value = next;
    }
}

#[test]
fn exhaustive_small_relations_match_reachability_inevitability_and_safety_oracles() {
    for support in [1, 2, 3] {
        let raw = space(1);
        let source = raw.restrict(&table(&raw, support), &()).unwrap();
        let pair = pair(&source);
        let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
        let legal_pairs = (0..4)
            .filter(|i| support & (1 << (i % 2)) != 0 && support & (1 << (i / 2)) != 0)
            .fold(0, |m, i| m | (1 << i));
        for edges in 0..16 {
            let edges = edges & legal_pairs;
            let relation = WorldRelation::new(&pair, &table(pair.space(), edges), &()).unwrap();
            let star = plan
                .star(&relation, FixedPointLimits::default(), &())
                .unwrap();
            for mask in 0..4 {
                let goal = mask & support;
                let event = table(&source, goal);
                let reach = iterate(0, |x| goal | modal_mask(edges, x, support, 2, false, false));
                let inevitable =
                    iterate(0, |x| goal | modal_mask(edges, x, support, 2, true, true));
                let safe = iterate(support, |x| {
                    goal & modal_mask(edges, x, support, 2, true, false)
                });
                assert_eq!(
                    members(
                        relation
                            .can_reach(&event, FixedPointLimits::default(), &())
                            .unwrap()
                            .event(),
                        support,
                        2
                    ),
                    reach
                );
                assert_eq!(
                    members(
                        relation
                            .inevitably_reach(&event, FixedPointLimits::default(), &())
                            .unwrap()
                            .event(),
                        support,
                        2
                    ),
                    inevitable
                );
                assert_eq!(
                    members(
                        relation
                            .safe_throughout(&event, FixedPointLimits::default(), &())
                            .unwrap()
                            .event(),
                        support,
                        2
                    ),
                    safe
                );
                assert_eq!(members(&star.may(&event, &()).unwrap(), support, 2), reach);
                assert_eq!(members(&star.all(&event, &()).unwrap(), support, 2), safe);
            }
            assert!(
                plan.compose(&star, &star, &())
                    .unwrap()
                    .equivalent(&star, &())
                    .unwrap()
            );
            assert!(relation.included_in(&star, &()).unwrap());
        }
    }
}

#[test]
fn arbitrary_four_state_closures_match_warshall_and_keep_coupled_support() {
    for support in [7, 11, 15] {
        let raw = space(2);
        let source = raw.restrict(&table(&raw, support), &()).unwrap();
        let pair = pair(&source);
        let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
        let mut random = 321u64;
        for _ in 0..128 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let mut edges = 0;
            for s in 0..4 {
                for t in 0..4 {
                    if support & (1 << s) != 0 && support & (1 << t) != 0 {
                        edges |= random & (1 << (s + 4 * t));
                    }
                }
            }
            let relation = WorldRelation::new(&pair, &table(pair.space(), edges), &()).unwrap();
            let mut closed = edges;
            for s in 0..4 {
                if support & (1 << s) != 0 {
                    closed |= 1 << (s + 4 * s);
                }
            }
            for middle in 0..4 {
                for s in 0..4 {
                    for t in 0..4 {
                        if closed & (1 << (s + 4 * middle)) != 0
                            && closed & (1 << (middle + 4 * t)) != 0
                        {
                            closed |= 1 << (s + 4 * t);
                        }
                    }
                }
            }
            let closure = plan
                .star(&relation, FixedPointLimits::default(), &())
                .unwrap();
            assert_eq!(
                closure.region().align_to(pair.space(), &()).unwrap(),
                table(pair.space(), closed)
            );
            for goal in [1, 3, 6, 9, 15] {
                let expected = iterate(0, |x| {
                    (goal & support) | modal_mask(edges, x, support, 4, true, true)
                });
                let actual = relation
                    .inevitably_reach(&table(&source, goal), FixedPointLimits::default(), &())
                    .unwrap();
                assert_eq!(members(actual.event(), support, 4), expected);
            }
        }
    }
}

#[test]
fn shared_environments_remain_separate_through_star() {
    let states = space(2);
    let environment = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let base = CoordinateMap::coordinates(&states, &environment, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = FibreProduct::new(SpaceId([3; 32]), &base, &base, &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
    let r = WorldRelation::new(&pair, &table(pair.space(), (1 << 8) | (1 << 7)), &()).unwrap(); // 0→2,3→1
    let star_program = plan.star_program(&r, &()).unwrap();
    assert_eq!(star_program.carrier().worlds(), 8);
    let closure = plan.star(&r, FixedPointLimits::default(), &()).unwrap();
    assert_eq!(closure.region().count(&()).unwrap(), 6);
    assert_eq!(closure.region().contains(4), Err(Error::IllegalWorld(4))); // 0→1, foreign environment
    assert!(closure.region().contains(8).unwrap());
    assert!(closure.region().contains(7).unwrap());
}

#[test]
fn fixed_point_helpers_refuse_changed_environment_roles_before_empty_shortcuts() {
    let states = space(2);
    let environment = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let base = [0, 1].map(|coordinate| {
        CoordinateMap::coordinates(&states, &environment, &[coordinate], &())
            .unwrap()
            .certify_surjective(&())
            .unwrap()
    });
    let st = FibreProduct::new(SpaceId([3; 32]), &base[0], &base[1], &()).unwrap();
    let tu = FibreProduct::new(SpaceId([4; 32]), &base[1], &base[0], &()).unwrap();
    let su = FibreProduct::new(SpaceId([5; 32]), &base[0], &base[0], &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([6; 32]), &st, &tu, &su, &()).unwrap();
    let relation = WorldRelation::new(&st, &st.space().empty(), &()).unwrap();
    assert_eq!(
        plan.star_program(&relation, &()).unwrap_err(),
        Error::EnvironmentMismatch
    );
    let limits = FixedPointLimits::default();
    for result in [
        relation.can_reach(&states.empty(), limits, &()),
        relation.inevitably_reach(&states.empty(), limits, &()),
        relation.safe_throughout(&states.full(), limits, &()),
    ] {
        assert_eq!(result.unwrap_err(), Error::EnvironmentMismatch);
    }
    let endorelation = WorldRelation::new(&su, &su.space().empty(), &()).unwrap();
    assert_eq!(
        endorelation
            .can_reach(&environment.empty(), limits, &())
            .unwrap_err(),
        Error::SpaceMismatch
    );
}

#[test]
fn program_validation_includes_binders_contexts_and_unsafe_variance() {
    let source = space(2);
    let other = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    let mut builder = EventProgramBuilder::new(&source, &()).unwrap();
    let foreign = EventProgramBuilder::new(&source, &()).unwrap();
    assert!(matches!(
        builder.apply(BoolOp4::FALSE, &builder.input(), &foreign.input(), &()),
        Err(Error::ProgramMismatch)
    ));
    let wrong = builder.constant(&other.empty(), &()).unwrap();
    assert!(matches!(
        builder.apply(BoolOp4::TRUE, &builder.input(), &wrong, &()),
        Err(Error::SpaceMismatch)
    ));
    let input = builder.input();
    let neg = builder.complement(&input, &()).unwrap();
    let double = builder.complement(&neg, &()).unwrap();
    let mut negative = EventProgramBuilder::new(&source, &()).unwrap();
    let n = negative.complement(&negative.input(), &()).unwrap();
    let negative = negative.finish(&n, &()).unwrap();
    assert_eq!(negative.variance(), Variance::Decreasing);
    assert_eq!(
        negative.fixed_points(&()).unwrap_err(),
        Error::NonMonotoneProgram
    );
    assert_eq!(
        negative.evaluate(&other.full(), &()),
        Err(Error::SpaceMismatch)
    );
    let double = builder
        .finish(&double, &())
        .unwrap()
        .fixed_points(&())
        .unwrap();
    assert!(
        double
            .least(FixedPointLimits::default(), &())
            .unwrap()
            .event()
            .is_empty()
    );
    assert!(
        double
            .greatest(FixedPointLimits::default(), &())
            .unwrap()
            .event()
            .is_full()
    );
    let mut wrong_end = EventProgramBuilder::new(&source, &()).unwrap();
    let root = wrong_end.constant(&other.empty(), &()).unwrap();
    assert_eq!(
        wrong_end
            .finish(&root, &())
            .unwrap()
            .fixed_points(&())
            .unwrap_err(),
        Error::SpaceMismatch
    );
}

#[test]
fn independent_owners_conditionals_maps_and_pruned_instruction_views_are_checked() {
    let source = space(2);
    let independently_owned =
        Event::from_bytes(&table(&source, 3).to_bytes(&()).unwrap(), &()).unwrap();
    let mut builder = EventProgramBuilder::new(&source, &()).unwrap();
    let unused = builder.complement(&builder.input(), &()).unwrap();
    let condition = builder.constant(&independently_owned, &()).unwrap();
    let full = builder.constant(&source.full(), &()).unwrap();
    let root = builder
        .ite(&condition, &builder.input(), &full, &())
        .unwrap();
    let program = builder.finish(&root, &()).unwrap();
    drop(unused);
    assert_eq!(program.instructions().len(), 4);
    assert!(
        program
            .instructions()
            .iter()
            .all(|i| !matches!(i.operation(), ProgramOp::Not(_)))
    );
    for mask in 0..16 {
        let out = program.evaluate(&table(&source, mask), &()).unwrap();
        assert_eq!(members(&out, 15, 4), (mask & 3) | 0b1100);
    }
    assert_eq!(
        program
            .fixed_points(&())
            .unwrap()
            .least(FixedPointLimits::default(), &())
            .unwrap()
            .event()
            .to_bytes(&())
            .unwrap(),
        table(&source, 12).to_bytes(&()).unwrap()
    );
    let target = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let readout = CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap();
    for operation in [
        MapOp::Image,
        MapOp::UniversalImage,
        MapOp::NonvacuousImage,
        MapOp::Possible,
        MapOp::Guaranteed,
    ] {
        let mut builder = EventProgramBuilder::new(&source, &()).unwrap();
        let root = builder
            .map(operation, &readout, &builder.input(), &())
            .unwrap();
        let program = builder.finish(&root, &()).unwrap();
        assert_eq!(program.variance(), Variance::Increasing);
        let input = table(&source, 3);
        let expected = match operation {
            MapOp::Image => readout.image(&input, &()).unwrap(),
            MapOp::UniversalImage => readout.universal_image(&input, &()).unwrap(),
            MapOp::NonvacuousImage => readout.nonvacuous_image(&input, &()).unwrap(),
            MapOp::Possible => readout.possible(&input, &()).unwrap(),
            MapOp::Guaranteed => readout.guaranteed(&input, &()).unwrap(),
            MapOp::Pullback => unreachable!(),
        };
        assert_eq!(program.evaluate(&input, &()).unwrap(), expected);
    }
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
fn modal_instructions_keep_both_direction_and_all_participating_nodes() {
    let source = space(2);
    let pairs = pair(&source);
    let relation =
        WorldRelation::new(&pairs, &table(pairs.space(), (1 << 4) | (1 << 14)), &()).unwrap();
    for mode in [ModalOp::May, ModalOp::All, ModalOp::Must, ModalOp::Post] {
        let mut builder = EventProgramBuilder::new(&source, &()).unwrap();
        let step = builder
            .modal(mode, &relation, &builder.input(), &())
            .unwrap();
        let program = builder.finish(&step, &()).unwrap();
        for mask in 0..16 {
            let input = table(&source, mask);
            let expected = match mode {
                ModalOp::May => relation.may(&input, &()).unwrap(),
                ModalOp::All => relation.all(&input, &()).unwrap(),
                ModalOp::Must => relation.must(&input, &()).unwrap(),
                ModalOp::Post => relation.post(&input, &()).unwrap(),
            };
            assert_eq!(program.evaluate(&input, &()).unwrap(), expected);
        }
    }
    let mut builder = EventProgramBuilder::new(&source, &()).unwrap();
    let step = builder
        .modal(ModalOp::May, &relation, &builder.input(), &())
        .unwrap();
    let constant = builder
        .apply(BoolOp4::FALSE, &builder.input(), &step, &())
        .unwrap();
    let program = builder.finish(&constant, &()).unwrap();
    assert_eq!(program.instructions().len(), 3);
    let mut steps = 0;
    assert_eq!(
        program.evaluate_counted(&source.full(), &mut steps, 2, &()),
        Err(Error::Capacity(Capacity::ProgramSteps))
    );
    assert_eq!(steps, 2);
    assert!(program.evaluate(&source.full(), &()).unwrap().is_empty());
    assert_eq!(
        program.evaluate(&source.full(), &StopAfter(Cell::new(10))),
        Err(Error::Cancelled)
    );
}

#[test]
fn budget_exhaustion_never_returns_a_partial_fixed_point() {
    let source = space(0);
    assert!(matches!(
        EventProgramBuilder::with_limit(&source, 0, &()),
        Err(Error::Capacity(Capacity::ProgramNodes))
    ));
    let mut builder = EventProgramBuilder::with_limit(&source, 2, &()).unwrap();
    let root = builder.constant(&source.full(), &()).unwrap();
    assert!(matches!(
        builder.constant(&source.empty(), &()),
        Err(Error::Capacity(Capacity::ProgramNodes))
    ));
    let fixed = builder
        .finish(&root, &())
        .unwrap()
        .fixed_points(&())
        .unwrap();
    assert_eq!(fixed.carrier().worlds(), 1);
    assert_eq!(
        fixed
            .least(
                FixedPointLimits {
                    iterations: 1,
                    program_steps: 100
                },
                &()
            )
            .unwrap_err(),
        Error::Capacity(Capacity::FixedPointIterations)
    );
    assert_eq!(
        fixed
            .least(
                FixedPointLimits {
                    iterations: 2,
                    program_steps: 1
                },
                &()
            )
            .unwrap_err(),
        Error::Capacity(Capacity::ProgramSteps)
    );
    assert_eq!(
        fixed
            .least(FixedPointLimits::default(), &StopAfter(Cell::new(0)))
            .unwrap_err(),
        Error::Cancelled
    );
    let result = fixed
        .least(
            FixedPointLimits {
                iterations: 2,
                program_steps: 2,
            },
            &(),
        )
        .unwrap();
    assert_eq!(result.iterations(), 2);
    drop((source, fixed));
    assert!(result.event().is_full());
    assert_eq!(result.event().count(&()).unwrap(), 1);
}

#[test]
fn symbolic_programs_and_star_do_not_enumerate_their_finite_carriers() {
    let source = space(62);
    let mut order = (1..62).collect::<Vec<_>>();
    order.push(0);
    let rotate = CoordinateMap::coordinates(&source, &source, &order, &()).unwrap();
    let mut builder = EventProgramBuilder::new(&source, &()).unwrap();
    let seed = builder
        .constant(&source.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let moved = builder
        .map(MapOp::Pullback, &rotate, &builder.input(), &())
        .unwrap();
    let root = builder.apply(BoolOp4::OR, &seed, &moved, &()).unwrap();
    let fixed = builder
        .finish(&root, &())
        .unwrap()
        .fixed_points(&())
        .unwrap();
    assert_eq!(fixed.carrier().worlds(), 1 << 62);
    let result = fixed.least(FixedPointLimits::default(), &()).unwrap();
    assert_eq!(result.iterations(), 63);
    assert_eq!(result.event().count(&()).unwrap(), (1 << 62) - 1);
    let states = space(20);
    let pair = pair(&states);
    let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
    let identity = WorldRelation::identity(&pair, &()).unwrap();
    let prepared = plan.star_program(&identity, &()).unwrap();
    assert_eq!(prepared.carrier().worlds(), 1 << 40);
    let closed = prepared
        .least(
            FixedPointLimits {
                iterations: 2,
                program_steps: 100,
            },
            &(),
        )
        .unwrap();
    assert_eq!(closed.event(), identity.region());
    assert_eq!(closed.iterations(), 2);
    assert_eq!(plan.workspace().space().dimensions(), 60);
}
