use super::{Case, Run, MAX_BUNDLES};

use bumbledb::schema::ValueType;
use bumbledb::ResultBuffer;

use crate::compare;
use crate::families::param_args;
use crate::naive::ParamValue;
use crate::translate::ParamSlot;

impl<S> Run<'_, S> {
    /// Executes one query × param draw on both stores and compares.
    /// Returns `false` once the bundle budget is exhausted (stop the
    /// run). Set params bind through the engine's `ParamArg` surface;
    /// the `SQLite` side receives the per-draw re-rendered SQL in
    /// `case.sql` with its element lists embedded as literals.
    ///
    /// Divergence-by-error is a mismatch, not a panic: if either side
    /// errors at prepare or execute where the other answers, that is an
    /// arbitration bundle with the erring side's `ERROR: <text>` in
    /// place of its rows — the audit confirmed a real divergence class
    /// here (`SQLite`'s transient SUM overflow vs the i128 accumulator).
    /// Both-sides-error is a bundle too: no case is *expected* to error
    /// today, so agreement-in-error would hide a tool defect as
    /// verification. Setup errors (store open, corpus load) stay panics.
    pub(super) fn check(
        &mut self,
        case: &Case<'_>,
        param_order: &[ParamSlot],
        params: &[ParamValue],
    ) -> bool {
        // Column types come from the engine's prepared query; without
        // them the oracle's rows cannot even be decoded, so a prepare
        // failure is an engine-side error and the oracle records "not
        // executed" rather than a fabricated second error.
        let (ours, theirs): (
            Result<Vec<compare::Row>, String>,
            Result<Vec<compare::Row>, String>,
        ) = match self.db.prepare(case.query) {
            Err(e) => (
                Err(format!("{e}")),
                Err("not executed: no column types without a prepared query".to_owned()),
            ),
            Ok(mut prepared) => {
                let types: Vec<ValueType> = prepared.column_types().cloned().collect();
                let mut buffer = ResultBuffer::new();
                let args = param_args(params);
                let ours = self
                    .db
                    .read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                    .map(|()| compare::from_buffer(&buffer, &types))
                    .map_err(|e| format!("{e}"));
                let theirs = self
                    .conn
                    .prepare_cached(case.sql)
                    .map_err(|e| e.to_string())
                    .and_then(|mut stmt| {
                        compare::from_sqlite(&mut stmt, param_order, params, &types)
                    });
                (ours, theirs)
            }
        };

        self.cases += 1;
        if self.cases.is_multiple_of(100) {
            eprintln!("verify: {}/{} cases", self.cases, self.total);
        }

        let verdict: Result<(), (String, String, String)> = match (ours, theirs) {
            (Ok(ours), Ok(theirs)) => compare::multisets(ours.clone(), theirs.clone())
                .map_err(|m| (m.to_string(), render_rows(&ours), render_rows(&theirs))),
            (Err(engine), Ok(theirs)) => Err((
                "divergence by error: the engine errored where the oracle answered".to_owned(),
                format!("ERROR: {engine}"),
                render_rows(&theirs),
            )),
            (Ok(ours), Err(oracle)) => Err((
                "divergence by error: the oracle errored where the engine answered".to_owned(),
                render_rows(&ours),
                format!("ERROR: {oracle}"),
            )),
            (Err(engine), Err(oracle)) => Err((
                "both sides errored — a tool defect must not look like verification".to_owned(),
                format!("ERROR: {engine}"),
                format!("ERROR: {oracle}"),
            )),
        };

        if let Err((mismatch, ours_text, theirs_text)) = verdict {
            let bundle = self
                .out_dir
                .join(format!("mismatch-{}", self.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            // The rule notation first (`ir::render` — total on malformed
            // queries, so a roster rejection still shows its query), the
            // raw IR after for arbitration by structure.
            std::fs::write(
                bundle.join("query.txt"),
                format!(
                    "{}\n{}\n\n{:#?}\n",
                    case.label,
                    self.db.render_query(case.query),
                    case.query
                ),
            )
            .expect("bundle");
            std::fs::write(bundle.join("query.sql"), case.sql).expect("bundle");
            std::fs::write(bundle.join("params.txt"), format!("{params:#?}\n")).expect("bundle");
            std::fs::write(bundle.join("mismatch.txt"), mismatch).expect("bundle");
            std::fs::write(bundle.join("ours.txt"), ours_text).expect("bundle");
            std::fs::write(bundle.join("theirs.txt"), theirs_text).expect("bundle");
            if let Some(golden) = case.golden_sql {
                std::fs::write(bundle.join("golden.sql"), golden).expect("bundle");
            }
            eprintln!("verify: MISMATCH {} -> {}", case.label, bundle.display());
            self.bundles.push(bundle);
        }
        self.bundles.len() < MAX_BUNDLES
    }
}

/// Renders a comparison multiset for a bundle artifact.
fn render_rows(rows: &[compare::Row]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "{} row(s)", rows.len());
    for row in rows {
        let _ = writeln!(out, "{row:?}");
    }
    out
}
