use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::rc::Rc;

use bumbledb_core::encoding::encode_u64;

use super::{ColtSource, KeyOwned, OwnedColtSource, SourceFilter, SourceFilterOp};
use crate::base_image::{ColumnImage, RelationBaseImage, RelationStats};
use crate::diagnostics::{
    allocation_delta, allocation_snapshot, with_allocation_tracking_for_test,
};
use crate::query::model::AtomOccurrenceId;
use crate::storage_format::FactHandle;
use crate::tuple::{GhtSource, TupleCursor, TupleField, TupleSchema};

#[derive(Clone, Copy)]
struct AllocationReport {
    name: &'static str,
    intended_complexity: &'static str,
    rows: usize,
    distinct_keys: usize,
    key_width: usize,
    filtered: bool,
    alloc_calls: u64,
    allocated_bytes: u64,
    net_bytes: i128,
}

#[test]
fn colt_allocation_benchmark_report() -> Result<(), Box<dyn std::error::Error>> {
    let reports = [
        force_report("force_duplicate_k8_unfiltered", 512, 8, 8, false)?,
        force_report("force_distinct_k8_unfiltered", 512, 512, 8, false)?,
        force_report("force_duplicate_k16_filtered", 512, 8, 16, true)?,
        suffix_iteration_report()?,
        map_lookup_report()?,
        batch_fill_report()?,
    ];

    print_reports(&reports);
    assert_complexity(&reports);
    Ok(())
}

fn force_report(
    name: &'static str,
    rows: usize,
    distinct_keys: usize,
    key_width: usize,
    filtered: bool,
) -> Result<AllocationReport, Box<dyn std::error::Error>> {
    let colt = grouped_colt(rows, distinct_keys, key_width, filtered)?;
    let key = encode_key(1, key_width);
    let delta = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        assert!(colt.get(crate::tuple::EncodedTupleRef::new(&key)).is_some());
        allocation_delta(start, allocation_snapshot())
    });
    let counters = colt.counters();
    assert!(counters.map_entries_built <= distinct_keys);
    Ok(AllocationReport {
        name,
        intended_complexity: "force allocates with distinct keys plus table storage, not source rows",
        rows,
        distinct_keys,
        key_width,
        filtered,
        alloc_calls: delta.alloc_calls,
        allocated_bytes: delta.allocated_bytes,
        net_bytes: delta.net_bytes,
    })
}

fn suffix_iteration_report() -> Result<AllocationReport, Box<dyn std::error::Error>> {
    let rows = 512;
    let colt = range_colt(rows)?;
    let delta = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let mut count = 0usize;
        let result = colt.try_for_each_tuple::<(), _>(|tuple| {
            assert_eq!(tuple.bytes().len(), 8);
            count += 1;
            Ok(ControlFlow::Continue(()))
        });
        assert!(result.is_ok());
        assert_eq!(count, rows);
        allocation_delta(start, allocation_snapshot())
    });
    assert_eq!(colt.counters().hash_maps_built, 0);
    Ok(AllocationReport {
        name: "suffix_iteration_k8_unfiltered",
        intended_complexity: "suffix iteration stays streaming and does not force a map",
        rows,
        distinct_keys: rows,
        key_width: 8,
        filtered: false,
        alloc_calls: delta.alloc_calls,
        allocated_bytes: delta.allocated_bytes,
        net_bytes: delta.net_bytes,
    })
}

fn map_lookup_report() -> Result<AllocationReport, Box<dyn std::error::Error>> {
    let rows = 512;
    let distinct_keys = 8;
    let key_width = 8;
    let colt = grouped_colt(rows, distinct_keys, key_width, false)?;
    let key = encode_key(3, key_width);
    assert!(colt.get(crate::tuple::EncodedTupleRef::new(&key)).is_some());
    let delta = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        for _ in 0..1000 {
            assert!(colt.get(crate::tuple::EncodedTupleRef::new(&key)).is_some());
        }
        allocation_delta(start, allocation_snapshot())
    });
    Ok(AllocationReport {
        name: "map_lookup_repeated_k8_forced",
        intended_complexity: "borrowed-key lookup is bounded after force",
        rows,
        distinct_keys,
        key_width,
        filtered: false,
        alloc_calls: delta.alloc_calls,
        allocated_bytes: delta.allocated_bytes,
        net_bytes: delta.net_bytes,
    })
}

fn batch_fill_report() -> Result<AllocationReport, Box<dyn std::error::Error>> {
    let rows = 1024;
    let colt = range_colt(rows)?;
    let mut cursor = TupleCursor::default();
    let delta = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let batch = colt.fill_batch(&mut cursor, 4);
        assert_eq!(batch.len(), 4);
        assert!(!batch.exhausted);
        allocation_delta(start, allocation_snapshot())
    });
    Ok(AllocationReport {
        name: "batch_fill_k8_size4_unfiltered",
        intended_complexity: "batch fill allocates with batch size, not source rows",
        rows,
        distinct_keys: rows,
        key_width: 8,
        filtered: false,
        alloc_calls: delta.alloc_calls,
        allocated_bytes: delta.allocated_bytes,
        net_bytes: delta.net_bytes,
    })
}

fn assert_complexity(reports: &[AllocationReport]) {
    for report in reports {
        match report.name {
            "force_duplicate_k8_unfiltered" | "force_duplicate_k16_filtered" => {
                assert!(report.alloc_calls < report.rows as u64);
            }
            "map_lookup_repeated_k8_forced" => assert!(report.alloc_calls < 1000),
            "batch_fill_k8_size4_unfiltered" => assert!(report.alloc_calls < 128),
            _ => {}
        }
    }
}

fn print_reports(reports: &[AllocationReport]) {
    print!("{{\"colt_allocation_reports\":[");
    for (index, report) in reports.iter().enumerate() {
        if index > 0 {
            print!(",");
        }
        print!(
            "{{\"name\":\"{}\",\"intended_complexity\":\"{}\",\"rows\":{},\"distinct_keys\":{},\"key_width\":{},\"filtered\":{},\"alloc_calls\":{},\"allocated_bytes\":{},\"net_bytes\":{}}}",
            report.name,
            report.intended_complexity,
            report.rows,
            report.distinct_keys,
            report.key_width,
            report.filtered,
            report.alloc_calls,
            report.allocated_bytes,
            report.net_bytes,
        );
    }
    println!("]}}");
}

fn grouped_colt(
    rows: usize,
    distinct_keys: usize,
    key_width: usize,
    filtered: bool,
) -> Result<OwnedColtSource, crate::tuple::TupleError> {
    let image = RelationBaseImage {
        relation_id: 0,
        name: "Grouped".to_owned(),
        row_handles: Rc::new(row_handles(rows)),
        columns: columns(rows, distinct_keys, key_width),
        stats: RelationStats { row_count: rows },
    };
    let filters = if filtered {
        vec![SourceFilter::Compare {
            field_id: 2,
            op: SourceFilterOp::Lte,
            value: KeyOwned::from_slice(&encode_u64((rows - 1) as u64)),
        }]
    } else {
        Vec::new()
    };
    Ok(ColtSource::new_filtered(
        AtomOccurrenceId(0),
        Rc::new(image),
        force_schemas(key_width)?,
        filters,
    ))
}

fn range_colt(rows: usize) -> Result<OwnedColtSource, crate::tuple::TupleError> {
    let image = RelationBaseImage {
        relation_id: 0,
        name: "Range".to_owned(),
        row_handles: Rc::new(row_handles(rows)),
        columns: BTreeMap::from([(0, u64_column(0, 0..rows as u64))]),
        stats: RelationStats { row_count: rows },
    };
    Ok(ColtSource::new(
        AtomOccurrenceId(0),
        Rc::new(image),
        vec![TupleSchema::new(vec![field(0, 0)?])],
    ))
}

fn force_schemas(key_width: usize) -> Result<Vec<TupleSchema>, crate::tuple::TupleError> {
    let key = if key_width == 16 {
        TupleSchema::new(vec![field(0, 0)?, field(1, 1)?])
    } else {
        TupleSchema::new(vec![field(0, 0)?])
    };
    Ok(vec![key, TupleSchema::new(vec![field(2, 2)?])])
}

fn columns(rows: usize, distinct_keys: usize, key_width: usize) -> BTreeMap<usize, ColumnImage> {
    let second_key_values = if key_width == 16 {
        u64_column(1, std::iter::repeat_n(0, rows))
    } else {
        u64_column(1, (0..rows).map(|offset| (offset / distinct_keys) as u64))
    };
    BTreeMap::from([
        (
            0,
            u64_column(0, (0..rows).map(|offset| (offset % distinct_keys) as u64)),
        ),
        (1, second_key_values),
        (2, u64_column(2, 0..rows as u64)),
    ])
}

fn encode_key(value: u64, key_width: usize) -> Vec<u8> {
    let mut bytes = encode_u64(value).to_vec();
    if key_width == 16 {
        bytes.extend_from_slice(&encode_u64(0));
    }
    bytes
}

fn row_handles(rows: usize) -> Vec<FactHandle> {
    (0..rows)
        .map(|offset| FactHandle([offset as u8; 16]))
        .collect()
}

fn u64_column(field_id: usize, values: impl IntoIterator<Item = u64>) -> ColumnImage {
    let values = values.into_iter().collect::<Vec<_>>();
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&encode_u64(value));
    }
    ColumnImage {
        field_id,
        width: 8,
        values: Rc::new(bytes),
        row_offsets: None,
    }
}

fn field(variable: usize, field_id: usize) -> Result<TupleField, crate::tuple::TupleError> {
    TupleField::new(variable, Some(field_id), 8)
}
