//! Ordinary row owners: consuming iteration moves values, while borrowed
//! visitation releases each decoded payload and preserves reusable capacity.

use super::*;

fn text_fields() -> [FieldDescriptor; 1] {
    [FieldDescriptor {
        name: "text".into(),
        value_type: ValueType::String,
    }]
}

#[test]
fn canonical_bytes_transfer_without_copying() {
    let work = WorkContext::new();
    let row =
        CanonicalRow::encode(&text_fields(), &[Value::String("payload".into())], &work).unwrap();
    let address = row.as_bytes().as_ptr();
    let bytes = row.into_bytes();
    assert_eq!(bytes.as_ptr(), address);
    assert_eq!(
        decode(&text_fields(), &bytes, &work).unwrap().values(),
        &[Value::String("payload".into())]
    );
}

#[test]
fn decoded_row_iterators_borrow_or_move_existing_values() {
    let work = WorkContext::new();
    let fields = text_fields();
    let row = CanonicalRow::encode(&fields, &[Value::String("independent".into())], &work).unwrap();
    let decoded = decode(&fields, &row, &work).unwrap();
    drop(row);
    drop(work);
    let Value::String(text) = &decoded.values()[0] else {
        panic!("text row")
    };
    let address = text.as_ptr();
    assert!(std::ptr::eq(
        (&decoded).into_iter().next().unwrap(),
        &raw const decoded.values()[0]
    ));
    let mut values = decoded.into_iter();
    assert_eq!(values.len(), 1);
    let Value::String(text) = values.next().unwrap() else {
        panic!("text row")
    };
    assert_eq!(text.as_ref(), "independent");
    assert_eq!(text.as_ptr(), address, "consuming a row moves its payload");
    assert!(values.next().is_none());
}

#[test]
#[cfg(feature = "alloc-counter")]
fn decode_visitor_releases_large_payloads_and_reuses_small_row_capacity() {
    let work = WorkContext::new();
    let fields = text_fields();
    let large =
        CanonicalRow::encode(&fields, &[Value::String("x".repeat(65_536).into())], &work).unwrap();
    let numeric = [FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
    }];
    let small = CanonicalRow::encode(&numeric, &[Value::U64(7)], &work).unwrap();
    let mut scratch = DecodeScratch::new(&work);
    scratch.prepare(1).unwrap();
    let before = crate::alloc_counter::snapshot();
    for _ in 0..32 {
        scratch
            .with_decoded(&fields, &large, |values| {
                let Value::String(text) = &values[0] else {
                    panic!("text row")
                };
                assert_eq!(text.len(), 65_536);
                Ok::<_, RowError>(())
            })
            .unwrap();
    }
    let after_large = crate::alloc_counter::snapshot();
    assert_eq!(
        after_large.absolute.live_bytes, before.absolute.live_bytes,
        "only vector capacity survives each visitor"
    );
    assert_eq!(
        after_large.window.allocs - before.window.allocs,
        32,
        "one text allocation per visit, no charge or vector allocation"
    );
    for _ in 0..128 {
        scratch
            .with_decoded(&numeric, &small, |values| {
                assert_eq!(values, &[Value::U64(7)]);
                Ok::<_, RowError>(())
            })
            .unwrap();
    }
    let after_small = crate::alloc_counter::snapshot();
    assert_eq!(
        after_small.window.allocs, after_large.window.allocs,
        "warm scalar visits reuse the vector without allocating"
    );
}
