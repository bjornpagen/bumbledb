//! The `R` pass: one cursor over `R | statement | key_bytes | source_rel |
//! source_row` — the heart of the sweep, the one namespace the commit path
//! deletes without verification (`docs/architecture/50-storage.md`
//! § R-delete verification) while target-side judgment trusts its prefixes
//! as the survivor authority. Every edge must resolve to a live source
//! fact that re-derives the same key bytes — a containment or window edge
//! additionally still inside its φ (the commit path's own satisfaction
//! helper); an order edge carries no σ, its key bytes are the grouping
//! projection with the position tail.

use crate::error::Result;
use crate::schema::{Enforcement, StatementView};
use crate::storage::commit::judgment;
use crate::storage::keys;

use super::{StoreFinding, Sweep, namespace};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let schema = s.schema;
    let mut derived = keys::DeterminantImage::scratch();
    for entry in namespace(s.data, txn, keys::NS_REVERSE)? {
        let (key, _) = entry?;
        let Some((sid, key_bytes, source_rel, source_row)) = keys::parse_reverse_key(key) else {
            s.malformed(key, "R key shape");
            continue;
        };
        // The statement id must name a containment, window, or order
        // statement whose source is the embedded relation — anything else
        // is not an R key the schema could ever have written.
        let (expected_relation, closed_target) = match schema.statement_checked(sid) {
            Some(StatementView::Containment(_, statement)) => (
                statement.source.relation,
                matches!(statement.enforcement, Enforcement::Closed { .. })
                    .then_some(statement.target.relation),
            ),
            Some(StatementView::Cardinality(_, statement)) => (statement.source.relation, None),
            Some(StatementView::Order(_, statement)) => (statement.relation, None),
            _ => {
                s.malformed(key, "R key statement");
                continue;
            }
        };
        if expected_relation != source_rel {
            s.malformed(key, "R key source relation");
            continue;
        }
        // A closed-TARGET containment never emits `R` traffic — its
        // target side is vacuous by construction (axioms don't delete),
        // so a stored edge's very existence is the finding
        // (`docs/architecture/30-dependencies.md`, the shape criterion).
        // (A closed-target WINDOW does store edges: they are its child
        // count's index.)
        if let Some(target) = closed_target {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: target,
                key: key.into(),
            });
            continue;
        }
        // Closed sources never commit (writes refused), so an R edge
        // naming one is corruption — the F pass's exemption, mirrored.
        if schema.relation(source_rel).is_closed() {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: source_rel,
                key: key.into(),
            });
            continue;
        }
        let layout = schema.relation(source_rel).layout();

        // R→F: the source must be live, re-derive these key bytes, and —
        // for the σ-gated statement kinds — still sit inside φ. A
        // wrong-width fact was already convicted by the F pass and cannot
        // be sliced, so it passes here.
        let backs = match s.fact(source_rel, source_row)? {
            None => false,
            Some(fact) if fact.len() != layout.fact_width() => true,
            Some(fact) => match schema.statement_checked(sid) {
                Some(StatementView::Containment(containment_id, statement)) => {
                    let key_permutation = match &statement.enforcement {
                        Enforcement::ScalarProbe {
                            key_permutation, ..
                        }
                        | Enforcement::IntervalCoverage {
                            key_permutation, ..
                        } => key_permutation,
                        Enforcement::Closed { .. } => {
                            unreachable!("closed-target edges convicted above")
                        }
                    };
                    judgment::satisfies(
                        &s.selections.containment(containment_id).source,
                        layout,
                        fact,
                    ) && {
                        keys::permuted_determinant_image(
                            layout,
                            &statement.source.projection,
                            key_permutation,
                            fact,
                            &mut derived,
                        );
                        derived.as_bytes() == key_bytes
                    }
                }
                Some(StatementView::Cardinality(window_id, statement)) => {
                    judgment::satisfies(&s.selections.window(window_id).source, layout, fact) && {
                        judgment::window_child_image(statement, layout, fact, &mut derived);
                        derived.as_bytes() == key_bytes
                    }
                }
                Some(StatementView::Order(_, statement)) => {
                    keys::determinant_image(layout, &statement.edge_projection, fact, &mut derived);
                    derived.as_bytes() == key_bytes
                }
                _ => unreachable!("the statement arm was classified above"),
            },
        };
        if !backs {
            s.push(StoreFinding::ReverseEdgeWithoutFact {
                statement: sid,
                reverse_key: key.into(),
            });
        }
    }
    Ok(())
}
