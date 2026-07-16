use super::R;
use crate::encoding::{ValueRef, encode_fact};
use crate::image::{ColumnView, LINE, PAD_MIN_STRIDE, PAD_TOLERANCE, SET_STRIDE, build};
use crate::schema::{FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

#[test]
fn twelve_column_bases_are_aligned_and_stride_padded() {
    let dir = TempDir::new("image-stride-small");
    // 12 columns, mixed widths.
    let fields: Vec<FieldDescriptor> = (0..12)
        .map(|i| FieldDescriptor {
            name: format!("f{i}").into(),
            value_type: if i % 3 == 0 {
                ValueType::Bool
            } else if i % 3 == 1 {
                ValueType::U64
            } else {
                ValueType::I64
            },
            generation: Generation::None,
        })
        .collect();
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Wide".into(),
            fields,
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    // A few rows so columns have nonzero extent.
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for row in 0..100i64 {
        let mut values = Vec::new();
        for i in 0..12 {
            values.push(match i % 3 {
                0 => ValueRef::Bool(row % 2 == 0),
                1 => ValueRef::U64(row.cast_unsigned() * 12 + i),
                _ => ValueRef::I64(row * 12 + i64::try_from(i).expect("small")),
            });
        }
        let mut bytes = Vec::new();
        encode_fact(&values, schema.relation(R).layout(), &mut bytes);
        delta.insert(&view, R, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");

    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    let mut word_addrs = Vec::new();
    let mut byte_addrs = Vec::new();
    for i in 0..12 {
        match image.column(i) {
            ColumnView::Words(w) => word_addrs.push(w.as_ptr().addr()),
            ColumnView::Bytes(b) => byte_addrs.push(b.as_ptr().addr()),
        }
    }
    for (i, addr) in word_addrs.iter().chain(&byte_addrs).enumerate() {
        assert_eq!(addr % LINE, 0, "column {i} base must be 128-byte aligned");
    }
    // The stride rule (measured): no big SAME-SLAB stride lands within
    // the tracker-aliasing tolerance of a 16 KiB multiple — lockstep
    // scans stride within a slab, so cross-slab distances (allocator
    // luck) are outside the padder's contract. (At 100 rows every
    // same-slab stride is far below PAD_MIN_STRIDE — assert the rule
    // vacuously holds here and structurally in
    // `big_column_strides_avoid_the_tracker_band`.)
    for slab in [&word_addrs, &byte_addrs] {
        for window in slab.windows(2) {
            let stride = window[1].abs_diff(window[0]);
            if stride >= PAD_MIN_STRIDE {
                let residue = stride % SET_STRIDE;
                assert!(
                    residue == 0
                        || (residue > PAD_TOLERANCE && residue < SET_STRIDE - PAD_TOLERANCE),
                    "stride {stride} sits in the tracker-aliasing band"
                );
            }
        }
    }
}

/// The stride rule under DRAM-scale spans (measured): a
/// power-of-two row span — the exact shape the old stagger rule
/// turned into a 4–6× DRAM-scan pathology — lays out with every
/// same-slab stride clear of the 16 KiB tracker band.
#[test]
fn big_column_strides_avoid_the_tracker_band() {
    // 4 u64 columns × 16384 rows: span = 128 KiB exactly (pow-2,
    // 16 KiB-multiple) — unpadded strides would land at residue 0.
    let fields: Vec<FieldDescriptor> = (0..4)
        .map(|i| FieldDescriptor {
            name: format!("c{i}").into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        })
        .collect();
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Big".into(),
            fields,
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let dir = TempDir::new("image-stride");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for row in 0..16_384u64 {
        let values = [
            ValueRef::U64(row),
            ValueRef::U64(row ^ 1),
            ValueRef::U64(row ^ 2),
            ValueRef::U64(row ^ 3),
        ];
        let mut bytes = Vec::new();
        encode_fact(&values, schema.relation(R).layout(), &mut bytes);
        delta.insert(&view, R, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");
    let addrs: Vec<usize> = (0..4)
        .map(|i| match image.column(i) {
            ColumnView::Words(w) => w.as_ptr().addr(),
            ColumnView::Bytes(_) => unreachable!("all u64"),
        })
        .collect();
    for (i, window) in addrs.windows(2).enumerate() {
        let stride = window[1] - window[0];
        assert!(stride >= PAD_MIN_STRIDE, "spans are DRAM-scale here");
        let residue = stride % SET_STRIDE;
        assert!(
            residue == 0 || (residue > PAD_TOLERANCE && residue < SET_STRIDE - PAD_TOLERANCE),
            "stride {i}→{} = {stride} sits in the tracker band (residue {residue})",
            i + 1
        );
    }
}
