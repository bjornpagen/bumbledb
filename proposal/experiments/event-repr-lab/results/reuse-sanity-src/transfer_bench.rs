//! Real Free Join outputs, owned rebase, and the same query in the destination.
use super::relational_core::Algebra;
use super::transfer::*;
use super::*;

pub fn bench_transfer<C: Carrier>(nbits: u32) {
    for layout in ["bit-major", "face-major"] {
        one::<C, C>(nbits, layout);
    }
    if C::NAME != Packed::<8>::NAME {
        one::<C, Packed<8>>(nbits, "bit-major");
    }
}
fn one<C: Carrier, T: Carrier>(nbits: u32, target_layout: &str) {
    let product_setup = match std::env::var("EVENT_LAB_TRANSFER_SETUP").as_deref() {
        Ok("product") => true,
        Ok("fixture") | Err(std::env::VarError::NotPresent) => false,
        _ => panic!("invalid EVENT_LAB_TRANSFER_SETUP"),
    };
    let f = relations::fixture(nbits);
    let support = bits(f.n, |_| true);
    let names: Vec<_> = (0..nbits as u64).collect();
    let mut input = C::with_order(&support, f.n, relation_order(nbits));
    let ids: Vec<_> = f.bank.iter().map(|b| input.import(b)).collect();
    let base = Algebra::new(input, f.width, true);
    let rows = f.raw.each_ref().map(|rows| {
        rows.iter()
            .map(|r| [r[0], r[1], r[2], ids[r[3] as usize]])
            .collect()
    });
    let mut engine = native::Native::new(&rows, "clover");
    let expected: Vec<_> = f.bank.iter().chain(&f.expected).cloned().collect();
    let mut canonical = T::with_order(&support, f.n, face_order(nbits, 3, target_layout));
    let canonical_roots: Vec<_> = expected.iter().map(|b| canonical.import(b)).collect();
    let canonical_bytes = canonical.packet(&names, &canonical_roots).encode();
    {
        let mut queries = vec![];
        let mut encodes = vec![];
        let mut decodes = vec![];
        let mut target_setups = vec![];
        let mut restores = vec![];
        let mut packet_bytes = 0;
        let mut packet_nodes = 0;
        let mut source_bytes = 0;
        let mut target_bytes = 0;
        for _ in 0..trials() {
            let mut source = base.clone();
            let start = Instant::now();
            let (out, matched, _) = relations::query_regions(&mut engine, &mut source, f.groups);
            queries.push(start.elapsed().as_secs_f64());
            assert_eq!(matched, f.expected_rows);
            let mut roots = ids.clone();
            roots.extend(&out);
            let start = Instant::now();
            let packet = source.base.inner.packet(&names, &roots);
            let encoded = packet.encode();
            encodes.push(start.elapsed().as_secs_f64());
            packet_bytes = encoded.len();
            packet_nodes = packet.nodes.len();
            source_bytes = source.bytes();
            let start = Instant::now();
            let decoded = Packet::decode(&encoded).unwrap();
            decodes.push(start.elapsed().as_secs_f64());
            assert_eq!(packet, decoded);
            let start = Instant::now();
            let order = face_order(nbits, 3, target_layout);
            let target = Space::new(
                if product_setup {
                    T::full_space(nbits, order).unwrap()
                } else {
                    T::with_order(&support, f.n, order)
                },
                names.clone(),
            );
            target_setups.push(start.elapsed().as_secs_f64());
            let start = Instant::now();
            let batch = restore::<C, T>(
                &decoded,
                &target,
                &(0..nbits).collect::<Vec<_>>(),
                MapKind::Extension,
            )
            .unwrap();
            restores.push(start.elapsed().as_secs_f64());
            target_bytes = batch.retained_bytes();
            assert_eq!(batch.export(), expected);
            assert_eq!(
                batch.packet().encode(),
                canonical_bytes,
                "canonical destination bytes"
            );
            // Native rows now carry the actual destination scope token, retained by batch.
            let data = f.raw.each_ref().map(|rows| {
                rows.iter()
                    .map(|r| {
                        let key = batch.keys()[r[3] as usize];
                        [r[0], r[1], key.space, key.region]
                    })
                    .collect()
            });
            let scope = batch.keys()[0].space;
            let mut destination = native::Native::new(&data, "clover");
            let mut c = Algebra::new(batch.carrier_snapshot(), f.width, true);
            let (again, rows, _) =
                relations::query_regions_scoped(&mut destination, &mut c, f.groups, scope);
            assert_eq!(rows, f.expected_rows);
            assert_eq!(
                again,
                batch.keys()[ids.len()..]
                    .iter()
                    .map(|k| k.region)
                    .collect::<Vec<_>>()
            );
            for &key in batch.keys() {
                assert_eq!(target.resolve(key).unwrap(), key.region);
            }
            black_box(&batch);
        }
        println!(
            "EVENT_LAB {{\"kind\":\"transfer_free_join\",\"candidate\":\"{}\",\"bits\":{},\"target_layout\":\"{}\",\"target_candidate\":\"{}\",\"query_s\":{:?},\"encode_s\":{:?},\"decode_s\":{:?},\"target_setup_s\":{:?},\"restore_s\":{:?},\"packet_bytes\":{},\"packet_nodes\":{},\"canonical_packet_bytes\":{},\"canonical_packet_hash\":{},\"source_bytes_est\":{},\"target_bytes_est\":{},\"roots\":{},\"rows\":{},\"verified\":true}}",
            C::NAME,
            nbits,
            target_layout,
            T::NAME,
            queries,
            encodes,
            decodes,
            target_setups,
            restores,
            packet_bytes,
            packet_nodes,
            canonical_bytes.len(),
            hash(&canonical_bytes),
            source_bytes,
            target_bytes,
            ids.len() + f.expected.len(),
            f.expected_rows
        );
    }
}
