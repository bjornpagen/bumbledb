use std::cell::Cell;

use crate::{
    Capacity, Control, CoordinateMap, Error, Event, EventPartition, PartitionLimits, Result, Space,
    SpaceId,
};

fn space() -> Space {
    Space::new(SpaceId([1; 32]), 2, &()).unwrap()
}
fn table(space: &Space, mask: u64) -> Event {
    space.table(3, &[mask], &()).unwrap()
}
fn partition(parent: &Event, cells: &[Event]) -> EventPartition {
    EventPartition::on(parent, cells, PartitionLimits::default(), &()).unwrap()
}
fn mask(event: &Event, support: u64) -> u64 {
    (0..4)
        .filter(|&w| support & (1 << w) != 0 && event.contains(w).unwrap())
        .fold(0, |bits, w| bits | (1 << w))
}

#[test]
fn every_two_cell_admission_matches_pointwise_coverage_and_functionality() {
    let raw = space();
    for support in [15, 7, 9] {
        let space = raw.restrict(&table(&raw, support), &()).unwrap();
        for parent in 0..16 {
            for a in 0..16 {
                for b in 0..16 {
                    let p = parent & support;
                    let actual = EventPartition::on(
                        &table(&space, parent),
                        &[table(&space, a), table(&space, b)],
                        PartitionLimits::default(),
                        &(),
                    );
                    if a & b & p != 0 {
                        assert_eq!(actual.unwrap_err(), Error::PartitionOverlap);
                    } else if (a | b) & p != p {
                        assert_eq!(actual.unwrap_err(), Error::PartitionGap);
                    } else {
                        let actual = actual.unwrap();
                        assert_eq!(actual.cells().len(), 2);
                        assert_eq!(mask(&actual.cells()[0], support), a & p);
                        assert_eq!(mask(&actual.cells()[1], support), b & p);
                        for w in 0..4 {
                            if support & (1 << w) != 0 {
                                let expected = if p & (1 << w) == 0 {
                                    None
                                } else {
                                    Some(usize::from(a & (1 << w) == 0))
                                };
                                assert_eq!(actual.locate(w, &()).unwrap(), expected);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn cardinality_rosters_match_direct_per_world_counts_and_keep_duplicate_positions() {
    let raw = space();
    for support in [15, 7, 9] {
        let space = raw.restrict(&table(&raw, support), &()).unwrap();
        for encoding in 0..4096 {
            let values = [encoding & 15, (encoding >> 4) & 15, (encoding >> 8) & 15];
            let events = values.map(|bits| table(&space, bits));
            let counts = space
                .count_partition(&events, PartitionLimits::default(), &())
                .unwrap();
            assert_eq!(counts.cells().len(), 4);
            for k in 0..4 {
                let expected = (0..4)
                    .filter(|&w| {
                        support & (1 << w) != 0
                            && values.iter().filter(|&&e| e & (1 << w) != 0).count() == k
                    })
                    .fold(0, |bits, w| bits | (1 << w));
                assert_eq!(mask(&counts.cells()[k], support), expected);
            }
        }
        let empty = space
            .count_partition(&[], PartitionLimits::default(), &())
            .unwrap();
        assert_eq!(empty.cells(), &[space.full()]);
    }
}

#[test]
fn coarsening_is_ordinary_value_mapping_with_explicit_empty_output_buckets() {
    let space = space();
    for a in 0..16 {
        let source = partition(
            &space.full(),
            &[
                table(&space, a),
                table(&space, a).complement(),
                space.empty(),
            ],
        );
        for encoding in 0..27 {
            let destinations = [encoding % 3, (encoding / 3) % 3, encoding / 9];
            let grouped = source
                .coarsen(&destinations, 4, PartitionLimits::default(), &())
                .unwrap();
            assert_eq!(grouped.cells().len(), 4);
            for output in 0..4 {
                let expected = (0..4)
                    .filter(|w| destinations[usize::from(a & (1 << w) == 0)] == output)
                    .fold(0, |bits, w| bits | (1 << w));
                assert_eq!(mask(&grouped.cells()[output], 15), expected);
            }
            assert!(grouped.select(&[0, 1, 2, 3], &()).unwrap().is_full());
            assert!(grouped.select(&[], &()).unwrap().is_empty());
            assert_eq!(grouped.select(&[0, 0], &()).unwrap(), grouped.cells()[0]);
        }
    }
}

fn three_labels(space: &Space, mut encoding: u64) -> ([u64; 4], EventPartition) {
    let mut labels = [0; 4];
    let mut masks = [0; 3];
    for (world, label) in labels.iter_mut().enumerate() {
        *label = encoding % 3;
        masks[usize::try_from(*label).unwrap()] |= 1 << world;
        encoding /= 3;
    }
    (
        labels,
        partition(&space.full(), &masks.map(|m| table(space, m))),
    )
}

#[test]
fn refinement_preserves_shared_world_correlations_and_ordered_pairs() {
    let space = space();
    for a in 0..16 {
        let left = partition(
            &space.full(),
            &[table(&space, a), table(&space, a).complement()],
        );
        for encoding in 0..81 {
            let (labels, right) = three_labels(&space, encoding);
            let joint = left
                .refine(&right, PartitionLimits::default(), &())
                .unwrap();
            assert_eq!(joint.cells().len(), 6);
            for (world, &label) in labels.iter().enumerate() {
                let expected =
                    usize::from(a & (1 << world) == 0) * 3 + usize::try_from(label).unwrap();
                assert_eq!(joint.locate(world as u64, &()).unwrap(), Some(expected));
                for (index, cell) in joint.cells().iter().enumerate() {
                    assert_eq!(cell.contains(world as u64).unwrap(), index == expected);
                }
            }
        }
    }
    let left = partition(&table(&space, 3), &[table(&space, 1), table(&space, 2)]);
    let right = partition(&table(&space, 6), &[table(&space, 4), table(&space, 2)]);
    let joint = left
        .refine(&right, PartitionLimits::default(), &())
        .unwrap();
    assert_eq!(mask(joint.parent(), 15), 2);
    assert_eq!(joint.locate(1, &()).unwrap(), Some(3));
    assert_eq!(joint.locate(0, &()).unwrap(), None);
}

#[test]
fn readouts_and_pullbacks_preserve_partition_laws_but_images_need_not() {
    let space = space();
    let source = partition(
        &space.full(),
        &[
            table(&space, 3),
            table(&space, 4),
            table(&space, 8),
            space.empty(),
        ],
    );
    let target_raw = Space::new(SpaceId([2; 32]), 3, &()).unwrap();
    let target = target_raw
        .restrict(&target_raw.table(7, &[0b10_0101], &()).unwrap(), &())
        .unwrap(); // legal codes 0,2,5
    let readout = source.readout(&target, &[0, 2, 5, 0], &()).unwrap();
    for (world, code) in [0, 0, 2, 5].into_iter().enumerate() {
        for (bit, event) in readout.readouts().iter().enumerate() {
            assert_eq!(
                event.contains(world as u64).unwrap(),
                code & (1 << bit) != 0
            );
        }
    }
    let on_target = partition(
        &target.full(),
        &[
            target.table(7, &[0b10_0001], &()).unwrap(),
            target.table(7, &[0b00_0100], &()).unwrap(),
        ],
    );
    let pulled = on_target.pullback(&readout, &()).unwrap();
    assert_eq!(mask(&pulled.cells()[0], 15), 11);
    assert_eq!(mask(&pulled.cells()[1], 15), 4);
    // Collapse two distinct cells into the same target world. Image overlaps.
    let unit = Space::new(SpaceId([3; 32]), 0, &()).unwrap();
    let collapse = CoordinateMap::new(&space, &unit, &[], &()).unwrap();
    let images = source.cells()[..2]
        .iter()
        .map(|cell| collapse.image(cell, &()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        EventPartition::on(&unit.full(), &images, PartitionLimits::default(), &()).unwrap_err(),
        Error::PartitionOverlap
    );
    assert_eq!(
        source.readout(&target, &[0, 2, 5, 7], &()).unwrap_err(),
        Error::IllegalWorld(7)
    );
    let partial = partition(&table(&space, 3), &[table(&space, 3)]);
    assert_eq!(
        partial.readout(&unit, &[0], &()).unwrap_err(),
        Error::PartitionGap
    );
}

struct StopAfter(Cell<usize>);
impl Control for StopAfter {
    fn checkpoint(&self) -> Result<()> {
        let remaining = self.0.get();
        if remaining == 0 {
            Err(Error::Cancelled)
        } else {
            self.0.set(remaining - 1);
            Ok(())
        }
    }
}

#[test]
fn partition_validation_retains_empty_owners_indices_and_resource_refusals() {
    let space = space();
    let other = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    let limits = PartitionLimits::default();
    assert_eq!(
        EventPartition::on(&space.empty(), &[other.empty()], limits, &()).unwrap_err(),
        Error::SpaceMismatch
    );
    let empty = partition(&space.empty(), &[]);
    let wrong = partition(&other.empty(), &[]);
    assert_eq!(
        empty.refine(&wrong, limits, &()).unwrap_err(),
        Error::SpaceMismatch
    );
    let buckets = empty.coarsen(&[], 3, limits, &()).unwrap();
    assert_eq!(buckets.cells().len(), 3);
    assert!(buckets.cells().iter().all(Event::is_empty));
    assert_eq!(buckets.select(&[3], &()), Err(Error::PartitionIndex));
    assert_eq!(
        buckets.coarsen(&[0], 1, limits, &()).unwrap_err(),
        Error::PartitionArity
    );
    assert_eq!(
        buckets.coarsen(&[0, 0, 1], 1, limits, &()).unwrap_err(),
        Error::PartitionIndex
    );
    assert_eq!(
        buckets.readout(&space, &[0], &()).unwrap_err(),
        Error::PartitionArity
    );
    assert_eq!(
        buckets
            .refine(&buckets, PartitionLimits { cells: 8 }, &())
            .unwrap_err(),
        Error::Capacity(Capacity::PartitionCells)
    );
    assert_eq!(
        space
            .count_partition(&[], PartitionLimits { cells: 0 }, &())
            .unwrap_err(),
        Error::Capacity(Capacity::PartitionCells)
    );
    assert_eq!(
        empty
            .coarsen(&[], 2, PartitionLimits { cells: 1 }, &())
            .unwrap_err(),
        Error::Capacity(Capacity::PartitionCells)
    );
    assert_eq!(
        buckets.select(&[], &StopAfter(Cell::new(0))),
        Err(Error::Cancelled)
    );
    assert_eq!(
        space
            .count_partition(&[space.full(); 1], limits, &StopAfter(Cell::new(3)))
            .unwrap_err(),
        Error::Cancelled
    );
    let independent = Event::from_bytes(&space.full().to_bytes(&()).unwrap(), &()).unwrap();
    let retained = partition(&space.full(), &[independent]);
    drop(space);
    assert_eq!(retained.locate(2, &()).unwrap(), Some(0));
    assert_eq!(retained.locate(4, &()), Err(Error::IllegalWorld(4)));
}

#[test]
fn symbolic_counting_and_partitions_do_not_enumerate_worlds() {
    let space = Space::new(SpaceId([1; 32]), 62, &()).unwrap();
    let event = space.coordinate(61, &()).unwrap();
    let inputs = vec![event.clone(); 12];
    let counts = space
        .count_partition(&inputs, PartitionLimits::default(), &())
        .unwrap();
    assert_eq!(counts.cells().len(), 13);
    assert_eq!(counts.cells()[0], event.complement());
    assert_eq!(counts.cells()[12], event);
    assert!(counts.cells()[1..12].iter().all(Event::is_empty));
    assert_eq!(counts.cells()[12].count(&()).unwrap(), 1 << 61);
    let destination = Space::new(SpaceId([2; 32]), 4, &()).unwrap();
    let map = counts
        .readout(&destination, &(0..13).collect::<Vec<_>>(), &())
        .unwrap();
    assert_eq!(map.readouts()[2], event);
    assert_eq!(map.readouts()[3], event);
    assert!(map.readouts()[0].is_empty());
    assert!(map.readouts()[1].is_empty());
}
