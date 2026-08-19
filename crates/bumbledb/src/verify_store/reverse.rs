//! The `R` pass: one cursor over `R | statement | key_bytes | source_rel |
//! source_row` — the heart of the sweep, the one namespace the commit path
//! deletes without verification (`docs/architecture/50-storage.md`
//! § R-delete verification) while target-side judgment trusts its prefixes
//! as the survivor authority. Every edge must resolve to a live source
//! fact that re-derives the same key bytes and still sits inside its φ
//! (the commit path's own satisfaction helper).

use crate::error::Result;
use crate::schema::{Enforcement, StatementView};
use crate::storage::catalog::CatalogRead;
use crate::storage::commit::judgment;
use crate::storage::keys;

use super::{StoreFinding, Sweep, for_namespace};

#[expect(
    clippy::too_many_lines,
    reason = "one cursor, one arm per statement form — the sweep is clearer kept together"
)]
pub(super) fn sweep<C: CatalogRead + Copy>(s: &mut Sweep<'_, C>) -> Result<()> {
    let schema = s.schema;
    let mut derived = keys::DeterminantImage::scratch();
    for_namespace(s.catalog, keys::Namespace::Reverse, |key, value| {
        let Some((sid, key_bytes, source_rel, source_row)) = keys::parse_reverse_key(key) else {
            s.malformed(key, "R key shape");
            return Ok(());
        };
        // The statement id must name a containment or capacity
        // statement whose source is the embedded relation — anything else
        // is not an R key the schema could ever have written.
        let (expected_relation, closed_target) = match schema.statement_checked(sid) {
            Some(StatementView::Containment(_, statement)) => (
                statement.source.relation,
                matches!(statement.enforcement, Enforcement::Closed { .. })
                    .then_some(statement.target.relation),
            ),
            Some(StatementView::Capacity(_, statement)) => (statement.source.relation, None),
            _ => {
                s.malformed(key, "R key statement");
                return Ok(());
            }
        };
        if expected_relation != source_rel {
            s.malformed(key, "R key source relation");
            return Ok(());
        }
        // A closed-TARGET containment never emits `R` traffic — its
        // target side is vacuous by construction (axioms don't delete),
        // so a stored edge's very existence is the finding
        // (`docs/architecture/30-dependencies.md`, the shape criterion).
        // (A closed-target CAPACITY statement does store edges: they are
        // its child-group measure's index.)
        if let Some(target) = closed_target {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: target,
                key: key.into(),
            });
            return Ok(());
        }
        // Closed sources never commit (writes refused), so an R edge
        // naming one is corruption — the F pass's exemption, mirrored.
        if schema.relation(source_rel).body().closed_rows().is_some() {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: source_rel,
                key: key.into(),
            });
            return Ok(());
        }
        let layout = schema.relation(source_rel).layout();
        let catalog = s.catalog;

        // R→F: the source must be live, re-derive these key bytes, and
        // still sit inside φ. A
        // wrong-width fact was already convicted by the F pass and cannot
        // be sliced, so it passes here.
        let backs = match catalog.fetch_fact(source_rel, source_row)? {
            None => false,
            Some(fact) if fact.as_ref().len() != layout.fact_width() => true,
            Some(fact) => match schema.statement_checked(sid) {
                Some(StatementView::Containment(containment_id, statement)) => {
                    let key_projection = match &statement.enforcement {
                        Enforcement::ScalarProbe { key_projection, .. }
                        | Enforcement::IntervalCoverage { key_projection, .. } => key_projection,
                        Enforcement::Closed { .. } => {
                            unreachable!("closed-target edges convicted above")
                        }
                    };
                    let bytes = fact.as_ref();
                    judgment::satisfies(
                        &s.selections.containment(containment_id).source,
                        layout,
                        bytes,
                    ) && {
                        keys::determinant_image(
                            layout.encoded(bytes),
                            key_projection,
                            &mut derived,
                        );
                        derived.as_bytes() == key_bytes
                    }
                }
                Some(StatementView::Capacity(capacity_id, statement)) => {
                    let bytes = fact.as_ref();
                    let inside = judgment::satisfies(
                        &s.selections.capacity(capacity_id).source,
                        layout,
                        bytes,
                    ) && {
                        judgment::capacity_child_image(statement, layout, bytes, &mut derived);
                        derived.as_bytes() == key_bytes
                    };
                    // R→F weight desync, the third conjunct: a live,
                    // φ-satisfying, key-backing edge must also carry the
                    // fact's weight-field encoding in its value slot
                    // (unit edges: empty). A ray in
                    // the weighed field is content — the malformed
                    // finding, never an error.
                    if inside {
                        match judgment::expected_slot_weight(statement, layout, bytes) {
                            Ok(expected) => {
                                let derived_word;
                                let expected_bytes: &[u8] = match expected {
                                    Some(weight) => {
                                        derived_word = weight.to_le_bytes();
                                        &derived_word
                                    }
                                    None => &[],
                                };
                                if value != expected_bytes {
                                    s.push(StoreFinding::ReverseEdgeWeightDesync {
                                        statement: sid,
                                        reverse_key: key.into(),
                                        stored: value.into(),
                                        derived: expected_bytes.into(),
                                    });
                                }
                            }
                            Err(_) => s.malformed(key, "R capacity weight of a ray"),
                        }
                    }
                    inside
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
        Ok(())
    })
}
