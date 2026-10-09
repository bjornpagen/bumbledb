use std::fmt::Write as _;

use bumbledb::host::StoreReport;
use bumbledb::schema::render;
use bumbledb::{Db, Schema, Violations};

use crate::cli::CorpusArgs;
use crate::worlds::ledger::{Ledger, schema};

use super::corpus::gen_config;
use super::corpus_paths;

pub fn cmd_verify_store(corpus: &CorpusArgs) -> Result<i32, String> {
    let paths = corpus_paths(&corpus.dir, gen_config(corpus));
    if !paths.db.exists() {
        return Err(format!(
            "no store at {} — run first: bumbledb-bench gen --scale {} --seed {} --dir {}",
            paths.db.display(),
            corpus.scale.label(),
            corpus.seed,
            corpus.dir.display(),
        ));
    }
    let db = Db::open(&paths.db, Ledger, crate::harness::bench_work())
        .map_err(|e| format!("open db: {e:?}"))?;
    let report = db
        .verify_store(&crate::harness::bench_work())
        .map_err(|e| format!("verify store: {e:?}"))?;
    print!("{}", render_report(schema(), &report));
    Ok(i32::from(!report.is_coherent()))
}

/// Corruption cites no statement (a physical projection may serve several);
/// each violation names its statement through the statement renderer.
fn render_report(schema: &Schema, report: &StoreReport) -> String {
    let mut out = String::new();
    for corruption in &report.corruption {
        let _ = writeln!(out, "corruption: {corruption:?}");
    }
    for violation in report.violations.iter().flatten() {
        let _ = writeln!(
            out,
            "violation: {violation:?} — statement: {}",
            render::render(schema, violation.statement_id(schema))
        );
    }
    if report.is_coherent() {
        let _ = writeln!(out, "verify-store OK: namespaces coherent, judgments hold");
    } else {
        let findings =
            report.corruption.len() + report.violations.as_ref().map_or(0, Violations::len);
        let _ = writeln!(out, "verify-store FAILED: {findings} finding(s)");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::TempDir;
    use crate::worlds::ledger::{Account, AccountId, CurrencyId, HolderId};

    #[test]
    fn findings_render_through_the_statement_renderer() {
        let schema = schema();
        let dir = TempDir::new("verify-store-render");
        let db = Db::create(dir.path(), Ledger, crate::harness::bench_work())
            .expect("create")
            .unwrap();
        let orphan = Account {
            id: AccountId(1),
            holder: HolderId(7),
            currency: CurrencyId(0),
        };
        let outcome = db.write(crate::harness::bench_work(), |tx| tx.insert([&orphan]));
        let Ok(bumbledb::WriteOutcome::Rejected(violations)) = outcome else {
            panic!("an account without its holder is rejected: {outcome:?}");
        };
        let statement = violations
            .get(0)
            .expect("one violation")
            .statement_id(schema);

        let report = StoreReport {
            corruption: Box::new([]),
            violations: Some(violations),
        };
        let rendered = render_report(schema, &report);
        assert!(
            rendered.contains(&render::render(schema, statement)),
            "{rendered}"
        );
        assert!(
            rendered.contains("verify-store FAILED: 1 finding(s)"),
            "{rendered}"
        );

        let clean = db
            .verify_store(&crate::harness::bench_work())
            .expect("verify_store");
        let rendered = render_report(schema, &clean);
        assert!(rendered.contains("verify-store OK"), "{rendered}");
    }
}
