use bumbledb::ResultBuffer;

use crate::harness::{self, Modes, Rotation};
use crate::schema::schema;
use crate::translate::translate;
use crate::{clockproxy, families, report, sqlite_run, trace_out};

use super::BenchRun;

fn exec_digest(stats: &bumbledb::ExecutionStats) -> report::ExecDigest {
    use std::fmt::Write as _;
    let mut worst = 1.0_f64;
    let mut covers = String::new();
    for (index, node) in stats.nodes.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let (estimate, actual) = (node.estimate.max(1) as f64, node.actual.max(1) as f64);
        worst = worst.max((estimate / actual).max(actual / estimate));
        if index > 0 {
            covers.push(' ');
        }
        let _ = write!(covers, "n{index}:");
        for (position, cover) in node.covers.iter().enumerate() {
            if position > 0 {
                covers.push('/');
            }
            let _ = write!(
                covers,
                "s{}x{}",
                cover.subatom,
                cover.chosen_exact + cover.chosen_estimate
            );
        }
    }
    report::ExecDigest {
        worst_estimate_factor: worst,
        covers,
        emits: stats.emits,
    }
}

#[cfg(feature = "obs")]
fn alloc_report(
    snapshot: Option<bumbledb::alloc_counter::AllocSnapshot>,
) -> Option<report::AllocReport> {
    snapshot.map(|s| report::AllocReport {
        allocs: s.allocs,
        deallocs: s.deallocs,
        alloc_bytes: s.alloc_bytes,
        dealloc_bytes: s.dealloc_bytes,
    })
}

/// The stamp merge for a family whose ours/theirs blocks were guarded
/// as one bracket pair each: the reported bracket is the WORST of the
/// two (contamination of either engine's block dirties the ratio).
fn merge_stamps(ours: clockproxy::GhzStamp, theirs: clockproxy::GhzStamp) -> report::GhzReport {
    report::GhzReport {
        pre: ours.pre.min(theirs.pre),
        post: ours.post.min(theirs.post),
        retried: ours.retried || theirs.retried,
        contaminated: ours.contaminated() || theirs.contaminated(),
    }
}

impl BenchRun<'_> {
    /// One read family on both engines.
    pub(super) fn read_family(
        &mut self,
        family: &families::Family,
    ) -> Result<report::ReadFamilyReport, String> {
        eprintln!("bench: read family {}", family.name);
        let query = (family.query)();
        let mut prepared = self
            .db
            .prepare(&query)
            .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
        let sets = (family.params)(&self.cfg);
        let types: Vec<bumbledb::schema::ValueType> = prepared.column_types().cloned().collect();

        let mut rotation = Rotation::new(sets.clone());
        let mut buffer = ResultBuffer::new();
        let db = self.db;
        let mut run_ours = move |prepared: &mut bumbledb::PreparedQuery<'_>| {
            let params = rotation.next_set().to_vec();
            db.read(|snap| snap.execute(prepared, &params, &mut buffer))
                .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        };
        let modes = Modes {
            alloc_window: self.alloc,
            trace: false,
            proxy_per_rep: self.proxy_per_rep,
        };
        let proto = self.proto;
        // Process-start warm discipline (docs/silicon2/00): the first
        // family absorbs the start-band beyond its own warmups.
        if !self.first_family_warmed {
            for _ in 0..32 {
                run_ours(&mut prepared)?;
            }
            self.first_family_warmed = true;
        }
        let (ours, ghz_ours) = clockproxy::guarded(|| {
            harness::measure_batched(proto, modes, 1, || run_ours(&mut prepared))
        })?;
        // The quantum guard: a gated p50 below 12 timer ticks would be
        // quantization, not measurement — batch executes and divide.
        let batch = if ours.stats.p50 < harness::QUANTUM_FLOOR_NS {
            16
        } else {
            1
        };
        let (ours, ghz_ours) = if batch > 1 {
            eprintln!(
                "bench: {} p50 under the {} ns quantum floor — re-measuring at batch {batch}",
                family.name,
                harness::QUANTUM_FLOOR_NS
            );
            clockproxy::guarded(|| {
                harness::measure_batched(proto, modes, batch, || run_ours(&mut prepared))
            })?
        } else {
            (ours, ghz_ours)
        };
        if self.trace {
            let (_, events) = harness::traced_sample(&mut || run_ours(&mut prepared))?;
            let (engine, harness_events) = trace_out::split_harness(events);
            trace_out::write_trace_file(
                &self.trace_dir,
                &format!("{}.warm", family.name),
                &engine,
                &harness_events,
            )
            .map_err(|e| format!("trace: {e}"))?;
            let mut table = trace_out::FlameSummary::compute(&engine).render_top(10);
            if let Some(phases) = trace_out::render_phase_table(&engine) {
                table.push('\n');
                table.push_str(&phases);
            }
            self.flames.push(report::FlameEmbed {
                name: family.name.to_owned(),
                table,
            });
        }
        let (_, stats) = self
            .db
            .read(|snap| snap.profile(&mut prepared, &sets[0]))
            .map_err(|e| format!("profile: {e:?}"))?;

        let translated = translate(&query, schema()).map_err(|e| format!("translate: {e}"))?;
        let mut sqlite_family = sqlite_run::PreparedFamily::new(self.conn, &translated, types)?;
        let mut rotation = Rotation::new(sets);
        let (theirs, ghz_theirs) = clockproxy::guarded(|| {
            harness::measure_batched(proto, Modes::default(), batch, || {
                sqlite_run::sample(&mut sqlite_family, rotation.next_set())
            })
        })?;

        #[allow(clippy::cast_precision_loss)]
        let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
        #[cfg(feature = "obs")]
        let alloc = alloc_report(ours.alloc);
        #[cfg(not(feature = "obs"))]
        let alloc = None;
        Ok(report::ReadFamilyReport {
            name: family.name.to_owned(),
            verdict: report::verdict(family.kind, ours.stats.p50, theirs.stats.p50),
            p99_within_budget: report::within_budget(ours.stats.p99),
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50,
            alloc,
            exec: Some(exec_digest(&stats)),
            ghz: Some(merge_stamps(ghz_ours, ghz_theirs)),
            p50_norm: ours.p50_norm,
        })
    }
}
