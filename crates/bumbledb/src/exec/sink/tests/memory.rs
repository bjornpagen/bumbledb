use super::*;
use crate::exec::run::{Bindings, Sink as _};

fn pack_capacity(sink: &AggregateSink) -> usize {
    let GroupState::Pack { claims, .. } = &sink.group_state else {
        panic!("Pack fixture")
    };
    claims.capacity() * std::mem::size_of::<Vec<[u64; 2]>>()
        + claims
            .iter()
            .map(|group| group.capacity() * 16)
            .sum::<usize>()
}

fn pack_rows(sink: &mut AggregateSink, rows: u64) {
    let mut bindings = Bindings::new(4);
    for row in 0..rows {
        bindings.set(0, row % 64);
        bindings.set(1, row * 2);
        bindings.set(2, row * 2 + 1);
        bindings.set(3, row);
        assert_eq!(sink.emit(&bindings), crate::exec::run::Flow::Continue);
    }
}

#[test]
fn release_frees_nested_pack_capacity_after_large_to_small_reuse() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let mut sink = AggregateSink::new(finds, 4);
    pack_rows(&mut sink, 8192);
    let large = pack_capacity(&sink);
    assert!(large >= 8192 * 16);
    sink.reset();
    pack_rows(&mut sink, 1);
    assert_eq!(
        pack_capacity(&sink),
        large,
        "ordinary reset keeps warm capacity"
    );
    let before = crate::alloc_counter::snapshot().window;
    sink.release_memory();
    let after = crate::alloc_counter::snapshot().window;
    assert_eq!(
        pack_capacity(&sink),
        0,
        "outer and nested claim buffers are released"
    );
    assert_eq!(sink.key_scratch.capacity(), 0);
    assert_eq!(sink.binding_scratch.capacity(), 0);
    assert_eq!(sink.group_counts.capacity(), 0);
    assert!(sink.work.is_none());
    #[cfg(feature = "alloc-counter")]
    {
        assert_eq!(after.allocs - before.allocs, 0);
        assert!(after.dealloc_bytes - before.dealloc_bytes >= large as u64);
    }
    eprintln!(
        "Pack retained capacity: large={large}, small={large}, released=0; release allocations={}, bytes freed={}",
        after.allocs - before.allocs,
        after.dealloc_bytes - before.dealloc_bytes
    );
    sink.release_memory();
    sink.reset();
    pack_rows(&mut sink, 1);
    assert!(pack_capacity(&sink) < large);
    assert_eq!(sink.into_answers().unwrap(), vec![vec![0, 0, 1]]);
}

#[test]
fn release_preserves_dense_group_shape_and_exact_float_folds() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Sum,
            slot: 1,
        }),
    ];
    let mut sink = AggregateSink::new_dense(finds, 3, &[2]);
    let mut bindings = Bindings::new(3);
    for _ in 0..3 {
        sink.reset();
        for (id, value) in [1e16, 1.0, -1e16].into_iter().enumerate() {
            bindings.set(0, 1);
            bindings.set(1, crate::F64::from(value).to_order_key());
            bindings.set(2, id as u64);
            sink.emit(&bindings);
        }
        let mut rows = Vec::new();
        sink.finalize_into(&mut Vec::new(), |row| {
            rows.push(row.to_vec());
            Ok(())
        })
        .unwrap();
        assert_eq!(rows, [vec![1, 3, crate::F64::from(1.0).to_order_key()]]);
        sink.release_memory();
        let GroupTable::Dense {
            radixes,
            table,
            ordinals,
        } = &sink.groups
        else {
            panic!("release preserves the dense plan")
        };
        assert_eq!(radixes.as_ref(), [2]);
        assert!(table.is_empty());
        assert_eq!(ordinals.capacity(), 0);
        assert_eq!(sink.float_accs.capacity(), 0);
        let GroupState::Folds { accs, .. } = &sink.group_state else {
            unreachable!()
        };
        assert_eq!(accs.capacity(), 0);
    }
}

#[test]
fn release_closes_spill_and_forgets_cancelled_work_before_reuse() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let work = crate::api::db::test_operation();
    let mut sink = AggregateSink::new(finds, 4);
    sink.begin(Some(work.clone()));
    pack_rows(&mut sink, 32);
    sink.spill_groups().unwrap();
    assert!(sink.group_state_spilled());
    let path = sink.spill.as_ref().unwrap().table.scratch_path().unwrap();
    assert!(path.exists());
    work.cancel();
    sink.release_memory();
    assert!(!sink.group_state_spilled());
    assert!(
        !path.exists(),
        "explicit release closes private scratch storage"
    );
    assert!(sink.work.is_none());
    assert!(sink.dedup.seen().unwrap().work.is_none());
    sink.reset();
    pack_rows(&mut sink, 1);
    assert_eq!(sink.into_answers().unwrap(), [vec![0, 0, 1]]);
}
