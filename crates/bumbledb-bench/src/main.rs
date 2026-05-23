#![allow(clippy::result_large_err)]

use std::fmt::Write as _;
use std::fs::File;
use std::hint::black_box;
use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, TypedQuery};
use bumbledb_core::schema::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor,
    SchemaDescriptor, ValueType,
};
use bumbledb_lmdb::{
    AllocationPhaseStats, Environment, Fact, InputBindings, PlanCounters, QueryAllocationStats,
    QueryOutput, QueryPlan, QueryTimings, StorageSchema, Value,
};
use rusqlite::{Connection, params_from_iter};
use tracing_subscriber::fmt::format::FmtSpan;

mod open;

const DEFAULT_OPEN_LIMIT: usize = 100_000;

#[cfg(feature = "alloc-profile")]
mod alloc_profile {
    use std::alloc::{GlobalAlloc, Layout, System};

    pub struct CountingAllocator;

    // SAFETY: this allocator forwards all operations to the standard system
    // allocator and only records successful operations with lock-free atomics.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarding the exact layout to the system allocator.
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() {
                bumbledb_lmdb::allocation::record_alloc(layout.size());
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            bumbledb_lmdb::allocation::record_dealloc(layout.size());
            // SAFETY: forwarding the original pointer and layout to the system allocator.
            unsafe { System.dealloc(ptr, layout) };
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: forwarding the original pointer, layout, and requested new size.
            let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
            if !new_ptr.is_null() {
                bumbledb_lmdb::allocation::record_realloc(layout.size(), new_size);
            }
            new_ptr
        }
    }
}

#[cfg(feature = "alloc-profile")]
#[global_allocator]
static GLOBAL_ALLOCATOR: alloc_profile::CountingAllocator = alloc_profile::CountingAllocator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = Config::from_env()? else {
        return Ok(());
    };
    if config.trace {
        init_tracing(&config)?;
    }
    if !config.format.is_json_only() {
        println!("BumbleDB benchmark suite");
        println!(
            "scale={} open_limit={:?} repeats={} warmup={} cache_mode={} datasets={:?} queries={:?} open_datasets={}",
            config.scale,
            config.open_limit,
            config.repeats,
            config.warmup,
            config.cache_mode.as_str(),
            config.datasets,
            config.queries,
            config.has_open_datasets()
        );
        println!();
    }

    let mut datasets = all_datasets(config.scale);
    datasets.extend(open::open_datasets(&config)?);

    let datasets = datasets
        .into_iter()
        .filter(|dataset| {
            config.datasets.is_empty() || config.datasets.iter().any(|name| name == dataset.name)
        })
        .collect::<Vec<_>>();

    if datasets.is_empty() {
        return Err("no matching datasets".into());
    }

    let mut results = Vec::new();
    for dataset in datasets {
        results.extend(run_dataset(dataset, &config)?);
        if !config.format.is_json_only() {
            println!();
        }
    }

    if results.is_empty() {
        return Err(bench_error("no matching queries"));
    }

    if config.format.includes_markdown() {
        println!("{}", render_markdown_results(&results));
    }
    if config.format.includes_json() {
        println!("{}", render_json_results(&results));
    }

    if config.fail_gates {
        let failures = results
            .iter()
            .filter(|result| !result.gate.passed)
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            return Err(format!("{} benchmark gate(s) failed", failures.len()).into());
        }
    }

    Ok(())
}

include!("main/config.rs");

include!("main/tracing.rs");

include!("main/types.rs");

include!("main/run.rs");

include!("main/timing.rs");

include!("main/sqlite.rs");

include!("main/result.rs");

include!("main/render_markdown.rs");

include!("main/render_json.rs");

include!("main/datasets.rs");

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
