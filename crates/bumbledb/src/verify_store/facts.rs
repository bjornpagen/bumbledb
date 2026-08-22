//! The `F` pass: one cursor over `F | relation | row_id`. Per live fact —
//! its `M` entry must point back, every key statement's determinant must hold
//! its row id in `U`, and every outgoing containment whose φ it satisfies
//! must have its `R` edge **and its global judgment hold** (the target
//! tuple present or covered, through the commit path's own probes — one
//! `F` scan shared across every statement, never a scan per statement).

use std::ops::Bound;

use crate::encoding::InternId;
use crate::encoding::{FieldDecodeError, ValueType, decode_field, fact_hash, field_word_bytes};
use crate::error::{Check, CorruptionError, Direction, Error, Result, Violation};
use crate::schema::{AxiomIndex, CapacityEnforcement, Enforcement};
use crate::storage::catalog::{Bounds, CatalogMap, CatalogRead, ReadCursor};
use crate::storage::commit::judgment;
use crate::storage::keys::{self, DeterminantImage, KeyBuf, MAX_KEY};
use bumbledb_theory::schema::RelationId;

use super::{StoreFinding, Sweep, namespace_bounds};

#[expect(
    clippy::too_many_lines,
    reason = "the linear per-fact coherence walk is clearer kept together"
)]
pub(super) fn sweep<C: CatalogRead + Copy>(
    s: &mut Sweep<'_, C>,
    checker: &mut judgment::Checker<'_, C>,
) -> Result<()> {
    let schema = s.schema;
    let mut scratch: KeyBuf = [0; MAX_KEY];
    let mut determinant = DeterminantImage::scratch();
    let catalog = s.catalog;
    let (lo, hi) = namespace_bounds(keys::Namespace::Fact);
    let mut range = catalog.range(
        CatalogMap::Data,
        Bounds {
            start: Bound::Included(&lo),
            end: Bound::Excluded(&hi),
        },
    )?;
    while let Some(entry) = ReadCursor::next(&mut range)? {
        let key = entry.key;
        let fact = entry.value;
        let Some((rel, row_id)) = keys::parse_fact_key(key) else {
            s.malformed(key, "F key length");
            continue;
        };
        let Some(relation) = schema.relation_checked(rel) else {
            s.malformed(key, "F key relation");
            continue;
        };

        if relation.body().closed_rows().is_some() {
            s.corrupt(CorruptionError::ClosedRelationEntry {
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

        // the online reader and the offline proof. Keep walking after a

        for idx in 0..layout.field_count() {
            if let Err(error) = decode_field(layout.encoded(fact), idx) {
                let what = match error {
                    FieldDecodeError::InvalidBool(_) => "F fact bool",
                    FieldDecodeError::NonzeroFixedBytesPad(_) => "F fact fixed bytes padding",
                    FieldDecodeError::InvalidInterval(_) => "F fact interval",
                    FieldDecodeError::InvalidFixedIntervalStart(_) => "F fact fixed interval start",
                };
                s.malformed(key, what);
            }
        }

        for (idx, field) in relation.fields().iter().enumerate() {
            if field.generation == bumbledb_theory::schema::Generation::Fresh {
                let value = u64::from_be_bytes(field_word_bytes(layout.encoded(fact), idx));
                let field_id =
                    bumbledb_theory::schema::FieldId(u16::try_from(idx).expect("field count u16"));
                let max = s.max_fresh.entry((rel, field_id)).or_insert(0);
                *max = (*max).max(value);
            }
        }

        for idx in 0..layout.field_count() {
            if matches!(layout.field_type(idx), ValueType::String) {
                let id = InternId::from_raw(u64::from_be_bytes(field_word_bytes(
                    layout.encoded(fact),
                    idx,
                )));

                if id.is_sentinel() {
                    s.malformed(key, "F intern sentinel");
                    continue;
                }
                s.referenced_interns.insert(id);
                if id >= s.dict_next_id {
                    s.corrupt(CorruptionError::InternBeyondNextId {
                        relation: rel,
                        row_id,
                        intern_id: id,
                        next_id: s.dict_next_id,
                    });
                }
            }
        }

        let m_key = keys::membership_key(rel, &fact_hash(fact));
        let points_back = s
            .catalog
            .get(CatalogMap::Data, &m_key)?
            .is_some_and(|v| v.as_ref() == row_id.to_le_bytes().as_slice());
        if !points_back {
            s.corrupt(CorruptionError::FactWithoutMembership {
                relation: rel,
                row_id,
                membership_key: Box::from(m_key),
            });
        }

        if let Some(field) = schema.fresh_mint_field(rel) {
            let fresh =
                u64::from_be_bytes(field_word_bytes(layout.encoded(fact), usize::from(field.0)));
            if fresh != row_id {
                s.corrupt(CorruptionError::FreshRowDesync {
                    relation: rel,
                    row_id,
                    fresh,
                });
            }
        }

        for &key_id in relation.keys() {
            let statement = schema.key(key_id);
            if statement.form().as_fresh_row().is_some() {
                continue;
            }
            keys::determinant_image(
                layout.encoded(fact),
                &statement.projection,
                &mut determinant,
            );
            let u_key =
                keys::determinant_key(&mut scratch, rel, statement.id, determinant.as_bytes());
            let held = s
                .catalog
                .get(CatalogMap::Data, u_key)?
                .is_some_and(|v| v.as_ref() == row_id.to_le_bytes().as_slice());
            if !held {
                s.corrupt(CorruptionError::FactWithoutDeterminant {
                    relation: rel,
                    statement: statement.id,
                    row_id,
                    determinant_key: u_key.into(),
                });
            }
        }

        check_outgoing(
            s,
            checker,
            rel,
            row_id,
            fact,
            &mut scratch,
            &mut determinant,
        )?;
        check_marks(
            s,
            checker,
            rel,
            row_id,
            fact,
            &mut scratch,
            &mut determinant,
        )?;
    }
    check_extension_sources(s, checker)
}

fn check_marks<C: CatalogRead + Copy>(
    s: &mut Sweep<'_, C>,
    checker: &mut judgment::Checker<'_, C>,
    rel: RelationId,
    row_id: u64,
    fact: &[u8],
    scratch: &mut KeyBuf,
    determinant: &mut DeterminantImage,
) -> Result<()> {
    let schema = s.schema;
    let relation = schema.relation(rel);
    let layout = relation.layout();
    for &capacity_id in relation.capacity_sources() {
        let statement = schema.capacity(capacity_id);
        if !judgment::satisfies(&s.selections.capacity(capacity_id).source, layout, fact) {
            continue;
        }
        judgment::capacity_child_image(statement, layout, fact, determinant);
        let r_key = keys::reverse_key(scratch, statement.id, determinant.as_bytes(), rel, row_id);
        let catalog = s.catalog;
        match catalog.get(CatalogMap::Data, r_key)? {
            None => {
                s.corrupt(CorruptionError::FactWithoutReverseEdge {
                    statement: statement.id,
                    relation: rel,
                    row_id,
                    reverse_key: r_key.into(),
                });
            }
            Some(stored) => {
                let derived_word;
                let derived: &[u8] = match judgment::expected_slot_weight(statement, layout, fact) {
                    Ok(Some(weight)) => {
                        derived_word = weight.to_le_bytes();
                        &derived_word
                    }
                    Ok(None) => &[],

                    Err(_) => {
                        s.malformed(r_key, "R capacity weight of a ray");
                        continue;
                    }
                };
                if stored.as_ref() != derived {
                    s.corrupt(CorruptionError::ReverseEdgeWeightDesync {
                        statement: statement.id,
                        reverse_key: r_key.into(),
                        stored: stored.as_ref().into(),
                        derived: derived.into(),
                    });
                }
            }
        }
    }
    for &capacity_id in relation.capacity_targets() {
        let statement = schema.capacity(capacity_id);
        let CapacityEnforcement::ScalarProbe { target_key, .. } = &statement.enforcement else {
            continue;
        };
        {
            let checks = s.selections.capacity(capacity_id);
            if !judgment::satisfies(&checks.target, layout, fact) {
                continue;
            }
        }
        let key_statement = schema.key(*target_key);
        keys::determinant_image(layout.encoded(fact), &key_statement.projection, determinant);
        let checks = s.selections.capacity(capacity_id);
        match checker.check_capacity(statement, checks, determinant.as_bytes()) {
            Ok(Check::Holds) | Err(Error::Corruption(_)) => {}
            Ok(Check::Violated(violation)) => s.push(StoreFinding::Judgment(violation)),
            // A ray met at measure time (C10's judge-side refusal) is
            Err(Error::CapacityRayMeasure { .. }) => {
                s.malformed(determinant.as_bytes(), "capacity measure of a ray");
            }
            Err(other) => return Err(other),
        }
    }
    Ok(())
}

fn check_outgoing<C: CatalogRead + Copy>(
    s: &mut Sweep<'_, C>,
    checker: &mut judgment::Checker<'_, C>,
    rel: RelationId,
    row_id: u64,
    fact: &[u8],
    scratch: &mut KeyBuf,
    determinant: &mut DeterminantImage,
) -> Result<()> {
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
        let (target_key, key_projection) = match &statement.enforcement {
            Enforcement::ScalarProbe {
                target_key,
                key_projection,
            }
            | Enforcement::IntervalCoverage {
                target_key,
                key_projection,
                ..
            } => (target_key, key_projection),

            Enforcement::Closed { members } => {
                let id = u64::from_be_bytes(field_word_bytes(
                    layout.encoded(fact),
                    usize::from(statement.source.projection[0].0),
                ));
                if !AxiomIndex::try_from(id).is_ok_and(|index| members.contains(index)) {
                    s.push(StoreFinding::Judgment(Violation::containment(
                        schema.cite(sid),
                        Direction::TargetRequired,
                        fact.into(),
                    )));
                }
                continue;
            }
        };
        keys::determinant_image(layout.encoded(fact), key_projection, determinant);
        let r_key = keys::reverse_key(scratch, sid, determinant.as_bytes(), rel, row_id);
        let missing_edge = s.catalog.get(CatalogMap::Data, r_key)?.is_none();
        let probe = judgment::Probe {
            statement: sid,
            target_relation: statement.target.relation,
            target_key: *target_key,
            target_check: &checks.target,
            key_bytes: determinant.as_bytes(),
            fact_bytes: fact,
            direction: Direction::TargetRequired,
        };
        let judged = match &statement.enforcement {
            Enforcement::ScalarProbe { .. } => checker.check_scalar(&probe),
            Enforcement::IntervalCoverage {
                disjoint,
                source_tail,
                target_tail,
                ..
            } => checker.check_coverage(*disjoint, *source_tail, *target_tail, &probe),
            Enforcement::Closed { .. } => unreachable!("classified above"),
        };
        if missing_edge {
            s.corrupt(CorruptionError::FactWithoutReverseEdge {
                statement: sid,
                relation: rel,
                row_id,
                reverse_key: r_key.into(),
            });
        }
        match judged {
            Ok(Check::Holds) | Err(Error::Corruption(_)) => {}
            Ok(Check::Violated(violation)) => s.push(StoreFinding::Judgment(violation)),

            Err(other) => return Err(other),
        }
    }
    Ok(())
}

fn check_extension_sources<C: CatalogRead + Copy>(
    s: &mut Sweep<'_, C>,
    checker: &mut judgment::Checker<'_, C>,
) -> Result<()> {
    let schema = s.schema;
    let mut determinant = DeterminantImage::scratch();
    for relation in schema.relations() {
        let Some(rows) = relation.body().closed_rows() else {
            continue;
        };
        let layout = relation.layout();
        for &containment_id in relation.outgoing() {
            let statement = schema.containment(containment_id);
            let sid = statement.id;
            for row in rows {
                // before the finding push.
                let checks = s.selections.containment(containment_id);
                if !judgment::satisfies(&checks.source, layout, &row.fact) {
                    continue;
                }
                let judged = match &statement.enforcement {
                    Enforcement::ScalarProbe { key_projection, .. } => {
                        keys::determinant_image(
                            layout.encoded(&row.fact),
                            key_projection,
                            &mut determinant,
                        );
                        checker.check_scalar(&judgment::Probe::of(
                            statement,
                            &checks.target,
                            determinant.as_bytes(),
                            &row.fact,
                            Direction::TargetRequired,
                        ))
                    }
                    Enforcement::IntervalCoverage { .. } => {
                        unreachable!("closed sources cannot have interval containments")
                    }
                    Enforcement::Closed { members } => {
                        let id = u64::from_be_bytes(field_word_bytes(
                            layout.encoded(&row.fact),
                            usize::from(statement.source.projection[0].0),
                        ));
                        if AxiomIndex::try_from(id).is_ok_and(|index| members.contains(index)) {
                            Ok(Check::Holds)
                        } else {
                            Ok(Check::Violated(Violation::containment(
                                schema.cite(sid),
                                Direction::TargetRequired,
                                row.fact.clone(),
                            )))
                        }
                    }
                };
                match judged {
                    Ok(Check::Holds) | Err(Error::Corruption(_)) => {}
                    Ok(Check::Violated(violation)) => s.push(StoreFinding::Judgment(violation)),
                    Err(other) => return Err(other),
                }
            }
        }
    }
    Ok(())
}
