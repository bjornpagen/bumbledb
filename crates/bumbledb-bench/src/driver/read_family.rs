use bumbledb::{Answers, Db, Query};

use crate::calendar;
use crate::families::{Draw, Kind, param_args, set_bindings};
use crate::harness::{self, Rotation};
use crate::schema::schema;
use crate::translate::{Translated, translate};
use crate::{families, report, sqlite_run};

use super::BenchRun;

pub(super) struct ReadSpec<'a> {
    pub name: &'a str,
    pub kind: Kind,
    pub query: Query,
    pub sets: Vec<Draw>,

    pub sql_for: &'a dyn Fn(&Query, &Draw) -> Result<Translated, String>,
}

impl BenchRun<'_> {
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

    /// Time the registered parameter stream on both engines.
    fn measure_read<S>(
        &mut self,
        db: &Db<S>,
        conn: &rusqlite::Connection,
        spec: &ReadSpec<'_>,
    ) -> Result<report::ReadFamilyReport, String> {
        eprintln!("bench: read family {}", spec.name);
        let mut prepared = db
            .prepare(&spec.query, crate::harness::bench_work())
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
            db.read(crate::harness::bench_work(), |snap| {
                snap.execute(prepared, &args, &mut buffer)
            })
            .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        };
        let proto = self.proto;

        if !self.first_family_warmed {
            for _ in 0..32 {
                run_ours(&mut prepared)?;
            }
            self.first_family_warmed = true;
        }
        let initial_batch = self.read_batch.map_or(1, std::num::NonZeroU32::get);
        let ours = harness::measure_batched(proto, initial_batch, || run_ours(&mut prepared))?;

        let batch = if self.read_batch.is_none() && ours.stats.p50 < harness::QUANTUM_FLOOR_NS {
            harness::MAX_READ_BATCH
        } else {
            initial_batch
        };
        let ours = if self.read_batch.is_none() && batch > 1 {
            eprintln!(
                "bench: {} p50 under the {} ns quantum floor — re-measuring at batch {batch}",
                spec.name,
                harness::QUANTUM_FLOOR_NS
            );
            harness::measure_batched(proto, batch, || run_ours(&mut prepared))?
        } else {
            ours
        };

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
        let theirs = harness::measure_batched(proto, batch, || {
            let index = rotation.next_index();
            sqlite_run::sample_args(&mut sqlite_families[index], &sets[index])
        })?;

        let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
        Ok(report::ReadFamilyReport {
            name: spec.name.to_owned(),
            batch,
            verdict: report::verdict(spec.kind, ours.stats.p50, theirs.stats.p50),
            p99_within_budget: report::within_budget(ours.stats.p99),
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50,
        })
    }
}
