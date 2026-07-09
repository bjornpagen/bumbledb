//! The `F` pass: one cursor over `F | relation | row_id`. Per live fact —
//! its `M` entry must point back, every key statement's guard must hold
//! its row id in `U`, and every outgoing containment whose φ it satisfies
//! must have its `R` edge. The same walk feeds the per-relation tallies
//! (row count, max row id — no second scan) and collects the referenced
//! intern ids, checking each against the dictionary next-id counter.

use std::ops::Bound;

use crate::encoding::{fact_hash, field_bytes, TypeDesc};
use crate::error::Result;
use crate::schema::{RelationId, Resolved, StatementDescriptor};
use crate::storage::commit::judgment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

use super::{StoreFinding, Sweep};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let schema = s.schema;
    let (lo, hi) = ([keys::NS_FACT], [keys::NS_FACT + 1]);
    let bounds: (Bound<&[u8]>, Bound<&[u8]>) = (Bound::Included(&lo[..]), Bound::Excluded(&hi[..]));
    let mut scratch: KeyBuf = [0; MAX_KEY];
    let mut guard = Vec::new();
    for entry in s.data.range(txn.raw(), &bounds)? {
        let (key, fact) = entry?;
        if key.len() != keys::FACT_KEY_LEN {
            s.malformed(key, "F key length");
            continue;
        }
        let rel = RelationId(u32::from_be_bytes(
            key[1..5].try_into().expect("fixed-width slice"),
        ));
        let row_id = u64::from_be_bytes(key[5..].try_into().expect("fixed-width slice"));
        let Some(relation) = schema.relation_checked(rel) else {
            s.malformed(key, "F key relation");
            continue;
        };
        {
            let tally = s.tallies.entry(rel).or_default();
            tally.rows += 1;
            tally.max_row_id = tally.max_row_id.max(row_id);
        }
        let layout = relation.layout();
        if fact.len() != layout.fact_width() {
            s.malformed(key, "F fact width");
            continue;
        }

        // Referenced intern ids, bounded by the dictionary next-id.
        for idx in 0..layout.field_count() {
            if matches!(layout.field_type(idx), TypeDesc::String | TypeDesc::Bytes) {
                let id = u64::from_be_bytes(
                    field_bytes(fact, layout, idx)
                        .try_into()
                        .expect("interned fields are 8 bytes"),
                );
                s.referenced_interns.insert(id);
                if id >= s.dict_next_id {
                    s.push(StoreFinding::InternBeyondNextId {
                        relation: rel,
                        row_id,
                        intern_id: id,
                        next_id: s.dict_next_id,
                    });
                }
            }
        }

        // F→M: the membership entry must exist and point back.
        let m_len = keys::membership_key(&mut scratch, rel, &fact_hash(fact));
        let points_back = s
            .data
            .get(txn.raw(), &scratch[..m_len])?
            .is_some_and(|v| v == row_id.to_le_bytes().as_slice());
        if !points_back {
            s.push(StoreFinding::FactWithoutMembership {
                relation: rel,
                row_id,
                membership_key: scratch[..m_len].into(),
            });
        }

        // F→U: every key statement's guard must hold this row id
        // (guards re-derived by slicing, exactly as the commit path).
        for &sid in relation.keys() {
            keys::guard_bytes(
                layout,
                schema.statement(sid).key_projection(),
                fact,
                &mut guard,
            );
            let u_len = keys::guard_key(&mut scratch, rel, sid, &guard);
            let held = s
                .data
                .get(txn.raw(), &scratch[..u_len])?
                .is_some_and(|v| v == row_id.to_le_bytes().as_slice());
            if !held {
                s.push(StoreFinding::FactWithoutGuard {
                    relation: rel,
                    statement: sid,
                    row_id,
                    guard_key: scratch[..u_len].into(),
                });
            }
        }

        check_reverse_edges(s, rel, row_id, fact, &mut scratch, &mut guard)?;
    }
    Ok(())
}

/// F→R: one reverse edge per outgoing containment whose source selection
/// the fact satisfies — the derivation and the φ check are the commit
/// path's own.
fn check_reverse_edges(
    s: &mut Sweep<'_, '_>,
    rel: RelationId,
    row_id: u64,
    fact: &[u8],
    scratch: &mut KeyBuf,
    guard: &mut Vec<u8>,
) -> Result<()> {
    let txn = s.txn;
    let schema = s.schema;
    let relation = schema.relation(rel);
    let layout = relation.layout();
    for &sid in relation.outgoing() {
        let statement = schema.statement(sid);
        let StatementDescriptor::Containment { source, .. } = &statement.descriptor else {
            unreachable!("validated schema: outgoing ids name Containment statements")
        };
        let Resolved::Containment {
            key_permutation, ..
        } = &statement.resolved
        else {
            unreachable!("validated schema: Containment resolves as Containment")
        };
        if !judgment::satisfies(&s.selections.containment(sid).source, layout, fact) {
            continue;
        }
        keys::permuted_guard_bytes(layout, &source.projection, key_permutation, fact, guard);
        let r_len = keys::reverse_key(scratch, sid, guard, rel, row_id);
        if s.data.get(txn.raw(), &scratch[..r_len])?.is_none() {
            s.push(StoreFinding::FactWithoutReverseEdge {
                statement: sid,
                relation: rel,
                row_id,
                reverse_key: scratch[..r_len].into(),
            });
        }
    }
    Ok(())
}
