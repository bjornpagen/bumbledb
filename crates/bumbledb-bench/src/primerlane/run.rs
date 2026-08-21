//! The primerlane orchestration: builder lane → delta lane → read lane,
//! one function per arm (10-measurement.md; the seams the count read
//! lane and the accepted-collection arm extend). Wall time is std
//! `Instant` per phase; `--alloc` wraps each phase in an allocation
//! window; `--trace` holds one capture over the lanes and folds it into
//! the component table.

use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::schema::{FieldId, RelationId, SchemaDescriptor};
use bumbledb::{Admission, AdmissionTelemetry, Db, FreshField, InstanceBuilder};

use crate::cli::PrimerlaneArgs;

use super::report::{PhaseAlloc, PhaseRow, PrimerlaneReport, to_json, to_markdown};
use super::{PrimerConfig, components, corpus};

/// The measured-mode admission twin (the `measure.rs` idiom): the
/// feature fork lives here, so the run body is `#[cfg]`-free. Off-obs,
/// `--trace` and `--alloc` are typed refusals — a flag that silently
/// measured nothing would be a lie.
mod obs_gate {
    #[cfg(feature = "obs")]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "signature twin of the feature-off refusal (the obs.rs law)"
    )]
    pub(super) fn require(mode: &str) -> Result<(), String> {
        let _ = mode;
        Ok(())
    }

    #[cfg(not(feature = "obs"))]
    pub(super) fn require(mode: &str) -> Result<(), String> {
        Err(format!(
            "`--{mode}` needs the obs feature build (bumbledb/trace + bumbledb/alloc-counter)"
        ))
    }
}

/// Runs `f` as one named phase: optional allocation window around it,
/// wall time by `Instant`, the row appended in run order.
///
/// # Panics
///
/// If a duration in nanoseconds does not fit in `u64`.
fn phase<R>(
    phases: &mut Vec<PhaseRow>,
    name: &'static str,
    rows: u64,
    alloc: bool,
    f: impl FnOnce() -> Result<R, String>,
) -> Result<R, String> {
    if alloc {
        bumbledb::alloc_counter::reset();
    }
    let start = Instant::now();
    let value = f()?;
    let wall_ns = u64::try_from(start.elapsed().as_nanos()).expect("fits");
    let alloc = alloc.then(|| {
        let snap = bumbledb::alloc_counter::snapshot();
        PhaseAlloc {
            allocs: snap.window.allocs,
            deallocs: snap.window.deallocs,
            alloc_bytes: snap.window.alloc_bytes,
            dealloc_bytes: snap.window.dealloc_bytes,
            peak_live_bytes: snap.absolute.peak_live_bytes,
        }
    });
    phases.push(PhaseRow {
        name,
        wall_ns,
        rows,
        alloc,
    });
    Ok(value)
}

fn rel_at(idx: usize) -> RelationId {
    RelationId(u32::try_from(idx).expect("relation count fits u32"))
}

/// The fresh-field witnesses, resolved ONCE per store handle (never per
/// call — the V4 lesson): field 0 is the fresh id on every generated
/// relation.
fn fresh_fields(
    db: &Db<SchemaDescriptor>,
    counts: &[u64],
) -> Result<Vec<FreshField<SchemaDescriptor>>, String> {
    (0..counts.len())
        .map(|idx| {
            db.fresh_field(rel_at(idx), FieldId(0))
                .map_err(|e| format!("fresh field {idx}: {e:?}"))
        })
        .collect()
}

/// The builder write lane: `load_dyn` per relation → `admit` →
/// `Db::from_instance` into the scratch dir (Primer's persist path).
/// Returns the admission telemetry $A,I,R,F,J$.
fn builder_lane(
    cfg: &PrimerConfig,
    counts: &[u64],
    descriptor: &SchemaDescriptor,
    store: &Path,
    alloc: bool,
    phases: &mut Vec<PhaseRow>,
) -> Result<AdmissionTelemetry, String> {
    let total: u64 = counts.iter().sum();
    let mut builder =
        InstanceBuilder::new(descriptor.clone()).map_err(|e| format!("builder: {e:?}"))?;
    phase(phases, "builder_load", total, alloc, || {
        for (idx, &n) in counts.iter().enumerate() {
            let rel = rel_at(idx);
            let fresh = builder
                .fresh_field(rel, FieldId(0))
                .map_err(|e| format!("fresh field {idx}: {e:?}"))?;
            let range = builder
                .reserve_at(fresh, n)
                .map_err(|e| format!("reserve {idx}: {e:?}"))?;
            assert_eq!(range.start(), Some(0), "builder ids are index-aligned");
            builder
                .load_dyn(rel, (0..n).map(|i| corpus::row(cfg, counts, rel, i)))
                .map_err(|e| format!("load {idx}: {e:?}"))?;
        }
        Ok(())
    })?;
    let (admission, telemetry) = phase(phases, "builder_admit", total, alloc, || {
        builder
            .admit_measured()
            .map_err(|e| format!("admit: {e:?}"))
    })?;
    let instance = match admission {
        Admission::Accepted(instance) => instance,
        Admission::Rejected(v) => return Err(format!("builder admission rejected: {v}")),
    };
    phase(phases, "builder_publish", total, alloc, || {
        Db::from_instance(store, &instance).map_err(|e| format!("from_instance: {e:?}"))
    })?;
    Ok(telemetry)
}

/// The delta write lane: `Db::create`, seed the corpus's first halves in
/// one commit, then `insert_dyn` the second halves in one commit — the
/// incremental path against an existing store, full commit pipeline.
/// Returns the store for the read lane.
fn delta_lane(
    cfg: &PrimerConfig,
    counts: &[u64],
    descriptor: &SchemaDescriptor,
    store: &Path,
    alloc: bool,
    phases: &mut Vec<PhaseRow>,
) -> Result<Db<SchemaDescriptor>, String> {
    let total: u64 = counts.iter().sum();
    let seeded: u64 = counts.iter().map(|n| n / 2).sum();
    let db = phase(phases, "delta_create", 0, alloc, || {
        match Db::create(store, descriptor.clone()).map_err(|e| format!("create: {e:?}"))? {
            Admission::Accepted(db) => Ok(db),
            Admission::Rejected(v) => Err(format!("empty admission rejected: {v}")),
        }
    })?;
    let fresh = fresh_fields(&db, counts)?;
    phase(phases, "delta_seed", seeded, alloc, || {
        commit_halves(cfg, counts, &db, &fresh, Half::First)
    })?;
    phase(phases, "delta_write", total - seeded, alloc, || {
        commit_halves(cfg, counts, &db, &fresh, Half::Second)
    })?;
    Ok(db)
}

/// Which half of every relation one commit carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Half {
    First,
    Second,
}

/// One `db.write` carrying `half` of every relation: reserve the index
/// range, then `insert_dyn` the generated rows.
fn commit_halves(
    cfg: &PrimerConfig,
    counts: &[u64],
    db: &Db<SchemaDescriptor>,
    fresh: &[FreshField<SchemaDescriptor>],
    half: Half,
) -> Result<(), String> {
    let committed = db
        .write(|tx| {
            for (idx, &n) in counts.iter().enumerate() {
                let rel = rel_at(idx);
                let (start, end) = match half {
                    Half::First => (0, n / 2),
                    Half::Second => (n / 2, n),
                };
                let range = tx.reserve_at(fresh[idx], end - start)?;
                assert_eq!(range.start(), Some(start), "delta ids are index-aligned");
                tx.insert_dyn(rel, (start..end).map(|i| corpus::row(cfg, counts, rel, i)))?;
            }
            Ok(())
        })
        .map_err(|e| format!("delta write: {e:?}"))?;
    match committed {
        Admission::Accepted(_) => Ok(()),
        Admission::Rejected(v) => Err(format!("delta commit rejected: {v}")),
    }
}

/// The read lane: full `scan` decode per relation over the delta store
/// — the regression baseline. The exact-count read lane
/// (40-exact-count.md) and the accepted-collection arm
/// (20-accepted-collection.md) land beside this function as their own
/// arms.
fn scan_lane(
    db: &Db<SchemaDescriptor>,
    counts: &[u64],
    alloc: bool,
    phases: &mut Vec<PhaseRow>,
) -> Result<(), String> {
    let total: u64 = counts.iter().sum();
    let scanned = phase(phases, "scan_decode", total, alloc, || {
        db.read(|snap| {
            let mut rows = 0u64;
            for idx in 0..counts.len() {
                for fact in snap.scan(rel_at(idx))? {
                    std::hint::black_box(fact?);
                    rows += 1;
                }
            }
            Ok(rows)
        })
        .map_err(|e| format!("scan: {e:?}"))
    })?;
    if scanned == total {
        Ok(())
    } else {
        Err(format!(
            "scan decoded {scanned} facts, the generator wrote {total}"
        ))
    }
}

/// Runs the primerlane: both write lanes, the scan read lane, one
/// report.
///
/// # Errors
///
/// Setup, admission, or commit failure; a mode flag on a build without
/// the obs feature.
pub fn run(args: &PrimerlaneArgs) -> Result<i32, String> {
    if args.trace {
        obs_gate::require("trace")?;
    }
    if args.alloc {
        obs_gate::require("alloc")?;
    }
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-primerlane",
            crate::report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    let scratch = args.dir.join(format!("primerlane-scratch-{}", args.seed));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| format!("scratch: {e}"))?;

    let cfg = PrimerConfig {
        relations: args.relations,
        facts: args.facts,
        seed: args.seed,
    };
    let counts = corpus::relation_rows(&cfg);
    let descriptor = corpus::descriptor(&cfg);

    if args.trace {
        bumbledb::obs::start_capture();
    }
    let mut phases = Vec::new();
    let telemetry = builder_lane(
        &cfg,
        &counts,
        &descriptor,
        &scratch.join("builder-store"),
        args.alloc,
        &mut phases,
    )?;
    let db = delta_lane(
        &cfg,
        &counts,
        &descriptor,
        &scratch.join("delta-store"),
        args.alloc,
        &mut phases,
    )?;
    scan_lane(&db, &counts, args.alloc, &mut phases)?;
    drop(db);
    let components = if args.trace {
        let events = bumbledb::obs::finish_capture();
        let totals = components::totals(&events);
        let flame = crate::trace_out::emit_pair(&out_dir, "primerlane", events)?;
        print!("{flame}");
        Some(totals)
    } else {
        None
    };

    let report = PrimerlaneReport {
        provenance: crate::report::provenance(Path::new(".")),
        relations: cfg.relations,
        facts: cfg.facts,
        seed: cfg.seed,
        phases,
        telemetry,
        components,
    };
    std::fs::write(out_dir.join("primerlane-report.json"), to_json(&report))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = to_markdown(&report);
    std::fs::write(out_dir.join("primerlane-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(0)
}
