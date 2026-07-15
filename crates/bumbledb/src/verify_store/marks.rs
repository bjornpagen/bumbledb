//! The marks pass: the global re-verification the extension forms' own
//! namespaces cannot ride. **Order marks** are walked whole — every group
//! of every order statement on a writable subject, through the commit
//! path's own ordered group walk (`judgment::Checker::check_order_walk`,
//! one definition, never a sweeper copy) — because the incremental
//! checker consults touched groups only, and the class the sweeper owns
//! is exactly the untouched ones (`lean/Bumbledb/Countermodels.lean:
//! incremental_verdict_needs_holds`). Closed subjects are plain-only
//! (the ranked form is gate-refused) and were decided at validate. **Closed-parent windows** re-check
//! per sealed member axiom: those parents have no `F` rows to ride the
//! fact scan, so their roster walks here (the domain-quantification move,
//! `docs/architecture/30-dependencies.md`). Ordinary-parent windows ride
//! the `F` pass (`facts.rs`), one scan shared across every statement.

use crate::encoding::encode_u64;
use crate::error::{Error, Result, Violation};
use crate::schema::Enforcement;
use crate::storage::commit::judgment;

use super::{StoreFinding, Sweep};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let schema = s.schema;
    let mut checker = judgment::Checker::new(s.txn.raw(), s.data, schema);

    // Every order statement on a WRITABLE subject, every group: the
    // ordinal discipline and the ranked monotonicity, one whole-namespace
    // walk per statement. A closed subject's mark is plain-only — the
    // ranked form is gate-refused (`SchemaError::RankedOrderClosedSubject`)
    // — so it was decided at validate against the sealed extension (the
    // axioms ARE its final state) and has no stored edges to walk: the
    // skip is justified by the refusal, not by hope.
    for statement in schema.orders() {
        if schema.relation(statement.relation).is_closed() {
            continue;
        }
        let mut violations: Vec<Violation> = Vec::new();
        match checker.check_order_walk(statement, None, &mut violations) {
            // A corruption inside the walk (a stray edge naming no live
            // row) is a namespace desync the R pass convicts on its own;
            // whatever the walk found before it still reports.
            Ok(_) | Err(Error::Corruption(_)) => {}
            Err(other) => return Err(other),
        }
        for violation in violations {
            let Violation::Order {
                statement,
                defect,
                fact,
            } = violation
            else {
                unreachable!("the order walk cites order statements only");
            };
            s.push(StoreFinding::OrderViolation {
                statement,
                defect,
                fact,
            });
        }
    }

    // Every closed-parent window, every ψ-selected axiom: the axiom's id
    // encoding is the parent tuple, and the commit path's own window
    // check counts its child group.
    for (index, statement) in schema.windows().iter().enumerate() {
        let Enforcement::Closed { .. } = &statement.enforcement else {
            continue;
        };
        let window_id =
            crate::schema::WindowId(u16::try_from(index).expect("statement count fits u16"));
        let rows = schema
            .relation(statement.target.relation)
            .extension()
            .expect("the Closed enforcement arm resolves only against a closed target");
        for row_index in 0..rows.len() {
            let parent = encode_u64(u64::try_from(row_index).expect("row index fits u64"));
            // Fetched per row so the borrow of `s.selections` ends before
            // the finding push.
            let checks = s.selections.window(window_id);
            match checker.check_window(statement, checks, &parent) {
                Err(Error::CommitRejected { violations }) => {
                    for violation in violations {
                        let Violation::Cardinality {
                            statement,
                            fact,
                            count,
                        } = violation
                        else {
                            unreachable!("the window check cites cardinality statements only");
                        };
                        s.push(StoreFinding::WindowViolation {
                            statement,
                            fact,
                            count,
                        });
                    }
                }
                // A corruption inside the probe is a namespace desync
                // another pass convicts on its own.
                Ok(()) | Err(Error::Corruption(_)) => {}
                Err(other) => return Err(other),
            }
        }
    }
    Ok(())
}
