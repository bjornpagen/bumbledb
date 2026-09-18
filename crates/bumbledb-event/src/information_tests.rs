use crate::{
    BoolOp4, Control, CoordinateMap, Error, Event, FibreProduct, RelationalProduct, Result, Space,
    SpaceId, WorldRelation,
};

fn supported(support: u64, id: u8) -> Space {
    let raw = Space::new(SpaceId([id; 32]), 2, &()).unwrap();
    raw.restrict(&raw.table(3, &[support], &()).unwrap(), &())
        .unwrap()
}

fn event(space: &Space, bits: u64) -> Event {
    space.table(3, &[bits], &()).unwrap()
}

fn map(source: &Space, target: &Space, values: &[u64; 4]) -> CoordinateMap {
    let readouts = (0..target.dimensions())
        .map(|bit| {
            event(
                source,
                values.iter().enumerate().fold(0, |mask, (world, value)| {
                    mask | ((value >> bit & 1) << world)
                }),
            )
        })
        .collect::<Vec<_>>();
    CoordinateMap::new(source, target, &readouts, &()).unwrap()
}

fn bits(value: &Event, support: u64) -> u64 {
    (0..4)
        .filter(|world| support >> world & 1 != 0 && value.contains(*world).unwrap())
        .fold(0, |mask, world| mask | (1 << world))
}

fn possible(values: &[u64; 4], support: u64, selected: u64) -> u64 {
    (0..4)
        .filter(|&w| {
            support >> w & 1 != 0
                && (0..4).any(|v| support & selected & (1 << v) != 0 && values[v] == values[w])
        })
        .fold(0, |mask, w| mask | (1 << w))
}

fn determines(fine: &[u64; 4], coarse: &[u64; 4], support: u64) -> bool {
    (0..4).all(|v| {
        (0..4).all(|w| {
            support >> v & support >> w & 1 == 0 || fine[v] != fine[w] || coarse[v] == coarse[w]
        })
    })
}

#[test]
fn every_four_code_readout_matches_an_explicit_information_oracle() {
    let target = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    for support in [1, 6, 9, 11, 15] {
        let source = supported(support, 1);
        for code in 0..256 {
            let values = std::array::from_fn(|i| (code >> (2 * i)) & 3);
            let observation = map(&source, &target, &values);
            for mask in 0..16 {
                let input = event(&source, mask);
                let may = observation.possible(&input, &()).unwrap();
                let must = observation.guaranteed(&input, &()).unwrap();
                let ambiguous = observation.ambiguous(&input, &()).unwrap();
                let expected = possible(&values, support, mask);
                let counter = possible(&values, support, !mask);
                assert_eq!(bits(&may, support), expected);
                assert_eq!(bits(&must, support), support & !counter);
                assert_eq!(bits(&ambiguous, support), expected & counter);
                assert_eq!(observation.possible(&may, &()).unwrap(), may);
                assert_eq!(observation.guaranteed(&must, &()).unwrap(), must);
                assert!(must.signature(&input, &()).unwrap().included());
                assert!(input.signature(&may, &()).unwrap().included());
            }
        }
    }
}

const READOUTS: [[u64; 4]; 6] = [
    [0, 0, 0, 0],
    [0, 1, 2, 3],
    [0, 1, 0, 1],
    [0, 0, 1, 1],
    [0, 1, 1, 0],
    [0, 0, 0, 3],
];

#[test]
fn evidence_classifies_whole_cells_and_never_invents_vacuous_certainty() {
    let target = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    for support in 1..16 {
        let source = supported(support, 1);
        for values in READOUTS {
            let observation = map(&source, &target, &values);
            for mask in 0..16 {
                for given in 0..16 {
                    let cases = observation
                        .information(&event(&source, mask), &event(&source, given), &())
                        .unwrap();
                    let yes = possible(&values, support, mask & given);
                    let no = possible(&values, support, !mask & given);
                    assert_eq!(
                        bits(cases.reachable(), support),
                        possible(&values, support, given)
                    );
                    assert_eq!(bits(cases.possible(), support), yes);
                    assert_eq!(bits(cases.guaranteed(), support), yes & !no);
                    assert_eq!(bits(cases.ruled_out(), support), no & !yes);
                    assert_eq!(bits(cases.ambiguous(), support), yes & no);
                    let parts = [cases.guaranteed(), cases.ruled_out(), cases.ambiguous()];
                    let mut union = source.empty();
                    for part in parts {
                        assert!(union.signature(part, &()).unwrap().disjoint());
                        union = union.apply(BoolOp4::OR, part, &()).unwrap();
                    }
                    assert_eq!(union, *cases.reachable());
                }
            }
        }
    }
}

#[test]
fn readout_factorization_is_exactly_the_observation_fd() {
    let coarse_target = Space::new(SpaceId([3; 32]), 2, &()).unwrap();
    for support in 1..16 {
        let source = supported(support, 1);
        for fine in READOUTS {
            let image = (0..4)
                .filter(|w| support >> w & 1 != 0)
                .fold(0, |mask, w| mask | (1 << fine[w]));
            let fine_target = supported(image, 2);
            let fine_map = map(&source, &fine_target, &fine)
                .certify_surjective(&())
                .unwrap();
            for coarse in READOUTS {
                let coarse_map = map(&source, &coarse_target, &coarse);
                let factor = fine_map.factor(&coarse_map, &());
                assert_eq!(factor.is_ok(), determines(&fine, &coarse, support));
                if let Ok(factor) = factor {
                    assert!(
                        fine_map
                            .map()
                            .then(&factor, &())
                            .unwrap()
                            .equivalent(&coarse_map, &())
                            .unwrap()
                    );
                    for w in 0..4 {
                        if support >> w & 1 != 0 {
                            assert_eq!(factor.map_world(fine[w]).unwrap(), coarse[w]);
                        }
                    }
                } else {
                    assert_eq!(factor.unwrap_err(), Error::RoleMismatch);
                }
            }
        }
    }
}

#[test]
fn indistinguishability_is_the_same_algebra_as_relational_modalities() {
    let environment = Space::new(SpaceId([3; 32]), 0, &()).unwrap();
    let target = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    for support in [6, 11, 15] {
        let source = supported(support, 1);
        let base = CoordinateMap::new(&source, &environment, &[], &())
            .unwrap()
            .certify_surjective(&())
            .unwrap();
        let pair = FibreProduct::new(SpaceId([4; 32]), &base, &base, &()).unwrap();
        let plan = RelationalProduct::new(SpaceId([5; 32]), &pair, &pair, &pair, &()).unwrap();
        for values in READOUTS {
            let observation = map(&source, &target, &values);
            let kernel = WorldRelation::indistinguishable(&pair, &observation, &()).unwrap();
            assert!(kernel.domain(&()).unwrap().is_full());
            assert!(kernel.equivalent(&kernel.converse(), &()).unwrap());
            assert!(
                plan.compose(&kernel, &kernel, &())
                    .unwrap()
                    .equivalent(&kernel, &())
                    .unwrap()
            );
            assert!(
                WorldRelation::identity(&pair, &())
                    .unwrap()
                    .included_in(&kernel, &())
                    .unwrap()
            );
            for w in 0..4 {
                for v in 0..4 {
                    if support >> w & support >> v & 1 != 0 {
                        assert_eq!(
                            kernel.region().contains(w | (v << 2)).unwrap(),
                            values[usize::try_from(w).unwrap()]
                                == values[usize::try_from(v).unwrap()]
                        );
                    }
                }
            }
            for mask in 0..16 {
                let input = event(&source, mask);
                assert_eq!(
                    kernel.may(&input, &()).unwrap(),
                    observation.possible(&input, &()).unwrap()
                );
                assert_eq!(
                    kernel.all(&input, &()).unwrap(),
                    observation.guaranteed(&input, &()).unwrap()
                );
                assert_eq!(
                    kernel.must(&input, &()).unwrap(),
                    observation.guaranteed(&input, &()).unwrap()
                );
                for given in [0, 1, 5, 15] {
                    let evidence = event(&source, given);
                    let selection = pair.right().map().pullback(&evidence, &()).unwrap();
                    let restricted = WorldRelation::new(
                        &pair,
                        &kernel
                            .region()
                            .apply(BoolOp4::AND, &selection, &())
                            .unwrap(),
                        &(),
                    )
                    .unwrap();
                    let cases = observation.information(&input, &evidence, &()).unwrap();
                    assert_eq!(restricted.may(&input, &()).unwrap(), *cases.possible());
                    assert_eq!(restricted.must(&input, &()).unwrap(), *cases.guaranteed());
                    assert_eq!(restricted.domain(&()).unwrap(), *cases.reachable());
                }
            }
        }
    }
}

#[test]
fn environment_equality_cannot_silently_refine_an_observation() {
    let source = supported(15, 1);
    let environment = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let base = CoordinateMap::coordinates(&source, &environment, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = FibreProduct::new(SpaceId([3; 32]), &base, &base, &()).unwrap();
    let hidden = CoordinateMap::coordinates(&source, &environment, &[1], &()).unwrap();
    assert_eq!(
        WorldRelation::indistinguishable(&pair, &hidden, &()).unwrap_err(),
        Error::EnvironmentMismatch
    );
    let unit = Space::new(SpaceId([4; 32]), 0, &()).unwrap();
    let constant = CoordinateMap::new(&source, &unit, &[], &()).unwrap();
    assert_eq!(
        WorldRelation::indistinguishable(&pair, &constant, &()).unwrap_err(),
        Error::EnvironmentMismatch
    );
    let identity = CoordinateMap::identity(&source, &()).unwrap();
    assert!(
        WorldRelation::indistinguishable(&pair, &identity, &())
            .unwrap()
            .equivalent(&WorldRelation::identity(&pair, &()).unwrap(), &())
            .unwrap()
    );
    let independent = Event::from_bytes(&source.full().to_bytes(&()).unwrap(), &())
        .unwrap()
        .space();
    let same = CoordinateMap::coordinates(&independent, &environment, &[0], &()).unwrap();
    assert!(
        WorldRelation::indistinguishable(&pair, &same, &())
            .unwrap()
            .region()
            .is_full()
    );
}

#[test]
fn incomparable_observations_do_not_commute_on_coupled_support() {
    let source = supported(0b1011, 1);
    let target = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let first = CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap();
    let second = CoordinateMap::coordinates(&source, &target, &[1], &()).unwrap();
    let start = event(&source, 1);
    let first_then_second = second
        .possible(&first.possible(&start, &()).unwrap(), &())
        .unwrap();
    let second_then_first = first
        .possible(&second.possible(&start, &()).unwrap(), &())
        .unwrap();
    assert_eq!(bits(&first_then_second, 11), 3);
    assert!(second_then_first.is_full());
}

#[test]
fn information_uses_symbolic_maps_without_requiring_a_doubled_workspace() {
    let source = Space::new(SpaceId([1; 32]), 62, &()).unwrap();
    let target = Space::new(SpaceId([2; 32]), 61, &()).unwrap();
    let observation =
        CoordinateMap::coordinates(&source, &target, &(1..62).collect::<Vec<_>>(), &())
            .unwrap()
            .certify_surjective(&())
            .unwrap();
    let unknown = source.coordinate(0, &()).unwrap();
    assert!(observation.map().possible(&unknown, &()).unwrap().is_full());
    assert!(
        observation
            .map()
            .guaranteed(&unknown, &())
            .unwrap()
            .is_empty()
    );
    let certain = source.coordinate(61, &()).unwrap();
    assert_eq!(
        observation.map().guaranteed(&certain, &()).unwrap(),
        certain
    );
    let one = Space::new(SpaceId([3; 32]), 1, &()).unwrap();
    let readout = CoordinateMap::coordinates(&source, &one, &[61], &()).unwrap();
    let factor = observation.factor(&readout, &()).unwrap();
    assert_eq!(factor.map_world(1 << 60).unwrap(), 1);
}

struct Cancel;
impl Control for Cancel {
    fn checkpoint(&self) -> Result<()> {
        Err(Error::Cancelled)
    }
}

#[test]
fn information_validates_all_contexts_and_retains_owned_results() {
    let retained = {
        let source = supported(15, 1);
        let target = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
        let observation = CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap();
        let foreign = supported(15, 3);
        for value in [foreign.empty(), foreign.full()] {
            assert_eq!(observation.possible(&value, &()), Err(Error::SpaceMismatch));
            assert_eq!(
                observation.guaranteed(&value, &()),
                Err(Error::SpaceMismatch)
            );
            assert_eq!(
                observation
                    .information(&value, &source.empty(), &())
                    .unwrap_err(),
                Error::SpaceMismatch
            );
            assert_eq!(
                observation
                    .information(&source.empty(), &value, &())
                    .unwrap_err(),
                Error::SpaceMismatch
            );
        }
        assert_eq!(
            observation
                .information(&source.empty(), &source.empty(), &Cancel)
                .unwrap_err(),
            Error::Cancelled
        );
        let unit = Space::new(SpaceId([4; 32]), 0, &()).unwrap();
        let foreign_constant = CoordinateMap::new(&foreign, &unit, &[], &()).unwrap();
        assert_eq!(
            observation
                .certify_surjective(&())
                .unwrap()
                .factor(&foreign_constant, &())
                .unwrap_err(),
            Error::SpaceMismatch
        );
        observation
            .information(&source.coordinate(1, &()).unwrap(), &source.full(), &())
            .unwrap()
    };
    assert!(retained.reachable().is_full());
    assert!(retained.ambiguous().is_full());
    assert!(retained.guaranteed().is_empty());
    assert!(retained.ruled_out().is_empty());
    assert_eq!(retained.ambiguous().count(&()).unwrap(), 4);
}
