//! The `R` pass: one cursor over `R | statement | key_bytes | source_rel |
//! source_row` — the heart of the sweep, the one namespace the commit path
//! deletes without verification (`docs/architecture/50-storage.md`
//! § R-delete verification) while target-side judgment trusts its prefixes
//! as the survivor authority. Every edge must resolve to a live source
//! fact that still satisfies φ (the commit path's own satisfaction helper)
//! and re-derives the same permuted key bytes.

use crate::error::Result;
use crate::schema::{Resolved, StatementDescriptor};
use crate::storage::commit::judgment;
use crate::storage::keys;

use super::{namespace, StoreFinding, Sweep};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let schema = s.schema;
    let mut derived = Vec::new();
    for entry in namespace(s.data, txn, keys::NS_REVERSE)? {
        let (key, _) = entry?;
        let Some((sid, key_bytes, source_rel, source_row)) = keys::parse_reverse_key(key) else {
            s.malformed(key, "R key shape");
            continue;
        };
        // The statement id must name a containment whose source is the
        // embedded relation — anything else is not an R key the schema
        // could ever have written.
        let Some(statement) = schema.statements().get(usize::from(sid.0)) else {
            s.malformed(key, "R key statement");
            continue;
        };
        let StatementDescriptor::Containment { source, .. } = &statement.descriptor else {
            s.malformed(key, "R key statement");
            continue;
        };
        if source.relation != source_rel {
            s.malformed(key, "R key source relation");
            continue;
        }
        let Resolved::Containment {
            key_permutation, ..
        } = &statement.resolved
        else {
            unreachable!("validated schema: Containment resolves as Containment")
        };
        let layout = schema.relation(source_rel).layout();

        // R→F: the source must be live, still inside φ, and re-derive
        // these key bytes. A wrong-width fact was already convicted by
        // the F pass and cannot be sliced, so it passes here.
        let backs = match s.fact(source_rel, source_row)? {
            None => false,
            Some(fact) if fact.len() != layout.fact_width() => true,
            Some(fact) => {
                judgment::satisfies(&s.selections.containment(sid).source, layout, fact) && {
                    keys::permuted_guard_bytes(
                        layout,
                        &source.projection,
                        key_permutation,
                        fact,
                        &mut derived,
                    );
                    derived == key_bytes
                }
            }
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
