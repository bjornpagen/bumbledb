//! The `F` pass: one cursor over `F | relation | row_id`. Per live fact —
//! its `M` entry must point back, every key statement's guard must hold
//! its row id in `U`, and every outgoing containment whose φ it satisfies
//! must have its `R` edge **and its global judgment hold** (the target
//! tuple present or covered, through the commit path's own probes — one
//! `F` scan shared across every statement, never a scan per statement).
//! The same walk feeds the per-relation tallies (row count, max row id —
//! no second scan) and collects the referenced intern ids, checking each
//! against the dictionary next-id counter.

use crate::encoding::{TypeDesc, fact_hash, field_bytes};
use crate::error::{Direction, Error, Result};
use crate::schema::{Enforcement, RelationId};
use crate::storage::commit::judgment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

use super::{StoreFinding, Sweep, namespace};

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
        // Closed relations are exempt from the coherence walks — they
        // have no rows in the store — so the entry's existence is the
        // finding (never tallied: the counter pass reconciles facts that
        // may legally exist).
        if relation.is_closed() {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
            continue;
        }
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

        // Referenced intern ids, bounded by the dictionary next-id
        // (String only — bytes<N> values are inline, never interned).
        for idx in 0..layout.field_count() {
            if matches!(layout.field_type(idx), TypeDesc::String) {
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
        for &key_id in relation.keys() {
            let statement = schema.key(key_id);
            keys::guard_bytes(layout, &statement.projection, fact, &mut guard);
            let u_len = keys::guard_key(&mut scratch, rel, statement.id, &guard);
            let held = s
                .data
                .get(txn.raw(), &scratch[..u_len])?
                .is_some_and(|v| v == row_id.to_le_bytes().as_slice());
            if !held {
                s.push(StoreFinding::FactWithoutGuard {
                    relation: rel,
                    statement: statement.id,
                    row_id,
                    guard_key: scratch[..u_len].into(),
                });
            }
        }

        check_outgoing(s, &mut checker, rel, row_id, fact, &mut scratch, &mut guard)?;
    }
    check_extension_sources(s, &mut checker)
}

/// F→R plus the global containment judgment, per outgoing containment
/// whose source selection the fact satisfies — the derivation and the φ
/// check are the commit path's own. The `R` edge must exist, and the
/// target tuple must be present (scalar probe) or covered (coverage walk)
/// in the committed state — the same [`judgment::Checker`] the commit
/// path consumes, over this sweep's read snapshot. A judgment miss is
/// [`StoreFinding::JudgmentViolation`], directed `TargetRequired`: every
/// committed source is a standing one.
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
    for &containment_id in relation.outgoing() {
        let statement = schema.containment(containment_id);
        let sid = statement.id;
        let checks = s.selections.containment(containment_id);
        if !judgment::satisfies(&checks.source, layout, fact) {
            continue;
        }
        let (target_key, key_permutation, coverage) = match &statement.enforcement {
            Enforcement::Probe {
                target_key,
                key_permutation,
                coverage,
            } => (target_key, key_permutation, *coverage),
            // A closed-target containment has no `R` edge and no guard to
            // probe — the F↔R walk skips it, and the global judgment is
            // the membership test itself.
            Enforcement::Closed { members } => {
                let word = field_bytes(fact, layout, usize::from(statement.source.projection[0].0));
                let id = u64::from_be_bytes(word.try_into().expect("u64 field is 8 bytes"));
                if !crate::schema::closed_member(members, id) {
                    s.push(StoreFinding::JudgmentViolation {
                        statement: sid,
                        direction: Direction::TargetRequired,
                        fact: fact.into(),
                    });
                }
                continue;
            }
        };
        keys::permuted_guard_bytes(
            layout,
            &statement.source.projection,
            key_permutation,
            fact,
            guard,
        );
        let r_len = keys::reverse_key(scratch, sid, guard, rel, row_id);
        let missing_edge = s.data.get(txn.raw(), &scratch[..r_len])?.is_none();
        let probe = judgment::Probe {
            statement: sid,
            target_relation: statement.target.relation,
            target_key: *target_key,
            target_check: &checks.target,
            key_bytes: guard,
            fact_bytes: fact,
            direction: Direction::TargetRequired,
        };
        let judged = if coverage {
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

/// The global judgment over **constant sources**: a closed relation has
/// no `F` rows to ride the fact scan, so its outgoing statements
/// re-verify here — each sealed φ-row probes its target exactly as a
/// committed source fact would (domain quantification,
/// `docs/architecture/30-dependencies.md`). Closed→closed statements
/// re-run the compiled membership; validate refuted them at declaration,
/// so a finding here means the schema witness and the store disagree
/// about the theory itself.
fn check_extension_sources(
    s: &mut Sweep<'_, '_>,
    checker: &mut judgment::Checker<'_>,
) -> Result<()> {
    let schema = s.schema;
    let mut guard = Vec::new();
    for relation in schema.relations() {
        let Some(rows) = relation.extension() else {
            continue;
        };
        let layout = relation.layout();
        for &containment_id in relation.outgoing() {
            let statement = schema.containment(containment_id);
            let sid = statement.id;
            for row in rows {
                // Fetched per row so the borrow of `s.selections` ends
                // before the finding push.
                let checks = s.selections.containment(containment_id);
                if !judgment::satisfies(&checks.source, layout, &row.fact) {
                    continue;
                }
                let judged = match &statement.enforcement {
                    Enforcement::Probe {
                        target_key,
                        key_permutation,
                        coverage,
                    } => {
                        // Interval positions on closed containments are
                        // refused at validate — the coverage walk never
                        // runs from a constant source.
                        debug_assert!(!coverage);
                        keys::permuted_guard_bytes(
                            layout,
                            &statement.source.projection,
                            key_permutation,
                            &row.fact,
                            &mut guard,
                        );
                        checker.check_scalar(&judgment::Probe {
                            statement: sid,
                            target_relation: statement.target.relation,
                            target_key: *target_key,
                            target_check: &checks.target,
                            key_bytes: &guard,
                            fact_bytes: &row.fact,
                            direction: Direction::TargetRequired,
                        })
                    }
                    Enforcement::Closed { members } => {
                        let word = field_bytes(
                            &row.fact,
                            layout,
                            usize::from(statement.source.projection[0].0),
                        );
                        let id = u64::from_be_bytes(word.try_into().expect("u64 field is 8 bytes"));
                        if crate::schema::closed_member(members, id) {
                            Ok(())
                        } else {
                            Err(Error::ContainmentViolation {
                                statement: sid,
                                direction: Direction::TargetRequired,
                                fact: row.fact.clone(),
                            })
                        }
                    }
                };
                match judged {
                    Err(Error::ContainmentViolation {
                        statement,
                        direction,
                        fact,
                    }) => s.push(StoreFinding::JudgmentViolation {
                        statement,
                        direction,
                        fact,
                    }),
                    Ok(()) | Err(Error::Corruption(_)) => {}
                    Err(other) => return Err(other),
                }
            }
        }
    }
    Ok(())
}
