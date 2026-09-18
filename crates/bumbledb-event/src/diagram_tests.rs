use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use crate::{
    BoolOp4, Capacity, Control, DiagramNode, DiagramView, Error, Limits, Result, Space, SpaceId,
};

const NAME: SpaceId = SpaceId([37; 32]);

// Independent interpreter of the public views. It neither calls the snapshot's
// evaluator nor accesses arena references, stored completion or codec internals.
fn evaluate(node: DiagramNode<'_>, code: u64) -> bool {
    match node.view() {
        DiagramView::Constant(bit) => bit,
        DiagramView::Split {
            coordinate,
            low,
            high,
        } => evaluate(
            if code >> coordinate & 1 == 0 {
                low
            } else {
                high
            },
            code,
        ),
        DiagramView::Table {
            coordinates,
            words,
            complemented,
        } => {
            let mut index = 0;
            for (position, coordinate) in (0..62).filter(|c| coordinates >> c & 1 != 0).enumerate()
            {
                index |= ((code >> coordinate & 1) as usize) << position;
            }
            (words[index / 64] >> (index % 64) & 1 != 0) ^ complemented
        }
    }
}

fn reachable<'a>(node: DiagramNode<'a>, seen: &mut HashSet<DiagramNode<'a>>) {
    let node = node.regular();
    if matches!(node.view(), DiagramView::Constant(_)) || !seen.insert(node) {
        return;
    }
    if let DiagramView::Split { low, high, .. } = node.view() {
        reachable(low, seen);
        reachable(high, seen);
    }
}

#[test]
fn every_small_supported_event_has_exact_public_views_and_checked_reconstruction() {
    for support in 1..16 {
        for order in [[0, 1], [1, 0]] {
            let raw = Space::with_order(NAME, &order, Limits::default(), &()).unwrap();
            let space = raw
                .restrict(&raw.table(3, &[support], &()).unwrap(), &())
                .unwrap();
            let target_raw =
                Space::with_order(NAME, &[order[1], order[0]], Limits::default(), &()).unwrap();
            let target = target_raw
                .restrict(&target_raw.table(3, &[support], &()).unwrap(), &())
                .unwrap();
            for bits in 0..16 {
                let event = space.table(3, &[bits], &()).unwrap();
                let before = space.statistics().unwrap();
                let graph = event.diagram(&()).unwrap();
                assert_eq!(space.statistics().unwrap(), before);
                assert_eq!(graph.identity(), NAME);
                assert_eq!(graph.dimensions(), 2);
                assert_eq!(graph.order(), order);
                for code in 0..4 {
                    assert_eq!(evaluate(graph.support(), code), support >> code & 1 != 0);
                    assert_ne!(
                        evaluate(graph.root(), code),
                        evaluate(graph.root().complement(), code)
                    );
                    if support >> code & 1 != 0 {
                        assert_eq!(evaluate(graph.root(), code), bits >> code & 1 != 0);
                        assert_eq!(graph.contains(code).unwrap(), bits >> code & 1 != 0);
                    } else {
                        assert_eq!(graph.contains(code), Err(Error::IllegalWorld(code)));
                    }
                }
                assert_eq!(graph.contains(4), Err(Error::IllegalWorld(4)));
                let rebuilt = graph.rebuild(&target, &()).unwrap();
                assert_eq!(rebuilt.to_bytes(&()).unwrap(), event.to_bytes(&()).unwrap());
                assert_eq!(rebuilt.align_to(&space, &()).unwrap(), event);
                let mut seen = HashSet::new();
                reachable(graph.support(), &mut seen);
                reachable(graph.root(), &mut seen);
                assert_eq!(seen.len(), graph.records());
            }
        }
    }
}

// Count assignments using only raw diagram structure. Smooth skipped axes and
// memoize regular nodes so a highly shared diagram does not expand to a tree.
fn raw_count<'a>(node: DiagramNode<'a>, memo: &mut HashMap<DiagramNode<'a>, u64>) -> u64 {
    let regular = node.regular();
    let count = if let Some(&count) = memo.get(&regular) {
        count
    } else {
        let count = match regular.view() {
            DiagramView::Constant(bit) => u64::from(bit),
            DiagramView::Table {
                coordinates,
                words,
                complemented,
            } => {
                let size = 1 << coordinates.count_ones();
                let positive = words
                    .iter()
                    .map(|word| u64::from(word.count_ones()))
                    .sum::<u64>();
                if complemented {
                    size - positive
                } else {
                    positive
                }
            }
            DiagramView::Split {
                coordinate,
                low,
                high,
            } => {
                let rest = regular.coordinates() & !(1 << coordinate);
                (raw_count(low, memo) << (rest & !low.coordinates()).count_ones())
                    + (raw_count(high, memo) << (rest & !high.coordinates()).count_ones())
            }
        };
        memo.insert(regular, count);
        count
    };
    if node.is_complemented() {
        (1 << node.coordinates().count_ones()) - count
    } else {
        count
    }
}

#[test]
fn symbolic_inspection_preserves_sharing_and_permits_external_counting() {
    for order in [(0..62).collect::<Vec<_>>(), (0..62).rev().collect()] {
        let space = Space::with_order(NAME, &order, Limits::default(), &()).unwrap();
        let mut parity = space.empty();
        for coordinate in 0..62 {
            parity = parity
                .apply(
                    BoolOp4::XOR,
                    &space.coordinate(coordinate, &()).unwrap(),
                    &(),
                )
                .unwrap();
        }
        let graph = parity.diagram(&()).unwrap();
        assert_eq!(graph.records(), 54); // 53 splits, one nine-bit local table
        assert_eq!(graph.table_words(), 8);
        assert!(graph.records() < space.statistics().unwrap().records);
        let mut seen = HashSet::new();
        reachable(graph.root(), &mut seen);
        assert_eq!(seen.len(), graph.records());
        for node in &seen {
            if let DiagramView::Split {
                low,
                high,
                coordinate,
            } = node.view()
            {
                assert_eq!(low.regular(), high.regular());
                assert_eq!(low, high.complement());
                assert_eq!(low.coordinates() & (1 << coordinate), 0);
            }
        }
        assert_eq!(raw_count(graph.root(), &mut HashMap::new()), 1 << 61);
        let mut code = 1u64;
        for _ in 0..1024 {
            code ^= code << 13;
            code ^= code >> 7;
            code ^= code << 17;
            let code = code & ((1 << 62) - 1);
            assert_eq!(
                evaluate(graph.root(), code),
                !code.count_ones().is_multiple_of(2)
            );
            assert_eq!(
                graph.contains(code).unwrap(),
                !code.count_ones().is_multiple_of(2)
            );
        }
        let target = Space::with_order(
            NAME,
            &order.into_iter().rev().collect::<Vec<_>>(),
            Limits::default(),
            &(),
        )
        .unwrap();
        assert_eq!(
            graph.rebuild(&target, &()).unwrap().count(&()).unwrap(),
            1 << 61
        );
        // Capturing constants ignores the many unreachable intermediate nodes.
        assert_eq!(space.full().diagram(&()).unwrap().records(), 0);
    }
}

#[test]
fn borrowed_tables_have_sparse_semantic_coordinates_and_exact_padding() {
    let space = Space::new(NAME, 62, &()).unwrap();
    let coordinates = (1 << 61) | (1 << 37) | (1 << 8);
    for bits in [0b1001_0110, 0b0110_1001] {
        let event = space.table(coordinates, &[bits], &()).unwrap();
        let graph = event.diagram(&()).unwrap();
        let DiagramView::Table {
            coordinates: observed,
            words,
            complemented,
        } = graph.root().view()
        else {
            panic!("three essential coordinates use one local table")
        };
        assert_eq!(observed, coordinates);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0] & !255, 0);
        for index in 0..8 {
            let code = ((index & 1) << 8) | ((index >> 1 & 1) << 37) | ((index >> 2 & 1) << 61);
            assert_eq!(
                (words[0] >> index & 1 != 0) ^ complemented,
                bits >> index & 1 != 0
            );
            assert_eq!(graph.contains(code).unwrap(), bits >> index & 1 != 0);
        }
    }
}

#[test]
fn arbitrary_symbolic_support_and_polarity_match_an_independent_bitset() {
    let order = [11, 0, 8, 4, 2, 10, 1, 9, 3, 7, 5, 6];
    let raw = Space::with_order(NAME, &order, Limits::default(), &()).unwrap();
    let mut support = [0; 64];
    let mut membership = [0; 64];
    let mut state = 123_456_789u64;
    for value in support.iter_mut().chain(membership.iter_mut()) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *value = state;
    }
    support[0] |= 1;
    let space = raw
        .restrict(&raw.table(4095, &support, &()).unwrap(), &())
        .unwrap();
    let event = space.table(4095, &membership, &()).unwrap().complement();
    let snapshot = event.diagram(&()).unwrap();
    for code in 0..4096 {
        let legal = support[code / 64] >> (code % 64) & 1 != 0;
        assert_eq!(evaluate(snapshot.support(), code as u64), legal);
        if legal {
            let present = membership[code / 64] >> (code % 64) & 1 == 0;
            assert_eq!(evaluate(snapshot.root(), code as u64), present);
            assert_eq!(snapshot.contains(code as u64).unwrap(), present);
        } else {
            assert_eq!(
                snapshot.contains(code as u64),
                Err(Error::IllegalWorld(code as u64))
            );
        }
    }
    let target_raw = Space::with_order(
        NAME,
        &order.into_iter().rev().collect::<Vec<_>>(),
        Limits::default(),
        &(),
    )
    .unwrap();
    let target = target_raw
        .restrict(&target_raw.table(4095, &support, &()).unwrap(), &())
        .unwrap();
    let rebuilt = snapshot.rebuild(&target, &()).unwrap();
    let expected = support
        .iter()
        .zip(membership)
        .map(|(s, m)| u64::from((s & !m).count_ones()))
        .sum::<u64>();
    assert_eq!(rebuilt.count(&()).unwrap(), expected);
    assert_eq!(rebuilt.to_bytes(&()).unwrap(), event.to_bytes(&()).unwrap());
    let mut seen = HashSet::new();
    reachable(snapshot.support(), &mut seen);
    reachable(snapshot.root(), &mut seen);
    assert_eq!(seen.len(), snapshot.records());
}

#[test]
fn snapshots_are_independent_owned_values_and_views_allow_same_arena_construction() {
    let (snapshot, canonical) = {
        let raw = Space::new(NAME, 12, &()).unwrap();
        let support = raw
            .coordinate(0, &())
            .unwrap()
            .apply(BoolOp4::EQUIVALENCE, &raw.coordinate(11, &()).unwrap(), &())
            .unwrap();
        let space = raw.restrict(&support, &()).unwrap();
        let event = space.coordinate(11, &()).unwrap();
        let graph = event.diagram(&()).unwrap();
        let view = graph.root().view();
        let clone = event.clone();
        std::thread::spawn(move || clone.apply(BoolOp4::AND, &clone.complement(), &()).unwrap())
            .join()
            .unwrap();
        assert!(matches!(view, DiagramView::Table { .. }));
        let graph_clone = graph.clone();
        assert_eq!(graph.root(), graph_clone.root());
        assert_ne!(graph.root(), event.diagram(&()).unwrap().root());
        (graph_clone, event.to_bytes(&()).unwrap())
    };
    let restored = crate::Event::from_bytes(&canonical, &()).unwrap();
    assert_eq!(snapshot.rebuild(&restored.space(), &()).unwrap(), restored);
    assert!(snapshot.contains(2049).unwrap());
    assert_eq!(snapshot.contains(2048), Err(Error::IllegalWorld(2048)));
    assert_eq!(snapshot.root().complement().complement(), snapshot.root());
}

#[test]
fn original_support_gates_constants_and_rebuild_admission() {
    for dimensions in [0, 2] {
        let space = Space::new(NAME, dimensions, &()).unwrap();
        for event in [space.empty(), space.full()] {
            let graph = event.diagram(&()).unwrap();
            let wrong = Space::new(SpaceId([38; 32]), dimensions, &()).unwrap();
            assert_eq!(graph.rebuild(&wrong, &()), Err(Error::SpaceMismatch));
            let wide = Space::new(NAME, dimensions + 1, &()).unwrap();
            assert_eq!(graph.rebuild(&wide, &()), Err(Error::SpaceMismatch));
            if dimensions == 2 {
                let restricted = space
                    .restrict(&space.coordinate(0, &()).unwrap(), &())
                    .unwrap();
                assert_eq!(graph.rebuild(&restricted, &()), Err(Error::SpaceMismatch));
                let restricted = restricted.full().diagram(&()).unwrap();
                assert!(evaluate(restricted.root(), 0)); // a decoder alias
                assert!(!evaluate(restricted.support(), 0));
                assert_eq!(restricted.contains(0), Err(Error::IllegalWorld(0)));
                assert_eq!(restricted.rebuild(&space, &()), Err(Error::SpaceMismatch));
            }
        }
    }
}

struct StopAfter(Cell<usize>);
impl Control for StopAfter {
    fn checkpoint(&self) -> Result<()> {
        if self.0.get() == 0 {
            return Err(Error::Cancelled);
        }
        self.0.set(self.0.get() - 1);
        Ok(())
    }
}

#[test]
fn inspection_and_reconstruction_refuse_resources_without_mutating_the_input() {
    let raw = Space::new(NAME, 10, &()).unwrap();
    let support = raw.coordinate(0, &()).unwrap();
    let space = raw.restrict(&support, &()).unwrap();
    let event = space.coordinate(9, &()).unwrap();
    let stats = space.statistics().unwrap();
    assert!(matches!(
        event.diagram(&StopAfter(Cell::new(0))),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        event.diagram(&StopAfter(Cell::new(4))),
        Err(Error::Cancelled)
    ));
    assert_eq!(space.statistics().unwrap(), stats);
    let graph = event.diagram(&()).unwrap();
    assert_eq!(
        graph.rebuild(&space, &StopAfter(Cell::new(0))),
        Err(Error::Cancelled)
    );
    let limited = Space::with_order(
        NAME,
        &[0],
        Limits {
            operation_steps: 1,
            ..Limits::default()
        },
        &(),
    )
    .unwrap();
    assert!(matches!(
        limited.full().diagram(&()),
        Err(Error::Capacity(Capacity::OperationSteps))
    ));
    let no_memo = Space::with_order(
        NAME,
        &[0],
        Limits {
            memo_entries: 0,
            ..Limits::default()
        },
        &(),
    )
    .unwrap();
    let bit = no_memo.coordinate(0, &()).unwrap();
    assert!(matches!(
        bit.diagram(&()),
        Err(Error::Capacity(Capacity::MemoEntries))
    ));
    let full = Space::new(NAME, 1, &()).unwrap();
    let graph = full.coordinate(0, &()).unwrap().diagram(&()).unwrap();
    let no_records = Space::with_order(
        NAME,
        &[0],
        Limits {
            records: 1,
            ..Limits::default()
        },
        &(),
    )
    .unwrap();
    assert_eq!(
        graph.rebuild(&no_records, &()),
        Err(Error::Capacity(Capacity::Records))
    );
    assert!(graph.contains(1).unwrap());
}
