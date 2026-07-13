use bumbledb::{Db, Query, ResultBuffer};

use crate::calendar;
use crate::families::{Draw, Kind, has_sets, param_args, scalar_values, set_bindings};
use crate::harness::{self, Modes, Rotation};
use crate::schema::schema;
use crate::translate::{Translated, translate};
use crate::{clockproxy, families, report, sqlite_run, trace_out};

use super::BenchRun;

fn exec_digest(stats: &bumbledb::ExecutionStats) -> report::ExecDigest {
    use std::fmt::Write as _;
    let mut worst = 1.0_f64;
    let mut covers = String::new();
    for (index, node) in stats.rules.iter().flat_map(|r| &r.nodes).enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
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
        emitted: stats.emits,
        absorbed: stats.rules.iter().map(|rule| rule.absorbed).sum(),
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

/// One read family's identity, decoupled from its registry: the ledger
/// families and the calendar families measure through the same core
/// (one mechanism, two corpora), differing only in the store pair and
/// the SQL provider.
pub(super) struct ReadSpec<'a> {
    pub name: &'a str,
    pub kind: Kind,
    pub query: Query,
    pub sets: Vec<Draw>,
    /// Per-draw SQL — the translator for the ledger and paired calendar
    /// families, the hand-written coalesce for `free_busy`.
    pub sql_for: &'a dyn Fn(&Query, &Draw) -> Result<Translated, String>,
}

impl BenchRun<'_> {
    /// One read family on both engines — the ledger registry entry.
    pub(super) fn read_family(
        &mut self,
        family: &families::Family,
    ) -> Result<report::ReadFamilyReport, String> {
        let spec = ReadSpec {
            name: family.name,
            kind: family.kind,
            query: (family.query)(),
            sets: (family.params)(&self.cfg),
            sql_for: &|query, draw| translate(query, schema(), &set_bindings(draw)),
        };
        let db = self.db;
        let conn = self.conn;
        self.measure_read(db, conn, &spec)
    }

    /// One calendar family on both engines.
    pub(super) fn read_cal_family(
        &mut self,
        family: &calendar::families::CalFamily,
    ) -> Result<report::ReadFamilyReport, String> {
        let sql_for = |query: &Query, draw: &Draw| family.sql_for(query, draw);
        let spec = ReadSpec {
            name: family.name,
            kind: family.kind,
            query: (family.query)(),
            sets: (family.params)(&self.cfg),
            sql_for: &sql_for,
        };
        let db = self.cal_db;
        let conn = self.cal_conn;
        self.measure_read(db, conn, &spec)
    }

    /// The shared measurement core: warm both engines under the exact
    /// protocol, quantum-guarded, traced and profiled where the modes
    /// ask.
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // one family's full protocol, linear
    fn measure_read<S>(
        &mut self,
        db: &Db<S>,
        conn: &rusqlite::Connection,
        spec: &ReadSpec<'_>,
    ) -> Result<report::ReadFamilyReport, String> {
        eprintln!("bench: read family {}", spec.name);
        let mut prepared = db
            .prepare(&spec.query)
            .map_err(|e| format!("{}: prepare: {e:?}", spec.name))?;
        let sets = spec.sets.clone();
        let types: Vec<bumbledb::schema::ValueType> = prepared
            .predicate()
            .columns
            .iter()
            .map(|column| column.ty.clone())
            .collect();

        let mut rotation = Rotation::new(sets.clone());
        let mut buffer = ResultBuffer::new();
        let mut run_ours = move |prepared: &mut bumbledb::PreparedQuery<'_, S>| {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(prepared, &args, &mut buffer))
                .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        };
        let modes = Modes {
            alloc_window: self.alloc,
            trace: false,
            proxy_per_rep: self.proxy_per_rep,
        };
        let proto = self.proto;
        // Process-start warm discipline: the first
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
                spec.name,
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
                &format!("{}.warm", spec.name),
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
                name: spec.name.to_owned(),
                table,
            });
        }
        // Estimate digest: the profile path binds scalar params only —
        // set-bound families skip it (set selectivity is an execution
        // fact, not a plan static).
        let exec = if has_sets(&sets) {
            None
        } else {
            let (_, stats) = db
                .read(|snap| snap.profile(&mut prepared, &scalar_values(&sets[0])))
                .map_err(|e| format!("profile: {e:?}"))?;
            Some(exec_digest(&stats))
        };

        // One prepared statement per draw: scalar families re-render to
        // identical SQL; set-bound families genuinely differ per draw
        // (element lists as literals — prepared-statement parity is not
        // claimed for them, `60-validation.md`). Every statement is
        // prepared once and reused across the rotation's cycles.
        let mut sqlite_families = Vec::with_capacity(sets.len());
        for draw in &sets {
            let translated =
                (spec.sql_for)(&spec.query, draw).map_err(|e| format!("translate: {e}"))?;
            sqlite_families.push(sqlite_run::PreparedFamily::new(
                conn,
                &translated,
                types.clone(),
            )?);
        }
        let mut cursor = 0usize;
        let (theirs, ghz_theirs) = clockproxy::guarded(|| {
            harness::measure_batched(proto, Modes::default(), batch, || {
                let index = cursor;
                cursor = (cursor + 1) % sets.len();
                sqlite_run::sample_args(&mut sqlite_families[index], &sets[index])
            })
        })?;

        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
        #[cfg(feature = "obs")]
        let alloc = alloc_report(ours.alloc);
        #[cfg(not(feature = "obs"))]
        let alloc = None;
        Ok(report::ReadFamilyReport {
            name: spec.name.to_owned(),
            verdict: report::verdict(spec.kind, ours.stats.p50, theirs.stats.p50),
            p99_within_budget: report::within_budget(ours.stats.p99),
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50,
            alloc,
            exec,
            ghz: Some(super::ghz_report(ghz_ours.merge(ghz_theirs))),
            p50_norm: ours.p50_norm,
        })
    }
}
