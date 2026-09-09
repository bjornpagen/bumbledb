//! Store-free mechanism discriminator, not a published-baseline timing.
//! Mount only as a test child of aggregate::fold_row, then remove the hook.
#![allow(dead_code)]

use super::AggregateSink;
use crate::exec::run::{Bindings, Flow, LeafBatch, Sink};
use crate::exec::sink::{AggSpec, FindSpec};
use crate::ir::FoldOp;
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, OccBind, OccId, Occurrence, Role, SlotWidth};
use crate::plan::fj::{ValidatedPlan, binary2fj, factor, validate};
use crate::plan::planner::JoinOrder;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};
use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb-bench/src/boost.rs"]
mod boost;
#[path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb-bench/src/clockproxy.rs"]
mod clock;

const ROWS: usize = 128;
const ITERATIONS: usize = 32;
const SAMPLES: usize = 8;

fn plan(columns: usize, groups: usize) -> ValidatedPlan {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "WideRow".into(),
            fields: (0..columns)
                .map(|i| FieldDescriptor {
                    name: format!("v{i}").into(),
                    value_type: ValueType::U64,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .unwrap();
    let normalized = NormalizedQuery {
        dead: None,
        occurrences: vec![Occurrence {
            occ_id: OccId(0),
            bind: OccBind::Edb(RelationId(0)),
            role: Role::Positive,
            vars: (0..columns)
                .map(|i| (FieldId(i as u16), VarId(i as u16)))
                .collect(),
            filters: vec![],
            point_vars: vec![],
        }],
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: vec![],
        anti_probes: vec![],
        slot_widths: (0..columns)
            .map(|i| (VarId(i as u16), SlotWidth::of(&ValueType::U64)))
            .collect(),
    };
    let mut plan = binary2fj(
        &normalized,
        &JoinOrder {
            order: vec![OccId(0)],
            estimates: vec![ROWS as u64],
        },
    );
    factor(&mut plan);
    let sinks: BTreeSet<_> = (0..groups).map(|i| VarId(i as u16)).collect();
    let plan = validate(&plan, &normalized, &schema, &sinks).unwrap();
    for slot in 0..columns {
        assert_eq!(plan.slot_of(VarId(slot as u16)), slot);
    }
    assert!(
        plan.distinct_witness().is_some(),
        "the complete relation row is bound"
    );
    plan
}

fn value(row: usize, slot: usize, keys: usize) -> u64 {
    if slot + 1 == keys {
        row as u64
    } else {
        (slot * 17 + row % 8) as u64
    }
}

#[inline(never)]
fn direct(sink: &mut AggregateSink, batch: &LeafBatch<'_>, _: &mut Staging) {
    assert_eq!(sink.emit_batch(batch), Flow::Continue);
}

struct Staging {
    row: Vec<u64>,
    outer: Vec<usize>,
}

/// Outer-once/full-key staging with preallocated independent scratch, using
/// the SAME new numeric row-fold helper and shape refresh. No per-row source
/// search, staging allocation, or take/restore is charged to this control.
/// It does not impersonate the published binary's complete code generation.
#[inline(never)]
fn staged(sink: &mut AggregateSink, batch: &LeafBatch<'_>, staging: &mut Staging) {
    sink.refresh_shape_cache(batch.key_slots);
    let row = &mut staging.row;
    for &slot in &staging.outer {
        row[slot] = batch.bindings.get(slot);
    }
    for &entry in batch.survivors {
        for (word, &slot) in batch.key_slots.iter().enumerate() {
            row[slot] = batch.key(entry, word);
        }
        sink.fold_row(&row, |slot, _| row[slot]);
    }
    assert_eq!(
        Sink::progress(sink),
        crate::exec::sink::SinkProgress::Continue
    );
}

type Kernel = fn(&mut AggregateSink, &LeafBatch<'_>, &mut Staging);

/// Prior compact lookup ownership, using the same current numeric helper.
/// Like the staged control, this omits emit_batch's known fixture dispatch.
#[inline(never)]
fn taken(sink: &mut AggregateSink, batch: &LeafBatch<'_>, _: &mut Staging) {
    sink.refresh_shape_cache(batch.key_slots);
    let leaf_words = std::mem::take(&mut sink.cached_leaf_words);
    for &entry in batch.survivors {
        sink.fold_row(&[], |slot, _| match leaf_words[slot] {
            None => batch.bindings.get(slot),
            Some(word) => batch.key(entry, word.get() as usize - 1),
        });
    }
    sink.cached_leaf_words = leaf_words;
    assert_eq!(
        Sink::progress(sink),
        crate::exec::sink::SinkProgress::Continue
    );
}

/// Same dispatch boundary as taken(); only route ownership differs.
#[inline(never)]
fn in_place(sink: &mut AggregateSink, batch: &LeafBatch<'_>, _: &mut Staging) {
    sink.refresh_shape_cache(batch.key_slots);
    for &entry in batch.survivors {
        sink.fold_row(&[], |slot, leaf_words| match leaf_words[slot] {
            None => batch.bindings.get(slot),
            Some(word) => batch.key(entry, word.get() as usize - 1),
        });
    }
    assert_eq!(
        Sink::progress(sink),
        crate::exec::sink::SinkProgress::Continue
    );
}

fn execute(
    sink: &mut AggregateSink,
    batch: &LeafBatch<'_>,
    kernel: Kernel,
    staging: &mut Staging,
    chunk_size: usize,
) {
    sink.reset();
    for survivors in batch.survivors.chunks(chunk_size) {
        let chunk = LeafBatch {
            keys: batch.keys,
            arity: batch.arity,
            survivors,
            key_slots: batch.key_slots,
            bindings: batch.bindings,
        };
        kernel(sink, &chunk, staging);
    }
}

fn answers(sink: &mut AggregateSink) -> Vec<Vec<u64>> {
    let mut rows = Vec::new();
    sink.finalize_into(&mut Vec::new(), |row| {
        rows.push(row.to_vec());
        Ok(())
    })
    .unwrap();
    rows.sort_unstable();
    rows
}

fn measure(
    sink: &mut AggregateSink,
    batch: &LeafBatch<'_>,
    kernel: Kernel,
    staging: &mut Staging,
    chunk_size: usize,
) -> u128 {
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        // Each is a new logical execution, so the distinctness witness never
        // licenses replays within one aggregate. Includes reset in every arm.
        execute(
            black_box(&mut *sink),
            black_box(batch),
            kernel,
            black_box(&mut *staging),
            chunk_size,
        );
        black_box(&*sink);
    }
    start.elapsed().as_nanos()
}

#[test]
#[ignore = "serial release-mode warm-kernel discriminator; no profiler"]
fn wide_borrowed_input_discriminator() {
    assert!(!cfg!(debug_assertions), "release optimizer required");
    assert!(
        !cfg!(feature = "alloc-counter"),
        "timing must be uninstrumented"
    );
    boost::engage_from_env().unwrap();
    clock::warm_up(Duration::from_millis(200));
    println!(
        "PROTOCOL staged/taken/in-place/production ABCD/DCBA, {SAMPLES} samples x {ITERATIONS} executions x {ROWS} rows; chunks 1/128; reset included; no profiler"
    );
    for chunk_size in [1, ROWS] {
        for (keys, groups) in [
            (1, 1),
            (4, 1),
            (4, 4),
            (16, 1),
            (16, 16),
            (64, 1),
            (64, 64),
            (128, 1),
            (128, 128),
        ] {
            let columns = keys + 4;
            let plan = plan(columns, groups);
            let mut finds: Vec<_> = (0..groups)
                .map(|slot| FindSpec::Var { slot, width: 1 })
                .collect();
            finds.extend([
                FindSpec::Agg(AggSpec::Count),
                FindSpec::Agg(AggSpec::Fold {
                    op: FoldOp::Sum,
                    slot: keys - 1,
                    width: 1,
                    signed: false,
                }),
            ]);
            let make =
                || AggregateSink::new_distinct(&finds, columns, plan.distinct_witness().unwrap());
            let mut reference = make();
            let mut previous = make();
            let mut candidate = make();
            let mut production = make();
            let mut bindings = Bindings::new(columns);
            bindings.load_row(&vec![0; columns]);
            let key_slots: Vec<_> = (0..keys).rev().collect();
            let mut staging = Staging {
                row: vec![0; columns],
                outer: Vec::new(),
            };
            for slot in 0..columns {
                if !key_slots.contains(&slot) {
                    staging.outer.push(slot);
                }
            }
            let words: Vec<_> = (0..ROWS)
                .flat_map(|row| key_slots.iter().map(move |&slot| value(row, slot, keys)))
                .collect();
            // Odd multiplier modulo 128 visits every row once in scrambled order.
            let survivors: Vec<_> = (0..ROWS).map(|i| ((i * 37) % ROWS) as u32).collect();
            let batch = LeafBatch {
                keys: &words,
                arity: keys,
                survivors: &survivors,
                key_slots: &key_slots,
                bindings: &bindings,
            };
            let mut expected = BTreeMap::<Vec<u64>, (u64, u64)>::new();
            for row in 0..ROWS {
                let key = (0..groups).map(|slot| value(row, slot, keys)).collect();
                let entry = expected.entry(key).or_default();
                entry.0 += 1;
                entry.1 += row as u64;
            }
            let expected: Vec<_> = expected
                .into_iter()
                .map(|(mut key, (count, sum))| {
                    key.extend([count, sum]);
                    key
                })
                .collect();
            for (sink, kernel) in [
                (&mut reference, staged as Kernel),
                (&mut previous, taken as Kernel),
                (&mut candidate, in_place as Kernel),
                (&mut production, direct as Kernel),
            ] {
                for _ in 0..8 {
                    execute(sink, &batch, kernel, &mut staging, chunk_size);
                }
                assert_eq!(answers(sink), expected);
            }
            println!(
                "CAPACITY keys={keys} groups={groups} chunk={chunk_size} candidate_binding_bytes={} candidate_route_bytes={} shared_key_layout_bytes={} control_binding_bytes={} control_outer_bytes={}",
                candidate.binding_scratch.capacity() * std::mem::size_of::<u64>(),
                candidate.cached_leaf_words.capacity()
                    * std::mem::size_of::<Option<std::num::NonZeroU32>>(),
                candidate.cached_key_slots.capacity() * std::mem::size_of::<usize>(),
                staging.row.capacity() * std::mem::size_of::<u64>(),
                staging.outer.capacity() * std::mem::size_of::<usize>(),
            );
            for (label, sink, kernel) in [
                ("A0", &mut reference, staged as Kernel),
                ("B0", &mut previous, taken as Kernel),
                ("C0", &mut candidate, in_place as Kernel),
                ("D0", &mut production, direct as Kernel),
            ] {
                run_round(
                    label,
                    keys,
                    groups,
                    sink,
                    &batch,
                    kernel,
                    &mut staging,
                    chunk_size,
                    &expected,
                );
            }
            run_round(
                "D1",
                keys,
                groups,
                &mut production,
                &batch,
                direct,
                &mut staging,
                chunk_size,
                &expected,
            );
            run_round(
                "C1",
                keys,
                groups,
                &mut candidate,
                &batch,
                in_place,
                &mut staging,
                chunk_size,
                &expected,
            );
            run_round(
                "B1",
                keys,
                groups,
                &mut previous,
                &batch,
                taken,
                &mut staging,
                chunk_size,
                &expected,
            );
            run_round(
                "A1",
                keys,
                groups,
                &mut reference,
                &batch,
                staged,
                &mut staging,
                chunk_size,
                &expected,
            );
        }
    }
}

fn run_round(
    label: &str,
    keys: usize,
    groups: usize,
    sink: &mut AggregateSink,
    batch: &LeafBatch<'_>,
    kernel: Kernel,
    staging: &mut Staging,
    chunk_size: usize,
    expected: &[Vec<u64>],
) {
    // Keep every observation, including low-frequency or paused windows.
    // Do not retry or divide times by the proxy to manufacture a win.
    let pre = clock::effective_ghz();
    let mut samples = [0u128; SAMPLES];
    for sample in &mut samples {
        *sample = measure(sink, batch, kernel, staging, chunk_size);
    }
    let post = clock::effective_ghz();
    assert_eq!(answers(sink), expected);
    println!(
        "ROUND keys={keys} groups={groups} chunk={chunk_size} arm={label} iterations={ITERATIONS} rows={ROWS} pre={pre:.6} post={post:.6} flagged={} total_ns={samples:?}",
        pre.min(post) < clock::CONTAMINATION_GHZ
    );
}
