//! Only inputs cross the boundary. New results are computed in the retained owner.
use super::relational_core::Algebra;
use super::transfer::*;
use super::*;
use std::sync::Arc;

pub fn bench_owned<C: Carrier>(nbits: u32) {
    for layout in ["bit-major", "face-major"] {
        one::<C, C>(nbits, layout);
    }
    if C::NAME != Packed::<8>::NAME {
        one::<C, Packed<8>>(nbits, "bit-major");
    }
}
fn one<C: Carrier, T: Carrier>(nbits: u32, target_layout: &str) {
    let f = relations::fixture(nbits);
    let names: Vec<_> = (0..nbits as u64).collect();
    let support = bits(f.n, |_| true);
    let mut source = C::with_order(&support, f.n, relation_order(nbits));
    let input_ids: Vec<_> = f.bank.iter().map(|words| source.import(words)).collect();
    let encoded = source.packet(&names, &input_ids).encode();
    let packet = Packet::decode(&encoded).unwrap();
    assert_eq!(packet.roots.len(), f.bank.len());
    drop(source);
    // The oracle manager is never used as the query manager or input packet source.
    let mut oracle = T::with_order(&support, f.n, face_order(nbits, 3, target_layout));
    let expected_ids: Vec<_> = f
        .expected
        .iter()
        .map(|words| oracle.import(words))
        .collect();
    let expected_bytes = oracle.packet(&names, &expected_ids).encode();
    drop(oracle);
    let mut setup_s = vec![];
    let mut restore_s = vec![];
    let mut program_s = vec![];
    let mut query_s = vec![];
    let mut compute_s = vec![];
    let mut novel = vec![];
    let mut final_bytes = 0;
    let mut input_storage = None;
    let mut final_storage = None;
    for _ in 0..trials() {
        let start = Instant::now();
        let owner = Space::new(
            T::full_space(nbits, face_order(nbits, 3, target_layout)).unwrap(),
            names.clone(),
        );
        setup_s.push(start.elapsed().as_secs_f64());
        let weak = Arc::downgrade(&owner);
        let start = Instant::now();
        let inputs = restore::<C, T>(
            &packet,
            &owner,
            &(0..nbits).collect::<Vec<_>>(),
            MapKind::Extension,
        )
        .unwrap();
        restore_s.push(start.elapsed().as_secs_f64());
        input_storage = owner.inspect_inputs(inputs.keys(), |c, _| c.memory_stats()).unwrap();
        assert_eq!(inputs.export(), f.bank);
        let scope = inputs.keys()[0].space;
        let data = f.raw.each_ref().map(|rows| {
            rows.iter()
                .map(|r| {
                    let key = inputs.keys()[r[3] as usize];
                    [r[0], r[1], key.space, key.region]
                })
                .collect()
        });
        let mut engine = native::Native::new(&data, "clover");
        let start = Instant::now();
        let (answers, (rows, program_time, query_time), new) = owner.compute(|carrier| {
            let begin = Instant::now();
            let mut algebra = Algebra::new(carrier, f.width, true);
            let program_time = begin.elapsed().as_secs_f64();
            let begin = Instant::now();
            let (outputs, rows, _) =
                relations::query_regions_scoped(&mut engine, &mut algebra, f.groups, scope);
            let query_time = begin.elapsed().as_secs_f64();
            (outputs, (rows, program_time, query_time))
        });
        compute_s.push(start.elapsed().as_secs_f64());
        program_s.push(program_time);
        query_s.push(query_time);
        novel.push(new);
        assert_eq!(rows, f.expected_rows);
        assert!(new > 0, "query must publish genuinely new classes");
        assert!(answers.keys().iter().all(|key| key.space == scope));
        assert_eq!(answers.export(), f.expected);
        assert_eq!(answers.packet().encode(), expected_bytes);
        final_bytes = answers.retained_bytes();
        final_storage = owner.inspect_inputs(answers.keys(), |c, _| c.memory_stats()).unwrap();
        // The only remaining scope owner is the result batch itself.
        drop(engine);
        drop(inputs);
        drop(owner);
        assert_eq!(Arc::strong_count(&answers.owner), 1);
        for &key in answers.keys() {
            assert_eq!(answers.owner.resolve(key).unwrap(), key.region);
        }
        assert_eq!(answers.export(), f.expected);
        black_box(&answers);
        drop(answers);
        assert!(weak.upgrade().is_none());
    }
    println!(
        "EVENT_LAB {{\"kind\":\"owned_free_join\",\"candidate\":\"{}\",\"target_candidate\":\"{}\",\"bits\":{},\"target_layout\":\"{}\",\"target_setup_s\":{:?},\"restore_inputs_s\":{:?},\"program_setup_s\":{:?},\"query_s\":{:?},\"owned_compute_s\":{:?},\"novel_publication_classes\":{:?},\"input_roots\":{},\"output_roots\":{},\"input_packet_bytes\":{},\"output_packet_bytes\":{},\"final_arena_bytes_est\":{},\"input_storage\":{},\"final_storage\":{},\"verified\":true}}",
        C::NAME,
        T::NAME,
        nbits,
        target_layout,
        setup_s,
        restore_s,
        program_s,
        query_s,
        compute_s,
        novel,
        packet.roots.len(),
        f.expected.len(),
        encoded.len(),
        expected_bytes.len(),
        final_bytes,
        memory_json(input_storage),
        memory_json(final_storage)
    );
}
