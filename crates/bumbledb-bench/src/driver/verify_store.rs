use std::fmt::Write as _;

use bumbledb::error::CorruptionError;
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
    let db = Db::open(&paths.db, Ledger).map_err(|e| format!("open db: {e:?}"))?;
    let report = db
        .verify_store()
        .map_err(|e| format!("verify store: {e:?}"))?;
    print!("{}", render_report(schema(), &report));
    Ok(i32::from(!report.findings().is_empty()))
}

fn finding_statement(finding: &StoreFinding, schema: &Schema) -> Option<StatementId> {
    match finding {
        StoreFinding::Judgment(violation) => Some(violation.statement_id(schema)),
        StoreFinding::Corruption(
            CorruptionError::FactWithoutDeterminant { statement, .. }
            | CorruptionError::DeterminantWithoutFact { statement, .. }
            | CorruptionError::PointwiseOverlap { statement, .. }
            | CorruptionError::FactWithoutReverseEdge { statement, .. }
            | CorruptionError::ReverseEdgeWithoutFact { statement, .. }
            | CorruptionError::ReverseEdgeWeightDesync { statement, .. }
            | CorruptionError::FreshRowDeterminantEntry { statement, .. },
        ) => Some(*statement),
        StoreFinding::Corruption(_) => None,
    }
}

fn render_report(schema: &Schema, report: &StoreReport) -> String {
    let mut out = String::new();
    for finding in report.findings() {
        let _ = write!(out, "finding: {finding:?}");
        if let Some(id) = finding_statement(finding, schema) {
            let _ = write!(out, " — statement: {}", render::render(schema, id));
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "dangling intern ids (the accepted leak): {}",
        report.dangling_intern_ids()
    );
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
    use bumbledb::{RelationId, StoreVerdict};

    #[test]
    fn findings_render_through_the_statement_renderer() {
        let schema = schema();

        let containment = (0..schema.keys().len() + schema.containments().len())
            .map(|id| StatementId(u16::try_from(id).expect("small fixture")))
            .find(|&id| render::render(schema, id).contains("<="))
            .expect("the ledger schema declares containments");
        let report = StoreReport {
            verdict: StoreVerdict::Desynced {
                findings: vec![StoreFinding::Corruption(
                    CorruptionError::FactWithoutDeterminant {
                        relation: RelationId(0),
                        statement: containment,
                        row_id: 0,
                        determinant_key: Box::new([]),
                    },
                )]
                .into(),
                dangling_intern_ids: 0,
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
            verdict: StoreVerdict::Coherent {
                dangling_intern_ids: 3,
            },
        };
        let rendered = render_report(schema, &clean);
        assert!(rendered.contains("verify-store OK"), "{rendered}");
        assert!(
            rendered.contains("dangling intern ids (the accepted leak): 3"),
            "{rendered}"
        );
    }
}
