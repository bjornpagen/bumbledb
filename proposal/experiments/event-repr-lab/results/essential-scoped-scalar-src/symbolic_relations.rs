//! A real native query over a million-state chain, constructed symbolically.
//! No explicit bitset or matrix oracle is possible at the largest width.
use super::diagonal_checks::{counter_inputs, counter_outputs};
use super::relational_core::Algebra;
use super::transfer::Space;
use super::*;
use std::sync::Arc;

pub fn bench<C: Carrier>(width: u32) {
    let dimensions = 3 * width;
    let names: Vec<_> = (0..dimensions as u64).collect();
    let order = face_order(dimensions, 3, "bit-major");
    let size = 1u64 << width;
    let mut reference = C::full_space(dimensions, order.clone()).unwrap();
    let expected = counter_outputs(&mut reference, width);
    let expected_bytes = reference.packet(&names, &expected).encode();
    let expected_counts = [
        size * (size + 1) / 2 * size,
        size * (size + 1) / 2 * size,
        size * size * size,
        size * size * size,
        size * size,
    ];
    assert_eq!(expected.map(|e| reference.count(e)), expected_counts);
    drop(reference);
    let mut input_s = vec![];
    let mut program_s = vec![];
    let mut query_s = vec![];
    let mut owned_s = vec![];
    let mut iterations = vec![];
    let mut bytes = 0;
    let mut novel = 0;
    for _ in 0..trials() {
        let start = Instant::now();
        let mut c = C::full_space(dimensions, order.clone()).unwrap();
        let input_ids = counter_inputs(&mut c, width);
        let owner = Space::new(c, names.clone());
        let inputs = owner.publish(&input_ids);
        input_s.push(start.elapsed().as_secs_f64());
        let scope = inputs.keys()[0].space;
        let rows = input_ids.map(|r| vec![[0, 0, scope, r]]);
        let mut engine = native::Native::new(&rows, "clover");
        let start = Instant::now();
        let (answers, (setup, query, steps), new) = owner.compute(|c| {
            let start = Instant::now();
            let mut program = Algebra::new(c, width, true);
            let setup = start.elapsed().as_secs_f64();
            let start = Instant::now();
            let (out, rows, steps) =
                relations::query_regions_scoped(&mut engine, &mut program, 1, scope);
            let query = start.elapsed().as_secs_f64();
            assert_eq!(rows, 1);
            (out, (setup, query, steps))
        });
        owned_s.push(start.elapsed().as_secs_f64());
        program_s.push(setup);
        query_s.push(query);
        iterations.push(steps);
        assert!(new > 0);
        novel = new;
        assert_eq!(answers.counts(), expected_counts);
        assert_eq!(answers.packet().encode(), expected_bytes);
        bytes = answers.retained_bytes();
        let weak = Arc::downgrade(&owner);
        drop(engine);
        drop(inputs);
        drop(owner);
        assert_eq!(Arc::strong_count(&answers.owner), 1);
        assert_eq!(answers.counts(), expected_counts);
        black_box(&answers);
        drop(answers);
        assert!(weak.upgrade().is_none());
    }
    println!(
        "EVENT_LAB {{\"kind\":\"symbolic_relation_free_join\",\"candidate\":\"{}\",\"bits\":{},\"states\":{},\"identity_mode\":\"{}\",\"input_s\":{:?},\"program_setup_s\":{:?},\"query_s\":{:?},\"owned_compute_s\":{:?},\"iterations\":{:?},\"output_counts\":{:?},\"output_packet_bytes\":{},\"final_arena_bytes_est\":{},\"new_classes\":{},\"verified\":true}}",
        C::NAME,
        dimensions,
        size,
        super::relational_core::identity_mode(),
        input_s,
        program_s,
        query_s,
        owned_s,
        iterations,
        expected_counts,
        expected_bytes.len(),
        bytes,
        novel
    );
}
