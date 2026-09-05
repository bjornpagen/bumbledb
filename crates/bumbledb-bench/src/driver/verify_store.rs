use std::fmt::Write as _;

use bumbledb::schema::render;
use bumbledb::{Db, Schema, StatementId, StoreFinding, StoreReport};

use crate::cli::CorpusArgs;
use crate::schema::{Ledger, schema};

use super::corpus::gen_config;
use super::corpus_paths;

/// # Errors
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
        .verify_store()
        .map_err(|e| format!("verify store: {e:?}"))?;
    print!("{}", render_report(schema(), &report));
    Ok(i32::from(!report.findings().is_empty()))
}

fn finding_statement(finding: &StoreFinding) -> Option<StatementId> {
    match finding {
        // The complete re-judgment's violation names its statement directly.
        StoreFinding::Judgment(violation) => Some(violation.statement),
        // Physical projections may serve several statements; do not invent
        // a statement citation for a projection-level corruption.
        StoreFinding::Corruption(_) => None,
    }
}

fn render_report(schema: &Schema, report: &StoreReport) -> String {
    let mut out = String::new();
    for finding in report.findings() {
        let _ = write!(out, "finding: {finding:?}");
        if let Some(id) = finding_statement(finding) {
            let _ = write!(out, " — statement: {}", render::render(schema, id));
        }
        out.push('\n');
    }
    // The immortal-dictionary leak line is gone with the dictionary itself
    // (ENG-006): the successor store persists inline canonical text, so
    // there is no intern namespace left to leak.
    if report.findings().is_empty() {
        let _ = writeln!(out, "verify-store OK: namespaces coherent, judgments hold");
    } else {
        let _ = writeln!(
            out,
            "verify-store FAILED: {} finding(s)",
            report.findings().len()
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::StoreVerdict;

    #[test]
    fn findings_render_through_the_statement_renderer() {
        let schema = schema();

        let containment = (0..schema.keys().len() + schema.containments().len())
            .map(|id| StatementId(u16::try_from(id).expect("small fixture")))
            .find(|&id| render::render(schema, id).contains("<="))
            .expect("the ledger schema declares containments");
        let report = StoreReport {
            verdict: StoreVerdict::Desynced {
                findings: vec![StoreFinding::Judgment(
                    bumbledb::schema::judge::JudgedViolation {
                        statement: containment,
                        kind: bumbledb::schema::StatementKind::Containment,
                        direction: None,
                        measure: None,
                        examples: Box::new([]),
                        examples_truncated: false,
                    },
                )]
                .into(),
            },
        };
        let rendered = render_report(schema, &report);
        assert!(
            rendered.contains(&render::render(schema, containment)),
            "{rendered}"
        );
        assert!(
            rendered.contains("verify-store FAILED: 1 finding(s)"),
            "{rendered}"
        );

        let clean = StoreReport {
            verdict: StoreVerdict::Coherent,
        };
        let rendered = render_report(schema, &clean);
        assert!(rendered.contains("verify-store OK"), "{rendered}");
        assert!(
            !rendered.contains("intern"),
            "the dictionary leak line is gone with the dictionary (ENG-006): {rendered}"
        );
    }
}
