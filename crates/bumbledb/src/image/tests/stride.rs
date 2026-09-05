use super::R;
use crate::image::testsupport::TestSource;
use crate::image::{ColumnView, LINE, PAD_MIN_STRIDE, PAD_TOLERANCE, SET_STRIDE};
use crate::ir::Value;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

#[test]
fn twelve_column_bases_are_aligned_and_stride_padded() {
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
    let rows: Vec<Vec<Value>> = (0..100i64)
        .map(|row| {
            (0..12)
                .map(|i| match i % 3 {
                    0 => Value::Bool(row % 2 == 0),
                    1 => Value::U64(row.cast_unsigned() * 12 + i),
                    _ => Value::I64(row * 12 + i64::try_from(i).expect("small")),
                })
                .collect()
        })
        .collect();
    let source = TestSource::new(&schema, &[(R, rows)]);
    let (_cache, image) = source.image_with_cache(R);
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

#[test]
fn big_column_strides_avoid_the_tracker_band() {
    let fields: Vec<FieldDescriptor> = (0..4)
        .map(|i| FieldDescriptor {
            name: format!("c{i}").into(),
            value_type: ValueType::U64,
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
    let rows: Vec<Vec<Value>> = (0..16_384u64)
        .map(|row| {
            vec![
                Value::U64(row),
                Value::U64(row ^ 1),
                Value::U64(row ^ 2),
                Value::U64(row ^ 3),
            ]
        })
        .collect();
    let source = TestSource::new(&schema, &[(R, rows)]);
    let (_cache, image) = source.image_with_cache(R);
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
