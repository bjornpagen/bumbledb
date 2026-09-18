use std::cell::Cell;
use std::collections::HashSet;

use crate::arena::{Arena, Operation};
use crate::{
    BoolOp4, Capacity, Control, Error, Event, EventKey, Limits, Registry, Result, Space, SpaceId,
};

const NAME: SpaceId = SpaceId([1; 32]);

fn members(event: &Event, support: u64, worlds: u64) -> u64 {
    (0..worlds)
        .filter(|&w| support & (1 << w) != 0 && event.contains(w).unwrap())
        .fold(0, |mask, w| mask | (1 << w))
}

#[test]
fn all_truth_functions_on_every_four_world_support() {
    for support in 1u64..16 {
        let raw = Space::new(NAME, 2, &()).unwrap();
        let legal = raw.table(3, &[support], &()).unwrap();
        let space = raw.restrict(&legal, &()).unwrap();
        let events: Vec<_> = (0u64..16)
            .map(|m| space.table(3, &[m], &()).unwrap())
            .collect();
        for (a, left) in events.iter().enumerate() {
            let a = a as u64;
            assert_eq!(members(left, support, 4), a & support);
            assert_eq!(
                left.count(&()).unwrap(),
                u64::from((a & support).count_ones())
            );
            assert_eq!(left.is_empty(), a & support == 0);
            assert_eq!(left.is_full(), a & support == support);
            assert_eq!(&!&!left, left);
            assert_eq!(*left, events[usize::try_from(a & support).unwrap()]);
            let witness = left.witness(&()).unwrap();
            assert_eq!(witness.is_some(), a & support != 0);
            if let Some(w) = witness {
                assert_ne!(a & support & (1 << w), 0);
            }
            for (b, right) in events.iter().enumerate() {
                let b = b as u64;
                let signature = left.signature(right, &()).unwrap();
                assert_eq!(signature.included(), a & support & !b == 0);
                assert_eq!(signature.equal(), a & support == b & support);
                assert_eq!(signature.disjoint(), a & b & support == 0);
                for code in 0..16 {
                    let operation = BoolOp4::new(code).unwrap();
                    let expected = (0..4)
                        .filter(|&w| {
                            support >> w & 1 != 0
                                && operation.evaluate(a >> w & 1 != 0, b >> w & 1 != 0)
                        })
                        .fold(0, |m, w| m | (1 << w));
                    let actual = left.apply(operation, right, &()).unwrap();
                    assert_eq!(members(&actual, support, 4), expected);
                    assert_eq!(actual, events[usize::try_from(expected).unwrap()]);
                    assert_eq!(signature.possible(operation), expected != 0);
                    assert_eq!(signature.full(operation), expected == support);
                }
            }
        }
    }
}

#[test]
fn direct_ite_on_every_three_argument_four_world_function() {
    let space = Space::new(NAME, 2, &()).unwrap();
    let events: Vec<_> = (0u64..16)
        .map(|m| space.table(3, &[m], &()).unwrap())
        .collect();
    for c in 0..16 {
        for h in 0..16 {
            for l in 0..16 {
                let result = events[c].ite(&events[h], &events[l], &()).unwrap();
                assert_eq!(result, events[(c & h) | ((!c & 15) & l)]);
            }
        }
    }
}

#[test]
fn symbolic_splits_and_essential_reduction_across_table_boundary() {
    for order in [(0..12).collect::<Vec<_>>(), (0..12).rev().collect()] {
        let space = Space::with_order(NAME, &order, Limits::default(), &()).unwrap();
        let variables: Vec<_> = (0..12).map(|v| space.coordinate(v, &()).unwrap()).collect();
        let mut parity = space.empty();
        for variable in &variables {
            parity = parity.apply(BoolOp4::XOR, variable, &()).unwrap();
        }
        let mut reverse = space.empty();
        for variable in variables.iter().rev() {
            reverse = reverse.apply(BoolOp4::XOR, variable, &()).unwrap();
        }
        assert_eq!(parity, reverse);
        assert_eq!(parity.count(&()).unwrap(), 2048);
        let mut table = vec![0; 64];
        for world in 0u64..4096 {
            let expected = world.count_ones() % 2 != 0;
            assert_eq!(parity.contains(world).unwrap(), expected);
            if expected {
                table[usize::try_from(world / 64).unwrap()] |= 1 << (world % 64);
            }
        }
        assert_eq!(parity, space.table(4095, &table, &()).unwrap());
        let recovered = variables[0].ite(&parity, &parity, &()).unwrap();
        assert_eq!(recovered, parity);
        assert!(parity.saturate(1, &()).unwrap().is_full());
        let mut cancelled = parity;
        for variable in variables.iter().skip(3) {
            cancelled = cancelled.apply(BoolOp4::XOR, variable, &()).unwrap();
        }
        assert_eq!(cancelled, space.table(7, &[0b1001_0110], &()).unwrap());
    }
    let wide = Space::new(NAME, 62, &()).unwrap();
    let first = wide.coordinate(0, &()).unwrap();
    let last = wide.coordinate(61, &()).unwrap();
    let pair = first.apply(BoolOp4::AND, &last, &()).unwrap();
    assert_eq!(pair.count(&()).unwrap(), 1 << 60);
    assert_eq!(wide.full().count(&()).unwrap(), 1 << 62);
    assert!(pair.contains((1 << 61) | 1).unwrap());
}

#[test]
fn projection_uses_legal_fibres_and_counts_do_not_count_aliases() {
    let full = Space::new(NAME, 3, &()).unwrap();
    for support in 1u64..256 {
        let space = full
            .restrict(&full.table(7, &[support], &()).unwrap(), &())
            .unwrap();
        for input in [0, 1, 0b0011_1001, 0b1010_0101, 255] {
            let event = space.table(7, &[input], &()).unwrap();
            for hidden in 0..8 {
                let projected = event.saturate(hidden, &()).unwrap();
                let expected = (0..8)
                    .filter(|&w| {
                        support >> w & 1 != 0
                            && (0..8)
                                .any(|t| support & input & (1 << t) != 0 && (w ^ t) & !hidden == 0)
                    })
                    .fold(0, |m, w| m | (1 << w));
                assert_eq!(members(&projected, support, 8), expected);
                assert_eq!(
                    projected.count(&()).unwrap(),
                    u64::from(expected.count_ones())
                );
            }
        }
        for w in 0..8 {
            if support & (1 << w) == 0 {
                assert_eq!(space.full().contains(w), Err(Error::IllegalWorld(w)));
            }
        }
    }
}

#[test]
fn owners_are_checked_before_constants_and_results_keep_them_alive() {
    let a = Space::new(NAME, 2, &()).unwrap();
    let b = Space::new(NAME, 2, &()).unwrap();
    assert_eq!(
        a.empty().apply(BoolOp4::AND, &b.full(), &()),
        Err(Error::SpaceMismatch)
    );
    assert_eq!(
        a.full().ite(&a.full(), &b.empty(), &()),
        Err(Error::SpaceMismatch)
    );
    assert_eq!(a.empty().equivalent(&b.empty()), Err(Error::SpaceMismatch));
    assert_ne!(a.empty().key(), b.empty().key());
    let event = a.coordinate(1, &()).unwrap();
    let copied = event.clone();
    assert_eq!(event, copied);
    let subset = a.restrict(&event, &()).unwrap();
    assert!(event.in_space(&subset, &()).unwrap().is_full());
    assert_eq!(subset.full().in_space(&a, &()), Err(Error::SpaceMismatch));
    drop(a);
    drop(b);
    drop(subset);
    drop(copied);
    assert_eq!(event.count(&()).unwrap(), 2);
    assert_eq!(size_of::<EventKey>(), 16);
    #[expect(
        clippy::mutable_key_type,
        reason = "arena mutation never changes an Event's immutable key"
    )]
    let retained: HashSet<_> = [event.clone(), event, copied_event()].into_iter().collect();
    assert_eq!(retained.len(), 2);
}

fn copied_event() -> Event {
    Space::new(SpaceId([2; 32]), 2, &()).unwrap().full()
}

struct CancelAfter(Cell<usize>);
impl Control for CancelAfter {
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
fn cancellation_capacity_and_constructor_rejections_are_explicit() {
    assert!(matches!(
        Space::new(NAME, 63, &()),
        Err(Error::Capacity(Capacity::Coordinates))
    ));
    assert!(matches!(
        Space::with_order(NAME, &[1, 1], Limits::default(), &()),
        Err(Error::InvalidOrder)
    ));
    let zero = Space::new(NAME, 0, &()).unwrap();
    assert_eq!(zero.full().count(&()).unwrap(), 1);
    assert_eq!(zero.coordinate(0, &()), Err(Error::InvalidCoordinate(0)));
    assert!(matches!(
        zero.restrict(&zero.empty(), &()),
        Err(Error::EmptySpace)
    ));
    assert_eq!(zero.table(0, &[], &()), Err(Error::InvalidTable));
    assert_eq!(zero.table(0, &[2], &()), Err(Error::InvalidTable));
    let limits = Limits {
        records: 2,
        ..Limits::default()
    };
    let small = Space::with_order(NAME, &[0, 1], limits, &()).unwrap();
    let x = small.coordinate(0, &()).unwrap();
    assert_eq!(
        small.coordinate(1, &()),
        Err(Error::Capacity(Capacity::Records))
    );
    assert_eq!(small.coordinate(0, &()).unwrap(), x);
    assert_eq!(x.count(&()).unwrap(), 2);
    let space = Space::new(NAME, 12, &()).unwrap();
    let cancelled = CancelAfter(Cell::new(0));
    assert_eq!(
        space
            .full()
            .apply(BoolOp4::TRUE, &space.empty(), &cancelled),
        Err(Error::Cancelled)
    );
    let interrupted = CancelAfter(Cell::new(20));
    assert_eq!(
        space.table(4095, &[0xa55a_a55a_a55a_a55a; 64], &interrupted),
        Err(Error::Cancelled)
    );
    // Refusal publishes no handle, poisons no arena, and leaves old values valid.
    assert_eq!(space.full().count(&()).unwrap(), 4096);
    assert!(space.coordinate(0, &()).is_ok());
}

#[test]
fn all_interner_fingerprints_can_collide_without_changing_identity() {
    let mut arena = Arena::new(&[0, 1, 2], Limits::default()).unwrap();
    arena.collide_all();
    let mut op = Operation::new(&mut arena, &()).unwrap();
    let roots: Vec<_> = (0..256)
        .map(|bits| op.import_table(7, &[bits]).unwrap())
        .collect();
    assert_eq!(roots.iter().collect::<HashSet<_>>().len(), 256);
    for (bits, root) in roots.iter().enumerate() {
        assert_eq!(op.import_table(7, &[bits as u64]).unwrap(), *root);
        for world in 0..8 {
            assert_eq!(op.arena.evaluate(*root, world), bits >> world & 1 != 0);
        }
    }
}

#[test]
fn concurrent_interning_keeps_one_canonical_identity() {
    let space = Space::new(NAME, 12, &()).unwrap();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let space = space.clone();
            std::thread::spawn(move || {
                let mut value = space.empty();
                for v in 0..12 {
                    value = value
                        .apply(BoolOp4::XOR, &space.coordinate(v, &()).unwrap(), &())
                        .unwrap();
                }
                value
            })
        })
        .collect();
    let values: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(values.windows(2).all(|pair| pair[0] == pair[1]));
    drop(space);
    assert_eq!(values[0].count(&()).unwrap(), 2048);
}

#[test]
fn cardinality_constructs_regions_including_duplicate_and_empty_inputs() {
    let space = Space::new(NAME, 12, &()).unwrap();
    let mut events: Vec<_> = (0..12).map(|v| space.coordinate(v, &()).unwrap()).collect();
    events.push(events[0].clone());
    events.push(space.empty());
    events.push(space.full());
    for (low, high) in [(0, 0), (1, 1), (4, 9), (14, 20), (15, 14), (0, usize::MAX)] {
        let value = space.cardinality(&events, low, high, &()).unwrap();
        let mut expected_count = 0;
        for world in 0u64..4096 {
            let count = world.count_ones() as usize + usize::from(world & 1 != 0) + 1;
            let expected = low <= count && count <= high;
            assert_eq!(value.contains(world).unwrap(), expected);
            expected_count += u64::from(expected);
        }
        assert_eq!(value.count(&()).unwrap(), expected_count);
    }
    assert!(space.cardinality(&[], 0, 0, &()).unwrap().is_full());
    assert!(space.cardinality(&[], 1, 1, &()).unwrap().is_empty());
    let alien = Space::new(SpaceId([3; 32]), 1, &()).unwrap().empty();
    assert_eq!(
        space.cardinality(&[alien], 100, 100, &()),
        Err(Error::SpaceMismatch)
    );
}

#[test]
fn dense_oracle_for_symbolic_completed_functions_in_adversarial_orders() {
    for order in [
        (0..10).rev().collect::<Vec<_>>(),
        vec![5, 0, 9, 2, 8, 1, 7, 3, 6, 4],
    ] {
        let base = Space::with_order(NAME, &order, Limits::default(), &()).unwrap();
        let mut random = 0xdead_beef_8bad_f00du64;
        let mut next = || {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            random
        };
        let support: Vec<_> = (0..16).map(|_| next()).collect();
        let space = base
            .restrict(&base.table(1023, &support, &()).unwrap(), &())
            .unwrap();
        let mut entries = Vec::new();
        for _ in 0..8 {
            let data: Vec<_> = (0..16).map(|_| next()).collect();
            entries.push((space.table(1023, &data, &()).unwrap(), data));
        }
        for step in 0u8..128 {
            let a = usize::try_from(next() % 8).unwrap();
            let b = usize::try_from(next() % 8).unwrap();
            let c = usize::try_from(next() % 8).unwrap();
            let truth = BoolOp4::new(step % 16).unwrap();
            let result = if step % 3 == 0 {
                entries[c].0.ite(&entries[a].0, &entries[b].0, &()).unwrap()
            } else {
                entries[a].0.apply(truth, &entries[b].0, &()).unwrap()
            };
            let mut expected = vec![0u64; 16];
            for w in 0..1024 {
                let bit = |data: &[u64]| data[w / 64] >> (w % 64) & 1 != 0;
                let left = bit(&entries[a].1);
                let right = bit(&entries[b].1);
                let value = if step % 3 == 0 {
                    if bit(&entries[c].1) { left } else { right }
                } else {
                    truth.evaluate(left, right)
                };
                if value {
                    expected[w / 64] |= 1 << (w % 64);
                }
                if bit(&support) {
                    assert_eq!(result.contains(w as u64).unwrap(), value);
                }
            }
            let count: u64 = expected
                .iter()
                .zip(&support)
                .map(|(a, b)| u64::from((a & b).count_ones()))
                .sum();
            assert_eq!(result.count(&()).unwrap(), count);
            assert_eq!(result, space.table(1023, &expected, &()).unwrap());
            entries[usize::from(step % 8)] = (result, expected);
        }
    }
}

#[test]
fn canonical_codec_ignores_allocation_order_working_order_and_decoder_anchor() {
    for support in 1u64..16 {
        let mut baseline = None;
        for order in [[0, 1], [1, 0]] {
            let full = Space::with_order(NAME, &order, Limits::default(), &()).unwrap();
            // Deliberately change allocation history as well as physical order.
            for bits in (0..16).rev() {
                full.table(3, &[bits], &()).unwrap();
            }
            let restricted = full
                .restrict(&full.table(3, &[support], &()).unwrap(), &())
                .unwrap();
            let mut encodings = Vec::new();
            for bits in 0..16 {
                let value = restricted.table(3, &[bits], &()).unwrap();
                let bytes = value.to_bytes(&()).unwrap();
                let reopened =
                    Event::from_bytes_with_order(&bytes, Some(&order), Limits::default(), &())
                        .unwrap();
                assert_eq!(reopened.to_bytes(&()).unwrap(), bytes);
                assert_eq!(members(&reopened, support, 4), bits & support);
                assert_eq!(reopened.count(&()).unwrap(), value.count(&()).unwrap());
                encodings.push(bytes);
            }
            if let Some(expected) = &baseline {
                assert_eq!(&encodings, expected);
            } else {
                baseline = Some(encodings);
            }
        }
    }
    let space = Space::new(NAME, 12, &()).unwrap();
    let mut value = space.empty();
    for v in 0..12 {
        value = value
            .apply(BoolOp4::XOR, &space.coordinate(v, &()).unwrap(), &())
            .unwrap();
    }
    let bytes = value.to_bytes(&()).unwrap();
    let order: Vec<_> = (0..12).rev().collect();
    let reopened =
        Event::from_bytes_with_order(&bytes, Some(&order), Limits::default(), &()).unwrap();
    assert_eq!(reopened.to_bytes(&()).unwrap(), bytes);
    assert_eq!(reopened.count(&()).unwrap(), 2048);
    assert_eq!(
        reopened.complement().to_bytes(&()).unwrap(),
        value.complement().to_bytes(&()).unwrap()
    );
}

#[test]
fn codec_rejects_truncation_corruption_noncanonical_graphs_and_versions() {
    let space = Space::new(NAME, 2, &()).unwrap();
    let x = space.coordinate(0, &()).unwrap();
    let bytes = x.to_bytes(&()).unwrap();
    for end in 0..bytes.len() {
        assert!(Event::from_bytes(&bytes[..end], &()).is_err());
    }
    let mut wrong_version = bytes.clone();
    wrong_version[4] = 255;
    assert_eq!(
        Event::from_bytes(&wrong_version, &()),
        Err(Error::UnsupportedVersion(255))
    );
    for (offset, value) in [
        (0, 0),
        (6, 1),
        (7, 1),
        (40, 255),
        (44, 255),
        (48, 255),
        (52, 9),
        (53, 1),
        (57, 0),
    ] {
        let mut corrupt = bytes.clone();
        corrupt[offset] = value;
        assert!(Event::from_bytes(&corrupt, &()).is_err(), "offset {offset}");
    }
    let mut garbage = bytes.clone();
    garbage.push(0);
    assert_eq!(
        Event::from_bytes(&garbage, &()),
        Err(Error::InvalidEncoding)
    );
    let mut dead_node = bytes.clone();
    dead_node[48..52].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        Event::from_bytes(&dead_node, &()),
        Err(Error::InvalidEncoding)
    );
    let mut outside = bytes;
    outside[44..48].copy_from_slice(&2u32.to_le_bytes()); // legal x
    outside[48..52].copy_from_slice(&1u32.to_le_bytes()); // membership full
    assert_eq!(
        Event::from_bytes(&outside, &()),
        Err(Error::InvalidEncoding)
    );
    let small = Limits {
        records: 0,
        ..Limits::default()
    };
    assert!(Event::from_bytes_with_order(&x.to_bytes(&()).unwrap(), None, small, &()).is_err());
    assert_eq!(
        x.to_bytes(&CancelAfter(Cell::new(0))),
        Err(Error::Cancelled)
    );
}

#[test]
fn registry_aligns_fact_values_without_trusting_unregistered_keys() {
    let space = Space::new(NAME, 12, &()).unwrap();
    let x = space.coordinate(0, &()).unwrap();
    let mut registry = Registry::default();
    let stored = registry.intern(&x, &()).unwrap();
    assert_eq!(registry.resolve(stored.key().words(), &()).unwrap(), stored);
    assert_eq!(
        registry.resolve(x.complement().key().words(), &()),
        Err(Error::UnknownKey)
    );
    let order: Vec<_> = (0..12).rev().collect();
    let alternate = Event::from_bytes_with_order(
        &x.to_bytes(&()).unwrap(),
        Some(&order),
        Limits::default(),
        &(),
    )
    .unwrap();
    assert_ne!(alternate.key(), x.key());
    assert_eq!(registry.intern(&alternate, &()).unwrap(), stored);
    assert_eq!(
        registry.decode(&x.to_bytes(&()).unwrap(), &()).unwrap(),
        stored
    );
    assert_eq!(registry.len(), 1);
    let other_scope = Space::new(SpaceId([2; 32]), 12, &()).unwrap();
    assert_eq!(
        alternate.align_to(&other_scope, &()),
        Err(Error::SpaceMismatch)
    );
    let constrained = space.restrict(&x, &()).unwrap();
    assert_eq!(
        alternate.align_to(&constrained, &()),
        Err(Error::SpaceMismatch)
    );
    let unavailable = Registry::new(0);
    assert_eq!(
        unavailable.resolve(stored.key().words(), &()),
        Err(Error::UnknownKey)
    );
    assert_eq!(
        registry.intern(&stored, &CancelAfter(Cell::new(0))),
        Err(Error::Cancelled)
    );
    let mut limited = Registry::new(0);
    assert_eq!(
        limited.intern(&stored, &()),
        Err(Error::Capacity(Capacity::RegistryEntries))
    );
    assert!(limited.is_empty());
    drop(registry);
    drop(space);
    assert_eq!(stored.count(&()).unwrap(), 2048);
}

#[test]
fn opposite_direction_alignment_preserves_symbolic_values_without_deadlock() {
    let first = Space::new(NAME, 12, &()).unwrap();
    let mut parity = first.empty();
    for v in 0..12 {
        parity = parity
            .apply(BoolOp4::XOR, &first.coordinate(v, &()).unwrap(), &())
            .unwrap();
    }
    let order: Vec<_> = (0..12).rev().collect();
    let second = Event::from_bytes_with_order(
        &parity.to_bytes(&()).unwrap(),
        Some(&order),
        Limits::default(),
        &(),
    )
    .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = [(parity.clone(), second.space()), (second.clone(), first)]
        .into_iter()
        .map(|(value, destination)| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..20 {
                    assert_eq!(
                        value
                            .align_to(&destination, &())
                            .unwrap()
                            .count(&())
                            .unwrap(),
                        2048
                    );
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(parity.align_to(&second.space(), &()).unwrap(), second);
}

#[test]
fn format_one_constant_golden_and_resource_refusals() {
    let space = Space::new(SpaceId([0; 32]), 0, &()).unwrap();
    let mut golden = vec![0u8; 52];
    golden[..4].copy_from_slice(b"BEVT");
    golden[4] = 1;
    golden[44] = 1;
    assert_eq!(space.empty().to_bytes(&()).unwrap(), golden);
    golden[48] = 1;
    assert_eq!(space.full().to_bytes(&()).unwrap(), golden);
    assert!(Event::from_bytes(&golden, &()).unwrap().is_full());
    for (limits, error) in [
        (
            Limits {
                operation_steps: 0,
                ..Limits::default()
            },
            Capacity::OperationSteps,
        ),
        (
            Limits {
                table_words: 0,
                ..Limits::default()
            },
            Capacity::TableWords,
        ),
    ] {
        let limited = Space::with_order(NAME, &[0], limits, &()).unwrap();
        assert_eq!(limited.coordinate(0, &()), Err(Error::Capacity(error)));
        assert!(limited.empty().is_empty());
    }
    let limited = Space::with_order(
        NAME,
        &[0, 1],
        Limits {
            memo_entries: 0,
            ..Limits::default()
        },
        &(),
    )
    .unwrap();
    let x = limited.coordinate(0, &()).unwrap();
    let y = limited.coordinate(1, &()).unwrap();
    assert_eq!(
        x.ite(&y, &limited.empty(), &()),
        Err(Error::Capacity(Capacity::MemoEntries))
    );
}
