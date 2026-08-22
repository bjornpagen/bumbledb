//! Heap stage → packed catalog: key merge, live dictionary, freeze.
use std::collections::HashMap;

use crate::arena::{Arena, ArenaSlice};
use crate::encoding::{InternId, decode_u64, field_word_bytes};
use crate::error::{Admission, Conflict, Error, Result, Violation, Violations};
use crate::schema::{KeyForm, RelationBody, RelationId, Schema, StatementView};
use crate::storage::catalog::frozen::{FrozenCatalog, FrozenMap};
use crate::storage::catalog::heap::{FactRef, HeapStage};
use crate::storage::commit::judgment::{Selections, expected_slot_weight, satisfies};
use crate::storage::dict;
use crate::storage::keys::{self, DeterminantImage, KeyBuf};
use bumbledb_theory::schema::ValueType;

use super::complete::judge_complete;
use super::decorate::decorate_violations;

/// Readable packed catalog after the key merge, before the statement roster.
pub(crate) struct CandidateCatalog {
    inner: FrozenCatalog,
}

impl CandidateCatalog {
    pub(crate) fn freeze(self) -> FrozenCatalog {
        self.inner
    }

    fn catalog(&self) -> &FrozenCatalog {
        &self.inner
    }
}

/// Assigned fact after the sort-and-number pass. Bytes live in the run arena so
/// the stage's fact arena can drop.
struct AssignedFact {
    relation: RelationId,
    row: u64,
    hash: [u8; 32],
    bytes: ArenaSlice,
}

pub(crate) fn admit_catalog(
    schema: &Schema,
    mut stage: HeapStage,
) -> Result<Admission<FrozenCatalog>> {
    stage.discard_identity();
    let selections = Selections::encode_lookup(schema, |raw| Ok(stage.lookup_raw(raw)))?;

    let mut runs = Arena::new();
    let assigned = assign_rows(schema, &stage, &mut runs);
    stage.release_facts();

    let mut data_entries = Vec::new();
    let mut dict_entries = Vec::new();
    emit_fact_and_membership(&mut runs, &assigned, &mut data_entries);
    emit_key_trees(schema, &mut runs, &assigned, &mut data_entries);
    emit_reverse_edges(schema, &selections, &mut runs, &assigned, &mut data_entries)?;
    emit_floors_and_stats(schema, &stage, &assigned, &mut runs, &mut data_entries);
    emit_live_dict(schema, &stage, &assigned, &mut runs, &mut dict_entries)?;
    let dict_next = live_dict_next(schema, &runs, &assigned);
    drop(stage);

    data_entries.sort_by(|a, b| runs.get(a.0).cmp(runs.get(b.0)));
    dict_entries.sort_by(|a, b| runs.get(a.0).cmp(runs.get(b.0)));

    let mut key_violations = Vec::new();
    let data =
        pack_data_with_key_phase(schema, &runs, &assigned, &data_entries, &mut key_violations)?;
    let dict = FrozenMap::pack_slices(
        dict_entries
            .iter()
            .map(|(k, v)| (runs.get(*k), runs.get(*v))),
    );
    let candidate = CandidateCatalog {
        inner: FrozenCatalog::from_parts(data, dict, dict_next),
    };

    if !key_violations.is_empty() {
        let sealed = match Violations::seal(schema, key_violations) {
            Admission::Rejected(violations) => {
                decorate_violations(violations, schema, candidate.catalog())
            }
            Admission::Accepted(()) => unreachable!("nonempty collector"),
        };
        return Ok(Admission::Rejected(sealed));
    }

    let statement = judge_complete(candidate.catalog(), schema, &selections)?;
    match Violations::seal(schema, statement) {
        Admission::Accepted(()) => Ok(Admission::Accepted(candidate.freeze())),
        Admission::Rejected(violations) => Ok(Admission::Rejected(decorate_violations(
            violations,
            schema,
            candidate.catalog(),
        ))),
    }
}

fn assign_rows(schema: &Schema, stage: &HeapStage, runs: &mut Arena) -> Vec<AssignedFact> {
    let mut refs: Vec<FactRef> = stage.fact_refs().to_vec();
    refs.sort_by_key(|r| (r.relation, r.hash));
    let mut next_row = vec![0u64; schema.relations().len()];
    let mut assigned = Vec::with_capacity(refs.len());
    for fact in refs {
        let bytes = stage.fact_bytes(fact);
        let row = assigned_row(schema, fact.relation, bytes, &mut next_row);
        assigned.push(AssignedFact {
            relation: fact.relation,
            row,
            hash: fact.hash,
            bytes: runs.alloc(bytes),
        });
    }
    assigned
}

fn assigned_row(schema: &Schema, relation: RelationId, bytes: &[u8], next_row: &mut [u64]) -> u64 {
    let spec = schema.relation(relation);
    if let Some(field) = schema.fresh_mint_field(relation) {
        let word = field_word_bytes(spec.layout().encoded(bytes), usize::from(field.0));
        return decode_u64(word);
    }
    let idx = relation.0 as usize;
    let row = next_row[idx];
    next_row[idx] = row.checked_add(1).expect("row id space exhausted");
    row
}

fn emit_fact_and_membership(
    runs: &mut Arena,
    assigned: &[AssignedFact],
    entries: &mut Vec<(ArenaSlice, ArenaSlice)>,
) {
    for fact in assigned {
        let f_key = keys::fact_key(fact.relation, fact.row);
        entries.push((runs.alloc(&f_key), fact.bytes));
        let m_key = keys::membership_key(fact.relation, &fact.hash);
        entries.push((runs.alloc(&m_key), runs.alloc(&fact.row.to_le_bytes())));
    }
}

fn emit_key_trees(
    schema: &Schema,
    runs: &mut Arena,
    assigned: &[AssignedFact],
    entries: &mut Vec<(ArenaSlice, ArenaSlice)>,
) {
    let mut buf = [0; keys::MAX_KEY];
    let mut image = DeterminantImage::scratch();
    for fact in assigned {
        let spec = schema.relation(fact.relation);
        let layout = spec.layout();
        for &key_id in spec.keys() {
            let statement = schema.key(key_id);
            match statement.form() {
                KeyForm::FreshRow { .. } => {}
                KeyForm::Scalar | KeyForm::Pointwise { .. } => {
                    keys::determinant_image(
                        layout.encoded(runs.get(fact.bytes)),
                        &statement.projection,
                        &mut image,
                    );
                    let u_key = keys::determinant_key(
                        &mut buf,
                        fact.relation,
                        statement.id,
                        image.as_bytes(),
                    );
                    entries.push((runs.alloc(u_key), runs.alloc(&fact.row.to_le_bytes())));
                }
            }
        }
    }
}

fn emit_reverse_edges(
    schema: &Schema,
    selections: &Selections<'_>,
    runs: &mut Arena,
    assigned: &[AssignedFact],
    entries: &mut Vec<(ArenaSlice, ArenaSlice)>,
) -> Result<()> {
    let mut buf: KeyBuf = [0; keys::MAX_KEY];
    let mut image = DeterminantImage::scratch();
    for fact in assigned {
        let spec = schema.relation(fact.relation);
        let layout = spec.layout();
        let bytes = runs.get(fact.bytes).to_vec();
        for &containment_id in spec.outgoing() {
            let statement = schema.containment(containment_id);
            if !satisfies(
                &selections.containment(containment_id).source,
                layout,
                &bytes,
            ) {
                continue;
            }
            match &statement.enforcement {
                crate::schema::Enforcement::ScalarProbe { key_projection, .. }
                | crate::schema::Enforcement::IntervalCoverage { key_projection, .. } => {
                    keys::determinant_image(layout.encoded(&bytes), key_projection, &mut image);
                    let r_key = keys::reverse_key(
                        &mut buf,
                        statement.id,
                        image.as_bytes(),
                        fact.relation,
                        fact.row,
                    );
                    entries.push((runs.alloc(r_key), runs.alloc(&[])));
                }
                crate::schema::Enforcement::Closed { .. } => {}
            }
        }
        for &capacity_id in spec.capacity_sources() {
            let statement = schema.capacity(capacity_id);
            if !satisfies(&selections.capacity(capacity_id).source, layout, &bytes) {
                continue;
            }
            crate::storage::commit::judgment::capacity_child_image(
                statement, layout, &bytes, &mut image,
            );
            let r_key = keys::reverse_key(
                &mut buf,
                statement.id,
                image.as_bytes(),
                fact.relation,
                fact.row,
            );
            let value = match expected_slot_weight(statement, layout, &bytes)? {
                None => runs.alloc(&[]),
                Some(weight) => runs.alloc(&weight.to_le_bytes()),
            };
            entries.push((runs.alloc(r_key), value));
        }
    }
    Ok(())
}

fn emit_floors_and_stats(
    schema: &Schema,
    stage: &HeapStage,
    assigned: &[AssignedFact],
    runs: &mut Arena,
    entries: &mut Vec<(ArenaSlice, ArenaSlice)>,
) {
    for (&(relation, field), &floor) in stage.roster().iter().zip(stage.floors()) {
        if floor == 0 {
            continue;
        }
        let key = keys::fresh_key(relation, field);
        entries.push((runs.alloc(&key), runs.alloc(&floor.to_le_bytes())));
    }
    let mut counts = vec![0u64; schema.relations().len()];
    let mut max_row = vec![None; schema.relations().len()];
    for fact in assigned {
        let idx = fact.relation.0 as usize;
        counts[idx] += 1;
        max_row[idx] = Some(max_row[idx].map_or(fact.row, |seen: u64| seen.max(fact.row)));
    }
    for (i, spec) in schema.relations().iter().enumerate() {
        if matches!(spec.body(), RelationBody::Closed { .. }) {
            continue;
        }
        let relation = RelationId(u32::try_from(i).expect("relation count fits u32"));
        if counts[i] > 0 {
            let key = keys::stat_key(relation, keys::StatKind::RowCount);
            entries.push((runs.alloc(&key), runs.alloc(&counts[i].to_le_bytes())));
        }
        if spec.fresh_key().is_none()
            && let Some(max) = max_row[i]
        {
            let high = max.saturating_add(1);
            let key = keys::stat_key(relation, keys::StatKind::RowIdHighWater);
            entries.push((runs.alloc(&key), runs.alloc(&high.to_le_bytes())));
        }
    }
}

fn emit_live_dict(
    schema: &Schema,
    stage: &HeapStage,
    assigned: &[AssignedFact],
    runs: &mut Arena,
    entries: &mut Vec<(ArenaSlice, ArenaSlice)>,
) -> Result<()> {
    let mut ids = live_intern_ids(schema, runs, assigned);
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        let Some(raw) = stage.pending_raw(id) else {
            return Err(Error::Corruption(
                crate::error::CorruptionError::DanglingInternId(id),
            ));
        };
        let fwd = dict::forward_key(raw);
        let rev = dict::reverse_key(id);
        entries.push((
            runs.alloc(fwd.as_slice()),
            runs.alloc(&id.raw().to_be_bytes()),
        ));
        entries.push((runs.alloc(rev.as_slice()), runs.alloc(raw)));
    }
    Ok(())
}

fn live_intern_ids(schema: &Schema, runs: &Arena, assigned: &[AssignedFact]) -> Vec<InternId> {
    let mut ids = Vec::new();
    for fact in assigned {
        let spec = schema.relation(fact.relation);
        let layout = spec.layout();
        let bytes = runs.get(fact.bytes);
        for (idx, field) in spec.fields().iter().enumerate() {
            if field.value_type != ValueType::String {
                continue;
            }
            let word = field_word_bytes(layout.encoded(bytes), idx);
            ids.push(InternId::from_raw(decode_u64(word)));
        }
    }
    ids
}

fn live_dict_next(schema: &Schema, runs: &Arena, assigned: &[AssignedFact]) -> InternId {
    match live_intern_ids(schema, runs, assigned).into_iter().max() {
        Some(id) => InternId::from_raw(id.raw().saturating_add(1)),
        None => InternId::from_raw(0),
    }
}

fn pack_data_with_key_phase(
    schema: &Schema,
    runs: &Arena,
    assigned: &[AssignedFact],
    entries: &[(ArenaSlice, ArenaSlice)],
    violations: &mut Vec<Violation>,
) -> Result<FrozenMap> {
    let by_row = row_index(assigned);
    let mut unique: Vec<(&[u8], &[u8])> = Vec::with_capacity(entries.len());
    let mut i = 0;
    while i < entries.len() {
        let key = runs.get(entries[i].0);
        let value = runs.get(entries[i].1);
        let mut j = i + 1;
        while j < entries.len() && runs.get(entries[j].0) == key {
            j += 1;
        }
        if j > i + 1 {
            record_duplicate_key(
                schema,
                runs,
                assigned,
                &by_row,
                key,
                entries[i].1,
                entries[i + 1].1,
                violations,
            )?;
        }
        unique.push((key, value));
        i = j;
    }
    probe_pointwise_neighbors(schema, runs, assigned, &by_row, &unique, violations)?;
    Ok(FrozenMap::pack_slices(unique))
}

fn row_index(assigned: &[AssignedFact]) -> HashMap<(u32, u64), usize> {
    let mut map = HashMap::with_capacity(assigned.len());
    for (i, fact) in assigned.iter().enumerate() {
        map.insert((fact.relation.0, fact.row), i);
    }
    map
}

fn fact_at<'a>(
    assigned: &'a [AssignedFact],
    by_row: &HashMap<(u32, u64), usize>,
    runs: &'a Arena,
    relation: RelationId,
    row: u64,
) -> Result<&'a [u8]> {
    let idx = by_row.get(&(relation.0, row)).ok_or(Error::Corruption(
        crate::error::CorruptionError::MissingFact {
            relation,
            row_id: row,
        },
    ))?;
    Ok(runs.get(assigned[*idx].bytes))
}

#[allow(clippy::too_many_arguments)]
fn record_duplicate_key(
    schema: &Schema,
    runs: &Arena,
    assigned: &[AssignedFact],
    by_row: &HashMap<(u32, u64), usize>,
    key: &[u8],
    first_value: ArenaSlice,
    second_value: ArenaSlice,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    if let Some((relation, _)) = keys::parse_fact_key(key) {
        let Some(key_id) = schema.relation(relation).fresh_key() else {
            unreachable!("F-key collision only exists on a fresh-keyed relation");
        };
        let statement = schema.key(key_id);
        violations.push(Violation::functionality(
            schema.cite(statement.id),
            runs.get(second_value).into(),
            Conflict::Scalar,
        ));
        return Ok(());
    }
    if let Some((relation, statement_id, _)) = keys::parse_determinant_key(key) {
        let first_row = crate::storage::stored_u64(runs.get(first_value), "U row id")?;
        let second_row = crate::storage::stored_u64(runs.get(second_value), "U row id")?;
        let first = fact_at(assigned, by_row, runs, relation, first_row)?;
        let second = fact_at(assigned, by_row, runs, relation, second_row)?;
        let conflict = match schema.statement(statement_id) {
            StatementView::Key(_, key) => match key.form() {
                KeyForm::Scalar | KeyForm::FreshRow { .. } => Conflict::Scalar,
                KeyForm::Pointwise { .. } => Conflict::Pointwise {
                    incumbent: first.into(),
                },
            },
            _ => unreachable!("U keys name key statements"),
        };
        violations.push(Violation::functionality(
            schema.cite(statement_id),
            second.into(),
            conflict,
        ));
        return Ok(());
    }
    unreachable!("key-phase duplicates are F or U keys")
}

fn probe_pointwise_neighbors(
    schema: &Schema,
    runs: &Arena,
    assigned: &[AssignedFact],
    by_row: &HashMap<(u32, u64), usize>,
    unique: &[(&[u8], &[u8])],
    violations: &mut Vec<Violation>,
) -> Result<()> {
    for window in unique.windows(2) {
        let (prev, next) = (window[0], window[1]);
        let Some((rel_a, stmt_a, det_a)) = keys::parse_determinant_key(prev.0) else {
            continue;
        };
        let Some((rel_b, stmt_b, det_b)) = keys::parse_determinant_key(next.0) else {
            continue;
        };
        if rel_a != rel_b || stmt_a != stmt_b {
            continue;
        }
        let StatementView::Key(_, key) = schema.statement(stmt_a) else {
            continue;
        };
        let KeyForm::Pointwise { tail, .. } = key.form() else {
            continue;
        };
        let tail_bytes = tail.width();
        if det_a.len() < tail_bytes || det_b.len() < tail_bytes {
            return Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("U determinant tail"),
            ));
        }
        let prefix_a = &det_a[..det_a.len() - tail_bytes];
        let prefix_b = &det_b[..det_b.len() - tail_bytes];
        if prefix_a != prefix_b {
            continue;
        }
        let (_, end) = crate::encoding::interval_words(*tail, &det_a[det_a.len() - tail_bytes..])
            .ok_or(Error::Corruption(
            crate::error::CorruptionError::MalformedValue("U determinant tail"),
        ))?;
        let (ns, _) = crate::encoding::interval_words(*tail, &det_b[det_b.len() - tail_bytes..])
            .ok_or(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("U determinant tail"),
            ))?;
        if ns < end {
            let pred_row = crate::storage::stored_u64(prev.1, "U row id")?;
            let succ_row = crate::storage::stored_u64(next.1, "U row id")?;
            let incumbent = fact_at(assigned, by_row, runs, rel_a, pred_row)?;
            let cited = fact_at(assigned, by_row, runs, rel_b, succ_row)?;
            violations.push(Violation::functionality(
                schema.cite(stmt_a),
                cited.into(),
                Conflict::Pointwise {
                    incumbent: incumbent.into(),
                },
            ));
        }
    }
    Ok(())
}
