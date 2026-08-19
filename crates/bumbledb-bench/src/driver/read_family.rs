use bumbledb::{Answers, Db, Query, RuleStats, StatsBody};

use crate::calendar;
use crate::families::{Draw, Kind, has_sets, param_args, set_bindings};
use crate::harness::{self, Modes, Rotation};
use crate::schema::schema;
use crate::translate::{Translated, translate};
use crate::{clockproxy, families, report, sqlite_run, trace_out};

use super::BenchRun;

/// Pipeline-shaped execution digest. CQ: main-rule covers and absorbed.
/// Reach: interior emits + reach rounds — not `stats.rules` as a
/// universal table.
pub(crate) fn exec_digest(stats: &bumbledb::ExecutionStats) -> report::ExecDigest {
    match &stats.body {
        StatsBody::Cq { rules, .. } => cq_digest(stats.emits, rules),
        StatsBody::Reach { interiors, reach } => {
            use std::fmt::Write as _;
            let mut covers = String::new();
            for (index, interior) in interiors.iter().enumerate() {
                if index > 0 {
                    covers.push(' ');
                }
                let _ = write!(covers, "i{}:e{}", interior.interior, interior.emits);
            }
            if !covers.is_empty() {
                covers.push(' ');
            }
            let _ = write!(covers, "rec:r{}", reach.rounds.len());
            report::ExecDigest {
                worst_estimate_factor: 1.0,
                covers,
                emitted: stats.emits,
                absorbed: reach.rounds.iter().map(|round| round.absorbed).sum(),
            }
        }
    }
}

fn cq_digest(emits: u64, rules: &[RuleStats]) -> report::ExecDigest {
    use std::fmt::Write as _;
    let mut worst = 1.0_f64;
    let mut covers = String::new();
    for (index, node) in rules.iter().flat_map(RuleStats::nodes).enumerate() {
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
        emitted: emits,
        absorbed: rules.iter().map(RuleStats::absorbed).sum(),
    }
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
    /// protocol, frequency-checked, traced and profiled where the modes
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
            .signature()
            .columns
            .iter()
            .map(|column| *column.ty())
            .collect();

        let mut rotation = Rotation::new(sets.clone());
        let mut buffer = Answers::new();
        let mut run_ours = move |prepared: &mut bumbledb::PreparedQuery<S>| {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute(prepared, &args, &mut buffer))
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
        let (ours, ghz_ours) = clockproxy::frequency_checked(|| {
            harness::measure_batched(proto, modes, 1, || run_ours(&mut prepared))
        })?;
        // The quantum check: a gated p50 below 12 timer ticks would be
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
            clockproxy::frequency_checked(|| {
                harness::measure_batched(proto, modes, batch, || run_ours(&mut prepared))
            })?
        } else {
            (ours, ghz_ours)
        };
        if self.trace {
            let (_, events) = harness::traced_sample(&mut || run_ours(&mut prepared))?;
            let table =
                trace_out::emit_pair(&self.trace_dir, &format!("{}.warm", spec.name), events)?;
            self.flames.push(report::FlameEmbed {
                name: spec.name.to_owned(),
                table,
            });
        }
        // Estimate digest: set-bound families skip it — set selectivity
        // is an execution fact, not a plan static (the profile entry
        // itself binds sets since the R13 symmetry; the skip is the
        // digest's own semantics, and the frozen lanes keep their shape).
        let exec = if has_sets(&sets) {
            None
        } else {
            let (_, stats) = db
                .read(|snap| snap.profile(&mut prepared, &param_args(&sets[0])))
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
        let mut rotation = Rotation::new((0..sets.len()).collect::<Vec<_>>());
        let (theirs, ghz_theirs) = clockproxy::frequency_checked(|| {
            harness::measure_batched(proto, Modes::default(), batch, || {
                let index = rotation.next_index();
                sqlite_run::sample_args(&mut sqlite_families[index], &sets[index])
            })
        })?;

        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
        let alloc = ours.alloc.map(report::AllocReport::from);
        Ok(report::ReadFamilyReport {
            name: spec.name.to_owned(),
            verdict: report::verdict(spec.kind, ours.stats.p50, theirs.stats.p50),
            p99_within_budget: report::within_budget(ours.stats.p99),
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50,
            alloc,
            exec,
            ghz: Some(ghz_ours.merge(ghz_theirs).into()),
            p50_norm: ours.p50_norm,
        })
    }
}
