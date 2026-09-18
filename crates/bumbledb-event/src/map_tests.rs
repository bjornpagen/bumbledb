use std::cell::Cell;

use crate::{BoolOp4, Capacity, Control, CoordinateMap, Error, Event, Limits, Space, SpaceId};

fn space(name: u8, dimensions: u8, support: u64) -> Space {
    let full = Space::new(SpaceId([name; 32]), dimensions, &()).unwrap();
    let legal = full.table((1 << dimensions) - 1, &[support], &()).unwrap();
    full.restrict(&legal, &()).unwrap()
}

fn table(space: &Space, data: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[data], &())
        .unwrap()
}

fn members(event: &Event, support: u64) -> u64 {
    (0..(1 << event.space().dimensions()))
        .filter(|&world| support & (1 << world) != 0 && event.contains(world).unwrap())
        .fold(0, |bits, world| bits | (1 << world))
}

#[test]
fn exhaustive_four_world_maps_on_every_source_support() {
    // All 256 functions, all 15 nonempty source supports, and every predicate.
    // Independent integer-set images/preimages are the oracle. Alternate exact
    // target image and the entire target universe; neither is assumed onto.
    for support in 1u64..16 {
        let source = space(1, 2, support);
        for function in 0u64..256 {
            let mapped = |world: u64| (function >> (2 * world)) & 3u64;
            let range = (0..4)
                .filter(|&world| support & (1 << world) != 0)
                .fold(0, |mask, world| mask | (1 << mapped(world)));
            let target_support = if function % 2 == 0 { range } else { 15 };
            let target = space(2, 2, target_support);
            let readouts: Vec<_> = (0..2)
                .map(|bit| {
                    table(
                        &source,
                        (0..4).fold(0, |mask, world| {
                            mask | (((mapped(world) >> bit) & 1) << world)
                        }),
                    )
                })
                .collect();
            let map = CoordinateMap::new(&source, &target, &readouts, &()).unwrap();
            assert_eq!(
                members(&map.support_image(&()).unwrap(), target_support),
                range
            );
            assert_eq!(map.certify_surjective(&()).is_ok(), range == target_support);
            for data in 0..16 {
                let preimage = map.pullback(&table(&target, data), &()).unwrap();
                let expected_preimage = (0..4)
                    .filter(|&world| {
                        support & (1 << world) != 0 && data & (1 << mapped(world)) != 0
                    })
                    .fold(0, |mask, world| mask | (1 << world));
                assert_eq!(members(&preimage, support), expected_preimage);
                let input = table(&source, data);
                let image = map.image(&input, &()).unwrap();
                let expected_image = (0..4)
                    .filter(|&world| (support & data) & (1 << world) != 0)
                    .fold(0, |mask, world| mask | (1 << mapped(world)));
                assert_eq!(members(&image, target_support), expected_image);
                let all = map.universal_image(&input, &()).unwrap();
                let must = map.nonvacuous_image(&input, &()).unwrap();
                let expected_all = (0..4)
                    .filter(|&y| {
                        target_support & (1 << y) != 0
                            && (0..4).all(|x| {
                                support & (1 << x) == 0 || mapped(x) != y || data & (1 << x) != 0
                            })
                    })
                    .fold(0, |mask, y| mask | (1 << y));
                assert_eq!(members(&all, target_support), expected_all);
                assert_eq!(members(&must, target_support), expected_all & range);
            }
        }
    }
}

#[test]
fn original_support_is_checked_before_target_completion() {
    let source = space(1, 1, 3);
    let target = space(2, 2, 0b1001);
    let x = source.coordinate(0, &()).unwrap();
    assert!(matches!(
        CoordinateMap::new(&source, &target, &[source.empty(), source.full()], &()),
        Err(Error::MapOutsideSupport)
    ));
    assert!(matches!(
        CoordinateMap::new(&source, &target, &[x.clone(), source.full()], &()),
        Err(Error::MapOutsideSupport)
    ));
    let copied = CoordinateMap::new(&source, &target, &[x.clone(), x], &()).unwrap();
    let onto = copied.certify_surjective(&()).unwrap();
    assert!(onto.map().support_image(&()).unwrap().is_full());
    assert_eq!(copied.map_world(0).unwrap(), 0);
    assert_eq!(copied.map_world(1).unwrap(), 3);
    assert_eq!(copied.map_world(2), Err(Error::IllegalWorld(2)));
    // Onto individual readouts do not certify a complete product square.
    let pair = space(5, 2, 15);
    let duplicated = CoordinateMap::coordinates(&source, &pair, &[0, 0], &()).unwrap();
    assert_eq!(members(&duplicated.support_image(&()).unwrap(), 15), 9);
    assert!(matches!(
        duplicated.certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
    for bit in 0..2 {
        let projection = CoordinateMap::coordinates(&pair, &source, &[bit], &()).unwrap();
        assert!(
            duplicated
                .then(&projection, &())
                .unwrap()
                .certify_surjective(&())
                .is_ok()
        );
    }
    // Aliases for legal source code 1 must not create a code-0 image witness.
    let restricted = space(3, 1, 2);
    let target = space(4, 1, 3);
    let map = CoordinateMap::coordinates(&restricted, &target, &[0], &()).unwrap();
    assert_eq!(members(&map.support_image(&()).unwrap(), 3), 2);
    assert!(matches!(
        map.certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
}

#[test]
fn pullback_is_boolean_but_image_has_only_its_adjoint_laws() {
    let source = space(1, 2, 0b1110);
    let target = space(2, 1, 3);
    let map = CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap();
    for a in 0u64..16 {
        let a = table(&source, a);
        let image = map.image(&a, &()).unwrap();
        let all = map.universal_image(&a, &()).unwrap();
        for b in 0u64..4 {
            let b = table(&target, b);
            let pull = map.pullback(&b, &()).unwrap();
            assert_eq!(
                image.signature(&b, &()).unwrap().included(),
                a.signature(&pull, &()).unwrap().included()
            );
            assert_eq!(
                pull.signature(&a, &()).unwrap().included(),
                b.signature(&all, &()).unwrap().included()
            );
            let inside = map
                .image(&a.apply(BoolOp4::AND, &pull, &()).unwrap(), &())
                .unwrap();
            assert_eq!(inside, image.apply(BoolOp4::AND, &b, &()).unwrap());
            assert_eq!(
                map.pullback(&b.complement(), &()).unwrap(),
                pull.complement()
            );
        }
    }
    for a in 0..4 {
        for b in 0..4 {
            let a = table(&target, a);
            let b = table(&target, b);
            for code in 0..16 {
                let op = BoolOp4::new(code).unwrap();
                assert_eq!(
                    map.pullback(&a.apply(op, &b, &()).unwrap(), &()).unwrap(),
                    map.pullback(&a, &())
                        .unwrap()
                        .apply(op, &map.pullback(&b, &()).unwrap(), &())
                        .unwrap()
                );
            }
        }
    }
    let a = table(&source, 2);
    let b = table(&source, 8);
    assert!(
        map.image(&a.apply(BoolOp4::AND, &b, &()).unwrap(), &())
            .unwrap()
            .is_empty()
    );
    assert!(
        !map.image(&a, &())
            .unwrap()
            .apply(BoolOp4::AND, &map.image(&b, &()).unwrap(), &())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn composition_and_resident_simultaneous_substitution_keep_readouts_distinct() {
    let source = space(1, 3, 255);
    let x = source.coordinate(0, &()).unwrap();
    let y = source.coordinate(1, &()).unwrap();
    let z = source.coordinate(2, &()).unwrap();
    let nonlinear = x.apply(BoolOp4::XOR, &y, &()).unwrap();
    let first =
        CoordinateMap::new(&source, &source, &[y.clone(), x.clone(), nonlinear], &()).unwrap();
    let second = CoordinateMap::new(&source, &source, &[z, x, y], &()).unwrap();
    let composed = first.then(&second, &()).unwrap();
    let identity = CoordinateMap::identity(&source, &()).unwrap();
    assert!(
        first
            .then(&identity, &())
            .unwrap()
            .equivalent(&first, &())
            .unwrap()
    );
    assert!(
        identity
            .then(&first, &())
            .unwrap()
            .equivalent(&first, &())
            .unwrap()
    );
    assert!(!first.equivalent(&second, &()).unwrap());
    for bits in 0u64..256 {
        let event = table(&source, bits);
        assert_eq!(
            composed.pullback(&event, &()).unwrap(),
            first
                .pullback(&second.pullback(&event, &()).unwrap(), &())
                .unwrap()
        );
        assert_eq!(
            composed.image(&event, &()).unwrap(),
            second
                .image(&first.image(&event, &()).unwrap(), &())
                .unwrap()
        );
        assert_eq!(identity.pullback(&event, &()).unwrap(), event);
        assert_eq!(identity.image(&event, &()).unwrap(), event);
    }
    for world in 0..8 {
        assert_eq!(
            composed.map_world(world).unwrap(),
            second.map_world(first.map_world(world).unwrap()).unwrap()
        );
    }
}

#[test]
fn wide_maps_do_not_require_a_combined_workspace_or_world_enumeration() {
    let limits = Limits {
        operation_steps: 200_000,
        ..Limits::default()
    };
    let source =
        Space::with_order(SpaceId([1; 32]), &(0..62).collect::<Vec<_>>(), limits, &()).unwrap();
    let target = Space::with_order(
        SpaceId([2; 32]),
        &(0..62).rev().collect::<Vec<_>>(),
        limits,
        &(),
    )
    .unwrap();
    let identity =
        CoordinateMap::coordinates(&source, &target, &(0..62).collect::<Vec<_>>(), &()).unwrap();
    assert!(identity.support_image(&()).unwrap().is_full());
    let event = source
        .coordinate(61, &())
        .unwrap()
        .apply(BoolOp4::XOR, &source.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let image = identity.image(&event, &()).unwrap();
    assert_eq!(image.count(&()).unwrap(), 1 << 61);
    assert_eq!(identity.pullback(&image, &()).unwrap(), event);
    let unit = Space::new(SpaceId([3; 32]), 0, &()).unwrap();
    let terminal = CoordinateMap::new(&source, &unit, &[], &()).unwrap();
    assert!(terminal.image(&event, &()).unwrap().is_full());
    assert!(terminal.image(&source.empty(), &()).unwrap().is_empty());
    assert!(terminal.pullback(&unit.full(), &()).unwrap().is_full());
    let constant = CoordinateMap::new(&unit, &target, &vec![unit.full(); 62], &()).unwrap();
    let singleton = constant.support_image(&()).unwrap();
    assert_eq!(singleton.count(&()).unwrap(), 1);
    assert!(singleton.contains((1 << 62) - 1).unwrap());
}

#[test]
fn symbolic_nonlinear_maps_across_arena_orders_match_an_explicit_oracle() {
    let source = Space::with_order(
        SpaceId([1; 32]),
        &(0..11).rev().collect::<Vec<_>>(),
        Limits::default(),
        &(),
    )
    .unwrap();
    let target = Space::new(SpaceId([2; 32]), 12, &()).unwrap();
    let variables: Vec<_> = (0..11)
        .map(|bit| source.coordinate(bit, &()).unwrap())
        .collect();
    let mut readouts = variables.clone();
    let parity = variables
        .iter()
        .try_fold(source.empty(), |p, bit| p.apply(BoolOp4::XOR, bit, &()))
        .unwrap();
    readouts.push(parity.clone());
    readouts[3] = variables[0]
        .apply(BoolOp4::XOR, &variables[3], &())
        .unwrap();
    readouts[8] = variables[7]
        .apply(BoolOp4::AND, &variables[8], &())
        .unwrap();
    let map = CoordinateMap::new(&source, &target, &readouts, &()).unwrap();
    let mut words = [0u64; 64];
    for world in 0u64..4096 {
        if (world.wrapping_mul(157) ^ (world >> 3)).count_ones() % 3 == 0 {
            words[(world / 64) as usize] |= 1 << (world % 64);
        }
    }
    let test = target.table(4095, &words, &()).unwrap();
    let pull = map.pullback(&test, &()).unwrap();
    let image = map.image(&parity, &()).unwrap();
    let mut expected = [false; 4096];
    for world in 0u64..2048 {
        let mapped = (world & !(1 << 3 | 1 << 8))
            | (((world ^ (world << 3)) >> 3 & 1) << 3)
            | (((world >> 7 & 1) & (world >> 8 & 1)) << 8)
            | ((u64::from(world.count_ones()) % 2) << 11);
        assert_eq!(map.map_world(world).unwrap(), mapped);
        assert_eq!(
            pull.contains(world).unwrap(),
            test.contains(mapped).unwrap()
        );
        if world.count_ones() % 2 == 1 {
            expected[usize::try_from(mapped).unwrap()] = true;
        }
    }
    for (world, &present) in expected.iter().enumerate() {
        assert_eq!(image.contains(world as u64).unwrap(), present);
    }
}

#[test]
fn map_context_validation_includes_constants_and_shared_arena_restrictions() {
    let source = space(1, 2, 15);
    let target = space(2, 1, 3);
    let foreign = space(3, 2, 15);
    assert!(matches!(
        CoordinateMap::new(&source, &target, &[], &()),
        Err(Error::MapArity)
    ));
    assert!(matches!(
        CoordinateMap::new(&source, &target, &[foreign.empty()], &()),
        Err(Error::SpaceMismatch)
    ));
    assert!(matches!(
        CoordinateMap::coordinates(&source, &target, &[2], &()),
        Err(Error::InvalidCoordinate(2))
    ));
    let map = CoordinateMap::coordinates(&source, &target, &[1], &()).unwrap();
    assert_eq!(map.image(&foreign.empty(), &()), Err(Error::SpaceMismatch));
    assert_eq!(
        map.pullback(&foreign.full(), &()),
        Err(Error::SpaceMismatch)
    );
    let unit = Space::new(SpaceId([4; 32]), 0, &()).unwrap();
    let other = CoordinateMap::new(&foreign, &unit, &[], &()).unwrap();
    assert!(matches!(map.then(&other, &()), Err(Error::SpaceMismatch)));
    let restricted = source.restrict(&table(&source, 9), &()).unwrap();
    let inclusion = CoordinateMap::coordinates(&restricted, &source, &[0, 1], &()).unwrap();
    assert_eq!(members(&inclusion.support_image(&()).unwrap(), 15), 9);
    let swap = CoordinateMap::coordinates(&restricted, &restricted, &[1, 0], &()).unwrap();
    assert!(swap.support_image(&()).unwrap().is_full());
    assert!(matches!(
        CoordinateMap::coordinates(&source, &restricted, &[0, 1], &()),
        Err(Error::MapOutsideSupport)
    ));
    // Independently decoded equivalent contexts are explicitly aligned.
    let foreign_owner = Event::from_bytes(&table(&source, 6).to_bytes(&()).unwrap(), &()).unwrap();
    assert_eq!(
        map.image(&foreign_owner, &()).unwrap(),
        map.image(&table(&source, 6), &()).unwrap()
    );
    let copy = CoordinateMap::new(
        &source,
        &target,
        &[Event::from_bytes(
            &source.coordinate(1, &()).unwrap().to_bytes(&()).unwrap(),
            &(),
        )
        .unwrap()],
        &(),
    )
    .unwrap();
    assert!(copy.equivalent(&map, &()).unwrap());
}

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> crate::Result<()> {
        let remaining = self.0.get();
        if remaining == 0 {
            return Err(Error::Cancelled);
        }
        self.0.set(remaining - 1);
        Ok(())
    }
}

#[test]
fn checked_maps_refuse_resources_and_retain_owned_results() {
    let source = space(1, 2, 15);
    let target = space(2, 1, 3);
    let map = CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap();
    for budget in [0, 3, 8] {
        assert_eq!(
            map.image(&table(&source, 6), &Stop(Cell::new(budget))),
            Err(Error::Cancelled)
        );
        assert_eq!(
            map.pullback(
                &target.coordinate(0, &()).unwrap(),
                &Stop(Cell::new(budget))
            ),
            Err(Error::Cancelled)
        );
    }
    assert!(matches!(
        CoordinateMap::identity(&source, &Stop(Cell::new(0))),
        Err(Error::Cancelled)
    ));
    let tiny = Space::with_order(
        SpaceId([3; 32]),
        &[0],
        Limits {
            operation_steps: 0,
            ..Limits::default()
        },
        &(),
    )
    .unwrap();
    assert!(matches!(
        CoordinateMap::new(&tiny, &target, &[tiny.full()], &()),
        Err(Error::Capacity(Capacity::OperationSteps))
    ));
    let result = map.image(&table(&source, 10), &()).unwrap();
    let pulled = map
        .pullback(&target.coordinate(0, &()).unwrap(), &())
        .unwrap();
    drop(source);
    drop(target);
    drop(map);
    assert_eq!(result.count(&()).unwrap(), 1);
    assert_eq!(pulled.count(&()).unwrap(), 2);
}

#[test]
fn opposite_direction_maps_share_the_alignment_lock_order() {
    let left = space(1, 3, 255);
    let right = space(2, 3, 255);
    let forward = CoordinateMap::coordinates(&left, &right, &[2, 0, 1], &()).unwrap();
    let reverse = CoordinateMap::coordinates(&right, &left, &[1, 2, 0], &()).unwrap();
    std::thread::scope(|scope| {
        for (map, input) in [
            (forward, table(&left, 0x35)),
            (reverse, table(&right, 0x69)),
        ] {
            scope.spawn(move || {
                for _ in 0..32 {
                    let image = map.image(&input, &()).unwrap();
                    assert_eq!(map.pullback(&image, &()).unwrap(), input);
                }
            });
        }
    });
}
