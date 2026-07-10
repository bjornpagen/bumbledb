//! The `F` pass: one cursor over `F | relation | row_id`. Per live fact —
//! its `M` entry must point back, every key statement's guard must hold
//! its row id in `U`, and every outgoing containment whose φ it satisfies
//! must have its `R` edge **and its global judgment hold** (the target
//! tuple present or covered, through the commit path's own probes — one
//! `F` scan shared across every statement, never a scan per statement).
//! The same walk feeds the per-relation tallies (row count, max row id —
//! no second scan) and collects the referenced intern ids, checking each
//! against the dictionary next-id counter.

use crate::encoding::{fact_hash, field_bytes, TypeDesc};
use crate::error::{Direction, Error, Result};
use crate::schema::{RelationId, Resolved, StatementDescriptor};
use crate::storage::commit::judgment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

use super::{namespace, StoreFinding, Sweep};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let schema = s.schema;
    let mut scratch: KeyBuf = [0; MAX_KEY];
    let mut guard = Vec::new();
    let mut checker = judgment::Checker::new(txn.raw(), s.data, schema);
    for entry in namespace(s.data, txn, keys::NS_FACT)? {
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

        check_outgoing(s, &mut checker, rel, row_id, fact, &mut scratch, &mut guard)?;
    }
    Ok(())
}

/// F→R plus the global containment judgment, per outgoing containment
/// whose source selection the fact satisfies — the derivation and the φ
/// check are the commit path's own. The `R` edge must exist, and the
/// target tuple must be present (scalar probe) or covered (coverage walk)
/// in the committed state — the same [`judgment::Checker`] the commit
/// path consumes, over this sweep's read snapshot. A judgment miss is
/// [`StoreFinding::JudgmentViolation`], directed `TargetRequired`: every
/// committed source is a standing one.
#[allow(clippy::too_many_arguments)] // the F pass's per-fact scratch, threaded not owned
fn check_outgoing(
    s: &mut Sweep<'_, '_>,
    checker: &mut judgment::Checker<'_>,
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
        let StatementDescriptor::Containment { source, target } = &statement.descriptor else {
            unreachable!("validated schema: outgoing ids name Containment statements")
        };
        let Resolved::Containment {
            target_key,
            key_permutation,
            interval_position,
        } = &statement.resolved
        else {
            unreachable!("validated schema: Containment resolves as Containment")
        };
        let checks = s.selections.containment(sid);
        if !judgment::satisfies(&checks.source, layout, fact) {
            continue;
        }
        keys::permuted_guard_bytes(layout, &source.projection, key_permutation, fact, guard);
        let r_len = keys::reverse_key(scratch, sid, guard, rel, row_id);
        let missing_edge = s.data.get(txn.raw(), &scratch[..r_len])?.is_none();
        let probe = judgment::Probe {
            statement: sid,
            target_relation: target.relation,
            target_key: *target_key,
            target_check: &checks.target,
            key_bytes: guard,
            fact_bytes: fact,
            direction: Direction::TargetRequired,
        };
        let judged = if interval_position.is_some() {
            checker.check_coverage(&probe)
        } else {
            checker.check_scalar(&probe)
        };
        if missing_edge {
            s.push(StoreFinding::FactWithoutReverseEdge {
                statement: sid,
                relation: rel,
                row_id,
                reverse_key: scratch[..r_len].into(),
            });
        }
        match judged {
            Err(Error::ContainmentViolation {
                statement,
                direction,
                fact,
            }) => {
                s.push(StoreFinding::JudgmentViolation {
                    statement,
                    direction,
                    fact,
                });
            }
            // A corruption inside the probe (a guard row id resolving to
            // no fact, a malformed key width) is a namespace desync the
            // U pass convicts on its own — the judgment neither
            // double-reports it nor decides through it.
            Ok(()) | Err(Error::Corruption(_)) => {}
            Err(other) => return Err(other),
        }
    }
    Ok(())
}
