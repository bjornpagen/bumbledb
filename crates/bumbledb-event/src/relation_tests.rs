use crate::{
    BoolOp4, Capacity, Control, CoordinateMap, Error, Event, FaceProduct, FibreProduct, Limits,
    RelationalProduct, Space, SpaceId, SurjectiveMap, WorldRelation,
};

fn space(name: u8, dimensions: u8, support: u64) -> Space {
    let raw = Space::new(SpaceId([name; 32]), dimensions, &()).unwrap();
    raw.restrict(
        &raw.table((1 << dimensions) - 1, &[support], &()).unwrap(),
        &(),
    )
    .unwrap()
}

fn base(source: &Space, environment: &Space, coordinates: &[u8]) -> SurjectiveMap {
    CoordinateMap::coordinates(source, environment, coordinates, &())
        .unwrap()
        .certify_surjective(&())
        .unwrap()
}

fn product(name: u8, left: &SurjectiveMap, right: &SurjectiveMap) -> FibreProduct {
    FibreProduct::new(SpaceId([name; 32]), left, right, &()).unwrap()
}

fn event(space: &Space, mask: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[mask], &())
        .unwrap()
}

fn relation(product: &FibreProduct, mask: u64) -> WorldRelation {
    // Test matrices use the original product's semantic tuple order.
    WorldRelation::new(product, &event(product.space(), mask), &()).unwrap()
}

fn matrix(relation: &WorldRelation) -> u64 {
    let product = relation.product();
    assert!(product.space().dimensions() <= 6);
    let mut bits = 0;
    for code in 0..(1 << product.space().dimensions()) {
        if relation.region().contains(code) == Ok(true) {
            let s = product.left().map().map_world(code).unwrap();
            let t = product.right().map().map_world(code).unwrap();
            bits |= 1 << (s | (t << relation.input().dimensions()));
        }
    }
    bits
}

fn members(event: &Event) -> u64 {
    (0..(1 << event.space().dimensions()))
        .filter(|&world| event.contains(world) == Ok(true))
        .fold(0, |bits, world| bits | (1 << world))
}

fn bit(matrix: u64, width: u8, s: u64, t: u64) -> bool {
    matrix & (1 << (s | (t << width))) != 0
}

#[test]
fn products_admit_exact_asymmetric_fibres_and_preserve_one_environment() {
    let environment = space(1, 1, 3);
    let left = space(2, 2, 0b1011);
    let middle = space(3, 3, 0b0111_1110);
    let right = space(4, 2, 0b1101);
    let bases = [
        base(&left, &environment, &[0]),
        base(&middle, &environment, &[0]),
        base(&right, &environment, &[0]),
    ];
    let faces = FaceProduct::new(SpaceId([5; 32]), &bases, &()).unwrap();
    assert_eq!(faces.space().full().count(&()).unwrap(), 12);
    for code in 0u64..128 {
        let s = code & 3;
        let t = code >> 2 & 7;
        let u = code >> 5;
        let legal = 0b1011 & (1 << s) != 0
            && 0b0111_1110 & (1 << t) != 0
            && 0b1101 & (1 << u) != 0
            && s & 1 == t & 1
            && t & 1 == u & 1;
        assert_eq!(faces.space().full().contains(code).is_ok(), legal);
        if legal {
            for (projection, expected) in faces.projections().iter().zip([s, t, u]) {
                assert_eq!(projection.map().map_world(code).unwrap(), expected);
            }
        }
    }
    let pair = product(6, &bases[0], &bases[1]);
    assert_eq!(pair.space().full().count(&()).unwrap(), 9);
    for code in 0u64..32 {
        let s = code & 3;
        let t = code >> 2;
        let legal = 0b1011 & (1 << s) != 0 && 0b0111_1110 & (1 << t) != 0 && s & 1 == t & 1;
        assert_eq!(pair.space().full().contains(code).is_ok(), legal);
    }
    let square = pair
        .certify_square(
            faces.projections()[0].map(),
            faces.projections()[1].map(),
            &(),
        )
        .unwrap();
    assert!(square.joint().map().support_image(&()).unwrap().is_full());
    assert_eq!(square.product().space().identity(), pair.space().identity());
    assert_eq!(square.left().target().identity(), left.identity());
    assert_eq!(square.right().target().identity(), middle.identity());
}

#[test]
fn nonlinear_environment_readouts_and_copied_bit_squares_are_distinct() {
    let environment = space(1, 1, 3);
    let endpoint = space(2, 2, 15);
    let parity = endpoint
        .coordinate(0, &())
        .unwrap()
        .apply(BoolOp4::XOR, &endpoint.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let view = CoordinateMap::new(&endpoint, &environment, &[parity], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = product(3, &view, &view);
    assert_eq!(pair.space().full().count(&()).unwrap(), 8);
    for code in 0u64..16 {
        let s = code & 3;
        let t = code >> 2;
        assert_eq!(
            pair.space().full().contains(code).is_ok(),
            s.count_ones() % 2 == t.count_ones() % 2
        );
    }
    let identity = CoordinateMap::identity(&endpoint, &()).unwrap();
    assert!(identity.certify_surjective(&()).is_ok());
    assert!(matches!(
        pair.certify_square(&identity, &identity, &()),
        Err(Error::IncompleteImage)
    ));
    // Complete fibres allow several lifts; uniqueness is not required.
    let unit = space(4, 0, 1);
    let a = base(&environment, &unit, &[]);
    let cartesian = product(5, &a, &a);
    let extra = space(6, 3, 255);
    let first = CoordinateMap::coordinates(&extra, &environment, &[0], &()).unwrap();
    let second = CoordinateMap::coordinates(&extra, &environment, &[1], &()).unwrap();
    assert!(cartesian.certify_square(&first, &second, &()).is_ok());
    assert!(matches!(
        cartesian.certify_square(&first, &first, &()),
        Err(Error::IncompleteImage)
    ));
    // Noncommuting maps cannot become a valid square by target clipping.
    let single = product(
        7,
        &base(&environment, &environment, &[0]),
        &base(&environment, &environment, &[0]),
    );
    let flip = CoordinateMap::new(
        &environment,
        &environment,
        &[environment.coordinate(0, &()).unwrap().complement()],
        &(),
    )
    .unwrap();
    assert!(matches!(
        single.pair(
            &CoordinateMap::identity(&environment, &()).unwrap(),
            &flip,
            &()
        ),
        Err(Error::MapOutsideSupport)
    ));
}

#[test]
fn every_two_state_relation_matches_matrix_modal_and_residual_oracles() {
    let unit = space(1, 0, 1);
    let sources: Vec<_> = (2..5).map(|name| space(name, 1, 3)).collect();
    let bases: Vec<_> = sources.iter().map(|s| base(s, &unit, &[])).collect();
    let left = product(5, &bases[0], &bases[1]);
    let right = product(6, &bases[1], &bases[2]);
    let result = product(7, &bases[0], &bases[2]);
    let plan = RelationalProduct::new(SpaceId([8; 32]), &left, &right, &result, &()).unwrap();
    let mut composed = [[0; 16]; 16];
    let mut left_residual = [[0; 16]; 16];
    let mut right_residual = [[0; 16]; 16];
    for r in 0u64..16 {
        let r_index = usize::try_from(r).unwrap();
        let rel = relation(&left, r);
        assert_eq!(
            matrix(&rel.converse()),
            (r & 9) | ((r & 2) << 1) | ((r & 4) >> 1)
        );
        assert!(rel.equivalent(&rel.converse().converse(), &()).unwrap());
        let domain = (0..2)
            .filter(|&s| (0..2).any(|t| bit(r, 1, s, t)))
            .fold(0, |b, s| b | (1 << s));
        let range = (0..2)
            .filter(|&t| (0..2).any(|s| bit(r, 1, s, t)))
            .fold(0, |b, t| b | (1 << t));
        assert_eq!(members(&rel.domain(&()).unwrap()), domain);
        assert_eq!(members(&rel.range(&()).unwrap()), range);
        assert_eq!(matrix(&rel.complement()), 15 ^ r);
        for p in 0u64..4 {
            let target = event(&sources[1], p);
            let expected_may = (0..2)
                .filter(|&s| (0..2).any(|t| bit(r, 1, s, t) && p & (1 << t) != 0))
                .fold(0, |b, s| b | (1 << s));
            let expected_all = (0..2)
                .filter(|&s| (0..2).all(|t| !bit(r, 1, s, t) || p & (1 << t) != 0))
                .fold(0, |b, s| b | (1 << s));
            assert_eq!(members(&rel.may(&target, &()).unwrap()), expected_may);
            assert_eq!(members(&rel.all(&target, &()).unwrap()), expected_all);
            assert_eq!(
                members(&rel.must(&target, &()).unwrap()),
                expected_all & domain
            );
            assert_eq!(
                rel.post(&event(&sources[0], p), &()).unwrap(),
                rel.converse().may(&event(&sources[0], p), &()).unwrap()
            );
        }
        for q in 0u64..16 {
            let q_index = usize::try_from(q).unwrap();
            let composed_rel = plan.compose(&rel, &relation(&right, q), &()).unwrap();
            let expected = (0..4)
                .filter(|&pair| (0..2).any(|t| bit(r, 1, pair & 1, t) && bit(q, 1, t, pair >> 1)))
                .fold(0, |bits, pair| bits | (1 << pair));
            let actual = matrix(&composed_rel);
            assert_eq!(actual, expected);
            composed[r_index][q_index] = actual;
            let bound = relation(&result, q);
            let l = plan.left_residual(&rel, &bound, &()).unwrap();
            let expected_l = (0..4)
                .filter(|&pair| (0..2).all(|s| !bit(r, 1, s, pair & 1) || bit(q, 1, s, pair >> 1)))
                .fold(0, |bits, pair| bits | (1 << pair));
            assert_eq!(matrix(&l), expected_l);
            left_residual[r_index][q_index] = matrix(&l);
            let rr = plan
                .right_residual(&relation(&result, r), &relation(&right, q), &())
                .unwrap();
            let expected_r = (0..4)
                .filter(|&pair| (0..2).all(|u| !bit(q, 1, pair >> 1, u) || bit(r, 1, pair & 1, u)))
                .fold(0, |bits, pair| bits | (1 << pair));
            assert_eq!(matrix(&rr), expected_r);
            right_residual[r_index][q_index] = matrix(&rr);
        }
    }
    // Every R,Q,V triple tests both adjunctions using independently checked matrices.
    for (r, row) in composed.iter().enumerate() {
        for (q, &composition) in row.iter().enumerate() {
            for (v, residual_row) in right_residual.iter().enumerate() {
                assert_eq!(
                    composition & !(v as u64) == 0,
                    q as u64 & !left_residual[r][v] == 0
                );
                assert_eq!(
                    composition & !(v as u64) == 0,
                    r as u64 & !residual_row[q] == 0
                );
            }
        }
    }
}

#[test]
fn asymmetric_shared_environment_relational_products_match_explicit_matrices() {
    let environment = space(1, 1, 3);
    let input = space(2, 2, 0b1011);
    let middle = space(3, 3, 0b0111_1110);
    let output = space(4, 2, 0b1101);
    let bases = [
        base(&input, &environment, &[0]),
        base(&middle, &environment, &[0]),
        base(&output, &environment, &[0]),
    ];
    let left = product(5, &bases[0], &bases[1]);
    let right = product(6, &bases[1], &bases[2]);
    let result = product(7, &bases[0], &bases[2]);
    let plan = RelationalProduct::new(SpaceId([8; 32]), &left, &right, &result, &()).unwrap();
    let legal_result = matrix(&relation(&result, u64::from(u16::MAX)));
    let mut random = 12345u64;
    for _ in 0..32 {
        random = random
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let r = relation(&left, random & u64::from(u32::MAX));
        random = random.rotate_left(17).wrapping_mul(93);
        let q = relation(&right, random & u64::from(u32::MAX));
        let v = relation(&result, random >> 32 & u64::from(u16::MAX));
        let rm = matrix(&r);
        let qm = matrix(&q);
        let vm = matrix(&v);
        let composition = plan.compose(&r, &q, &()).unwrap();
        let mut expected = 0;
        for pair in 0..16 {
            if legal_result & (1 << pair) != 0
                && (0..8)
                    .any(|middle| bit(rm, 2, pair & 3, middle) && bit(qm, 3, middle, pair >> 2))
            {
                expected |= 1 << pair;
            }
        }
        assert_eq!(matrix(&composition), expected);
        let lr = plan.left_residual(&r, &v, &()).unwrap();
        let rr = plan.right_residual(&v, &q, &()).unwrap();
        let left_full = matrix(&relation(&left, u64::from(u32::MAX)));
        let right_full = matrix(&relation(&right, u64::from(u32::MAX)));
        let expected_l = (0..32)
            .filter(|&pair| {
                right_full & (1 << pair) != 0
                    && (0..4)
                        .all(|state| !bit(rm, 2, state, pair & 7) || bit(vm, 2, state, pair >> 3))
            })
            .fold(0, |bits, pair| bits | (1 << pair));
        let expected_r = (0..32)
            .filter(|&pair| {
                left_full & (1 << pair) != 0
                    && (0..4)
                        .all(|state| !bit(qm, 3, pair >> 2, state) || bit(vm, 2, pair & 3, state))
            })
            .fold(0, |bits, pair| bits | (1 << pair));
        assert_eq!(matrix(&lr), expected_l);
        assert_eq!(matrix(&rr), expected_r);
        assert_eq!(
            composition.included_in(&v, &()).unwrap(),
            q.included_in(&lr, &()).unwrap()
        );
        assert_eq!(
            composition.included_in(&v, &()).unwrap(),
            r.included_in(&rr, &()).unwrap()
        );
    }
}

#[test]
fn shared_middle_and_environment_cannot_be_projected_independently() {
    let environment = space(1, 1, 3);
    let input = space(2, 2, 15);
    let middle = space(3, 2, 15);
    let output = space(4, 2, 15);
    let bases = [
        base(&input, &environment, &[0]),
        base(&middle, &environment, &[0]),
        base(&output, &environment, &[0]),
    ];
    let left = product(5, &bases[0], &bases[1]);
    let right = product(6, &bases[1], &bases[2]);
    let result = product(7, &bases[0], &bases[2]);
    let plan = RelationalProduct::new(SpaceId([8; 32]), &left, &right, &result, &()).unwrap();
    let t_low = middle.coordinate(1, &()).unwrap().complement();
    let t_high = t_low.complement();
    let r = WorldRelation::new(
        &left,
        &left.right().map().pullback(&t_low, &()).unwrap(),
        &(),
    )
    .unwrap();
    let q = WorldRelation::new(
        &right,
        &right.left().map().pullback(&t_high, &()).unwrap(),
        &(),
    )
    .unwrap();
    assert!(r.domain(&()).unwrap().is_full());
    assert!(q.range(&()).unwrap().is_full());
    assert!(plan.compose(&r, &q, &()).unwrap().region().is_empty());
    let low_environment = middle.coordinate(0, &()).unwrap().complement();
    let r = WorldRelation::new(
        &left,
        &left.right().map().pullback(&low_environment, &()).unwrap(),
        &(),
    )
    .unwrap();
    let q = WorldRelation::new(
        &right,
        &right
            .left()
            .map()
            .pullback(&low_environment.complement(), &())
            .unwrap(),
        &(),
    )
    .unwrap();
    assert!(plan.compose(&r, &q, &()).unwrap().region().is_empty());
    // Equal environment ranges do not make two different environment maps equal.
    let inverted = CoordinateMap::new(
        &middle,
        &environment,
        &[middle.coordinate(0, &()).unwrap().complement()],
        &(),
    )
    .unwrap()
    .certify_surjective(&())
    .unwrap();
    let incompatible = product(9, &inverted, &bases[2]);
    assert!(matches!(
        RelationalProduct::new(SpaceId([10; 32]), &left, &incompatible, &result, &()),
        Err(Error::EnvironmentMismatch)
    ));
}

#[test]
fn role_checks_are_membership_dependencies_not_raw_coordinate_masks() {
    let unit = space(1, 0, 1);
    let bit_space = space(2, 1, 3);
    let b = base(&bit_space, &unit, &[]);
    let pair = product(3, &b, &b);
    let faces = FaceProduct::new(SpaceId([4; 32]), &[b.clone(), b.clone(), b], &()).unwrap();
    let view = pair
        .pair(
            faces.projections()[0].map(),
            faces.projections()[1].map(),
            &(),
        )
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let third = faces.projections()[2]
        .map()
        .pullback(&bit_space.coordinate(0, &()).unwrap(), &())
        .unwrap();
    assert!(matches!(
        WorldRelation::from_lifted(&pair, &view, &third, &()),
        Err(Error::RoleMismatch)
    ));
    for empty in [false, true] {
        let constant = if empty {
            faces.space().empty()
        } else {
            faces.space().full()
        };
        let role = WorldRelation::from_lifted(&pair, &view, &constant, &()).unwrap();
        assert_eq!(role.region().is_empty(), empty);
    }
    let original = relation(&pair, 6);
    let lifted = view.map().pullback(original.region(), &()).unwrap();
    assert!(
        WorldRelation::from_lifted(&pair, &view, &lifted, &())
            .unwrap()
            .equivalent(&original, &())
            .unwrap()
    );
    // A scratch coordinate determined by the shared environment is admissible.
    let identity = CoordinateMap::identity(&bit_space, &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let coupled_pair = product(5, &identity, &identity);
    let coupled = FaceProduct::new(
        SpaceId([6; 32]),
        &[identity.clone(), identity.clone(), identity],
        &(),
    )
    .unwrap();
    let pair_view = coupled_pair
        .pair(
            coupled.projections()[0].map(),
            coupled.projections()[1].map(),
            &(),
        )
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let third = coupled.space().coordinate(2, &()).unwrap();
    let descended = WorldRelation::from_lifted(&coupled_pair, &pair_view, &third, &()).unwrap();
    assert_eq!(descended.region().count(&()).unwrap(), 1);
}

#[test]
fn identity_tests_and_converse_compose_across_owned_presentations() {
    let unit = space(1, 0, 1);
    let endpoint = space(2, 2, 0b1101);
    let b = base(&endpoint, &unit, &[]);
    let pair = product(3, &b, &b);
    let decoded_endpoint = Event::from_bytes_with_order(
        &endpoint.full().to_bytes(&()).unwrap(),
        Some(&[1, 0]),
        Limits::default(),
        &(),
    )
    .unwrap()
    .space();
    let decoded_environment = Event::from_bytes(&unit.full().to_bytes(&()).unwrap(), &())
        .unwrap()
        .space();
    let decoded_base = base(&decoded_endpoint, &decoded_environment, &[]);
    let other = FibreProduct::with_order(
        SpaceId([4; 32]),
        &decoded_base,
        &decoded_base,
        &[3, 2, 1, 0],
        Limits::default(),
        &(),
    )
    .unwrap();
    let plan = RelationalProduct::new(SpaceId([5; 32]), &pair, &other, &pair, &()).unwrap();
    let identity = WorldRelation::identity(&pair, &()).unwrap();
    assert_eq!(identity.region().count(&()).unwrap(), 3);
    let test = WorldRelation::test(&other, &endpoint.coordinate(1, &()).unwrap(), &()).unwrap();
    for bits in [0, 1, 0x7ace, 0xffff] {
        let r = relation(&pair, bits);
        let rr = r.in_product(&other, &()).unwrap();
        assert!(r.equivalent(&rr, &()).unwrap());
        assert!(
            plan.compose(&identity, &rr, &())
                .unwrap()
                .equivalent(&r, &())
                .unwrap()
        );
        assert!(
            plan.compose(&r, &identity, &())
                .unwrap()
                .equivalent(&r, &())
                .unwrap()
        );
        assert!(
            plan.compose(&r, &test, &())
                .unwrap()
                .domain(&())
                .unwrap()
                .equivalent(&r.may(&endpoint.coordinate(1, &()).unwrap(), &()).unwrap())
                .unwrap()
        );
        let flipped_plan = RelationalProduct::new(
            SpaceId([6; 32]),
            &other.converse(),
            &pair.converse(),
            &pair.converse(),
            &(),
        )
        .unwrap();
        let composed = plan.compose(&r, &test, &()).unwrap();
        let converse = flipped_plan
            .compose(&test.converse(), &r.converse(), &())
            .unwrap();
        assert!(composed.converse().equivalent(&converse, &()).unwrap());
    }
}

#[test]
fn function_graphs_and_readouts_preserve_support_and_refuse_partiality() {
    let unit = space(1, 0, 1);
    let endpoint = space(2, 2, 0b1101);
    let b = base(&endpoint, &unit, &[]);
    let pair = product(3, &b, &b);
    let identity = WorldRelation::identity(&pair, &()).unwrap();
    // Graph of a many-to-one observation agrees with its map's image/preimage.
    let observation = space(7, 1, 3);
    let ob = base(&observation, &unit, &[]);
    let graph_product = product(8, &b, &ob);
    let map = CoordinateMap::coordinates(&endpoint, &observation, &[1], &()).unwrap();
    let graph = WorldRelation::graph(&graph_product, &map, &()).unwrap();
    assert!(graph.readout(&()).unwrap().equivalent(&map, &()).unwrap());
    assert!(graph.domain(&()).unwrap().is_full());
    assert_eq!(graph.range(&()).unwrap(), map.support_image(&()).unwrap());
    for mask in 0..4 {
        let e = event(&observation, mask);
        assert_eq!(graph.may(&e, &()).unwrap(), map.pullback(&e, &()).unwrap());
    }
    let identity_base = CoordinateMap::identity(&observation, &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let diagonal = product(9, &identity_base, &identity_base);
    let flip = CoordinateMap::new(
        &observation,
        &observation,
        &[observation.coordinate(0, &()).unwrap().complement()],
        &(),
    )
    .unwrap();
    assert!(matches!(
        WorldRelation::graph(&diagonal, &flip, &()),
        Err(Error::EnvironmentMismatch)
    ));
    let partial = relation(&pair, 0);
    assert!(matches!(partial.readout(&()), Err(Error::PartialRelation)));
    let nondeterministic = WorldRelation::new(&pair, &pair.space().full(), &()).unwrap();
    assert!(matches!(
        nondeterministic.readout(&()),
        Err(Error::NonFunctionalRelation)
    ));
    assert!(
        identity
            .readout(&())
            .unwrap()
            .equivalent(&CoordinateMap::identity(&endpoint, &()).unwrap(), &())
            .unwrap()
    );
}

#[test]
fn guarded_must_composition_keeps_dead_intermediate_states() {
    let unit = space(1, 0, 1);
    let bit_space = space(2, 1, 3);
    let b = base(&bit_space, &unit, &[]);
    let pair = product(3, &b, &b);
    let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
    for r in 0..16 {
        for q in 0..16 {
            let r = relation(&pair, r);
            let q = relation(&pair, q);
            let combined = plan.compose(&r, &q, &()).unwrap();
            for target in 0..4 {
                let e = event(&bit_space, target);
                let nested = r.must(&q.must(&e, &()).unwrap(), &()).unwrap();
                let guard = r.all(&q.domain(&()).unwrap(), &()).unwrap();
                let composed = combined
                    .must(&e, &())
                    .unwrap()
                    .apply(BoolOp4::AND, &guard, &())
                    .unwrap();
                assert_eq!(nested, composed);
                assert_eq!(
                    combined.may(&e, &()).unwrap(),
                    r.may(&q.may(&e, &()).unwrap(), &()).unwrap()
                );
                assert_eq!(
                    combined.all(&e, &()).unwrap(),
                    r.all(&q.all(&e, &()).unwrap(), &()).unwrap()
                );
            }
        }
    }
    let all = relation(&pair, 15);
    let live_only = relation(&pair, 1);
    let combined = plan.compose(&all, &live_only, &()).unwrap();
    assert!(combined.must(&bit_space.full(), &()).unwrap().is_full());
    assert!(
        all.must(&live_only.must(&bit_space.full(), &()).unwrap(), &())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn symbolic_diagonal_and_composition_use_a_compact_sixty_bit_workspace() {
    let unit = Space::new(SpaceId([1; 32]), 0, &()).unwrap();
    let states = Space::new(SpaceId([2; 32]), 20, &()).unwrap();
    let b = base(&states, &unit, &[]);
    let pair = product(3, &b, &b);
    let identity = WorldRelation::identity(&pair, &()).unwrap();
    assert_eq!(identity.region().count(&()).unwrap(), 1 << 20);
    let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
    assert_eq!(plan.workspace().space().dimensions(), 60);
    let result = plan.compose(&identity, &identity, &()).unwrap();
    assert!(result.equivalent(&identity, &()).unwrap());
    let lower = states.coordinate(0, &()).unwrap().complement();
    assert_eq!(result.may(&lower, &()).unwrap(), lower);
}

struct Cancel;
impl Control for Cancel {
    fn checkpoint(&self) -> crate::Result<()> {
        Err(Error::Cancelled)
    }
}

#[test]
fn associativity_reindexes_different_intermediate_product_presentations() {
    let unit = space(1, 0, 1);
    let states = [
        space(2, 1, 3),
        space(3, 2, 13),
        space(4, 1, 3),
        space(5, 2, 11),
    ];
    let bases: Vec<_> = states.iter().map(|state| base(state, &unit, &[])).collect();
    let left = product(6, &bases[0], &bases[1]);
    let middle = product(7, &bases[1], &bases[2]);
    let right = product(8, &bases[2], &bases[3]);
    let first_two = product(9, &bases[0], &bases[2]);
    let last_two = product(10, &bases[1], &bases[3]);
    let result = product(11, &bases[0], &bases[3]);
    let prefix =
        RelationalProduct::new(SpaceId([12; 32]), &left, &middle, &first_two, &()).unwrap();
    let suffix =
        RelationalProduct::new(SpaceId([13; 32]), &middle, &right, &last_two, &()).unwrap();
    let before =
        RelationalProduct::new(SpaceId([14; 32]), &first_two, &right, &result, &()).unwrap();
    let after = RelationalProduct::new(SpaceId([15; 32]), &left, &last_two, &result, &()).unwrap();
    for seed in 0..32u64 {
        let r = relation(&left, seed.wrapping_mul(137) & 255);
        let q = relation(&middle, seed.wrapping_mul(91) & 255);
        let t = relation(&right, seed.wrapping_mul(53) & 255);
        let left_associated = before
            .compose(&prefix.compose(&r, &q, &()).unwrap(), &t, &())
            .unwrap();
        let right_associated = after
            .compose(&r, &suffix.compose(&q, &t, &()).unwrap(), &())
            .unwrap();
        assert!(left_associated.equivalent(&right_associated, &()).unwrap());
    }
}

#[test]
fn relation_descriptors_refuse_foreign_constants_capacity_and_cancellation() {
    let unit = space(1, 0, 1);
    let endpoint = space(2, 1, 3);
    let b = base(&endpoint, &unit, &[]);
    let pair = product(3, &b, &b);
    let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
    let empty = relation(&pair, 0);
    let foreign_endpoint = space(5, 1, 3);
    let foreign_base = base(&foreign_endpoint, &unit, &[]);
    let foreign_pair = product(6, &foreign_base, &foreign_base);
    let foreign = relation(&foreign_pair, 15);
    assert!(matches!(
        plan.compose(&empty, &foreign, &()),
        Err(Error::SpaceMismatch)
    ));
    assert_eq!(
        empty.may(&foreign_endpoint.empty(), &()),
        Err(Error::SpaceMismatch)
    );
    assert!(matches!(
        WorldRelation::new(&pair, &foreign_pair.space().empty(), &()),
        Err(Error::SpaceMismatch)
    ));
    assert!(matches!(
        plan.compose(&empty, &empty, &Cancel),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        plan.left_residual(&empty, &empty, &Cancel),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        plan.right_residual(&empty, &empty, &Cancel),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        FibreProduct::new(SpaceId([7; 32]), &b, &b, &Cancel),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        FaceProduct::new(SpaceId([8; 32]), &[], &()),
        Err(Error::NoFaces)
    ));
    assert!(matches!(
        FibreProduct::with_order(SpaceId([9; 32]), &b, &b, &[0, 0], Limits::default(), &()),
        Err(Error::InvalidOrder)
    ));
    assert!(matches!(
        FibreProduct::with_order(
            SpaceId([12; 32]),
            &b,
            &b,
            &[0, 1],
            Limits {
                records: 1,
                ..Limits::default()
            },
            &(),
        ),
        Err(Error::Capacity(Capacity::Records))
    ));
    assert!(matches!(
        endpoint.restrict(&endpoint.empty(), &()),
        Err(Error::EmptySpace)
    ));
    let one_fibre = endpoint
        .restrict(&endpoint.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let misses_environment = CoordinateMap::coordinates(&one_fibre, &endpoint, &[0], &()).unwrap();
    assert!(matches!(
        misses_environment.certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
    let wide = Space::new(SpaceId([10; 32]), 32, &()).unwrap();
    let wide_base = base(&wide, &unit, &[]);
    assert!(matches!(
        FibreProduct::new(SpaceId([11; 32]), &wide_base, &wide_base, &()),
        Err(Error::Capacity(Capacity::Coordinates))
    ));
    let retained = WorldRelation::identity(&pair, &()).unwrap().converse();
    drop((pair, plan, endpoint, b, empty));
    assert_eq!(retained.domain(&()).unwrap().count(&()).unwrap(), 2);
}

#[test]
fn zero_coordinate_endpoints_still_require_presence_and_validate_contexts() {
    let unit = space(1, 0, 1);
    let base = CoordinateMap::identity(&unit, &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = product(2, &base, &base);
    assert_eq!(pair.space().dimensions(), 0);
    let plan = RelationalProduct::new(SpaceId([3; 32]), &pair, &pair, &pair, &()).unwrap();
    let id = WorldRelation::identity(&pair, &()).unwrap();
    let empty = WorldRelation::new(&pair, &pair.space().empty(), &()).unwrap();
    assert!(
        id.readout(&())
            .unwrap()
            .equivalent(base.map(), &())
            .unwrap()
    );
    assert!(plan.compose(&id, &id, &()).unwrap().region().is_full());
    assert!(plan.compose(&id, &empty, &()).unwrap().region().is_empty());
    assert!(empty.all(&unit.empty(), &()).unwrap().is_full());
    assert!(empty.must(&unit.full(), &()).unwrap().is_empty());
    assert!(matches!(empty.readout(&()), Err(Error::PartialRelation)));
    let foreign = space(4, 0, 1);
    assert_eq!(empty.may(&foreign.empty(), &()), Err(Error::SpaceMismatch));
}
