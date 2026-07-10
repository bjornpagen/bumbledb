//! `verify-store`: the CLI wrapper around the offline sweeper,
//! [`Db::verify_store`] (`docs/architecture/60-validation.md` — the third
//! validation leg: the oracles judge semantics, the sweeper judges the
//! store). Opens the digest-keyed store, sweeps once, renders every
//! finding — through the statement renderer where a statement id is
//! present — and exits nonzero iff findings are non-empty.

use std::fmt::Write as _;

use bumbledb::schema::render;
use bumbledb::{Db, Schema, StatementId, StoreFinding, StoreReport};

use crate::cli::CorpusArgs;
use crate::schema::{schema, Ledger};

use super::corpus::gen_config;
use super::corpus_paths;

/// `verify-store`. Returns the process exit code: 0 for a coherent store,
/// 1 when the report carries findings.
///
/// # Errors
///
/// A missing corpus (the message names `gen`) or an environmental
/// failure (open, LMDB) as a message — store findings are the report and
/// an exit code, never an error.
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
    Ok(i32::from(!report.findings.is_empty()))
}

/// The finding's statement id, when its variant carries one — the hook
/// for rendering the violated statement back in the `schema!` notation.
fn finding_statement(finding: &StoreFinding) -> Option<StatementId> {
    match finding {
        StoreFinding::FactWithoutGuard { statement, .. }
        | StoreFinding::GuardWithoutFact { statement, .. }
        | StoreFinding::PointwiseOverlap { statement, .. }
        | StoreFinding::FactWithoutReverseEdge { statement, .. }
        | StoreFinding::ReverseEdgeWithoutFact { statement, .. }
        | StoreFinding::JudgmentViolation { statement, .. } => Some(*statement),
        StoreFinding::FactWithoutMembership { .. }
        | StoreFinding::MembershipWithoutFact { .. }
        | StoreFinding::RowCountDesync { .. }
        | StoreFinding::RowIdHighWaterLow { .. }
        | StoreFinding::InternBeyondNextId { .. }
        | StoreFinding::Malformed { .. } => None,
    }
}

/// One line per finding (statement-carrying findings cite the statement
/// in the macro notation), the dictionary statistic, and the verdict.
fn render_report(schema: &Schema, report: &StoreReport) -> String {
    let mut out = String::new();
    for finding in &report.findings {
        let _ = write!(out, "finding: {finding:?}");
        if let Some(id) = finding_statement(finding) {
            let _ = write!(out, " — statement: {}", render::render(schema, id));
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "dangling intern ids (the accepted leak): {}",
        report.dangling_intern_ids
    );
    if report.findings.is_empty() {
        let _ = writeln!(out, "verify-store OK: namespaces coherent, judgments hold");
    } else {
        let _ = writeln!(
            out,
            "verify-store FAILED: {} finding(s)",
            report.findings.len()
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statement renderer engages exactly on the statement-carrying
    /// variants, in the ledger schema's own notation.
    #[test]
    fn findings_render_through_the_statement_renderer() {
        let schema = schema();
        // `Account(holder) <= Holder(id)` is the ledger's first declared
        // containment; its materialized id follows the serial auto-FDs.
        let containment = (0..schema.statements().len())
            .map(|id| StatementId(u16::try_from(id).expect("small fixture")))
            .find(|&id| render::render(schema, id).contains("<="))
            .expect("the ledger schema declares containments");
        let report = StoreReport {
            findings: vec![StoreFinding::JudgmentViolation {
                statement: containment,
                direction: bumbledb::Direction::TargetRequired,
                fact: Box::new([0; 8]),
            }],
            dangling_intern_ids: 0,
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
            findings: Vec::new(),
            dangling_intern_ids: 3,
        };
        let rendered = render_report(schema, &clean);
        assert!(rendered.contains("verify-store OK"), "{rendered}");
        assert!(
            rendered.contains("dangling intern ids (the accepted leak): 3"),
            "{rendered}"
        );
    }
}
