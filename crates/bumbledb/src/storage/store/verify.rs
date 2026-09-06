//! The offline successor-store sweeper: one coherent snapshot, one pass per
//! physical namespace, then the complete judgment re-run globally.
//!
//! The sweeper's knowledge is the engine's knowledge: key derivations come
//! from [`super::keys`], fingerprints from the store's own fingerprinter,
//! row validity from the canonical codec, and the semantic re-check is the
//! production judge (`schema::judge::judge_final_state`) over this exact
//! snapshot — never a second implementation of any law.
//!
//! ```text
//! rows        key shape, schema knowledge, closed-relation intrusion,
//!             canonical validity, selected scalar home agreement, membership
//!             for unclustered rows, EVERY non-home determinant entry present,
//!             globally unique ordinals, per-relation tallies and max ordinal
//! membership  resolves to a live row whose recomputed fingerprint is the
//!             stored bucket
//! determinant home indexes absent; secondary locator resolves to a live row
//!             with the expected home and recomputed determinant routing
//! meta        family/layout/schema identity, generation presence, stored
//!             row counts against the tallies, the next-row-id ratchet,
//!             host-record key bounds
//! judgment    every sealed statement re-judged over the full final state
//! ```

use bumbledb_theory::schema::RelationId;

use crate::schema::ProjectionId;

use super::error::{StoreError, StoreResult};
use super::format::{
    self, FAMILY, K_FAMILY, K_GENERATION, K_HOST_RECORD_TAG, K_LAYOUT, K_NEXT_ROW_ID,
    K_ROW_COUNT_TAG, K_SCHEMA, K_STORE_ID, LAYOUT, RowId, RowLocator,
};
use super::keys;
use super::rows;
use super::snapshot::OwnedSnapshot;
use crate::canonical::RowError;
use crate::schema::Schema;
use crate::schema::judge::{
    CandidateFacts, JudgeBudget, JudgeError, JudgeScratch, JudgedViolation, Judgment,
    RankedRowVisitor, judge_final_state_with_scratch, store_fault,
};
use crate::work::WorkContext;

/// One observed physical desync inside a recognized successor store.
/// Payloads are namespace ids and typed positions — never formatted
/// strings, never raw key bytes reinterpreted as verdicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyCorruption {
    /// A physical key in a store namespace has an impossible shape.
    MalformedKey { what: &'static str },
    /// A row is stored under a relation the schema does not declare.
    UnknownRelation { relation: RelationId },
    /// A row is stored under a closed relation (closed extension rows are
    /// sealed in the schema, never in the store).
    ClosedRelationRow { relation: RelationId, row: RowId },
    /// A stored row's bytes are not a canonical row of its relation.
    MalformedRow {
        relation: RelationId,
        row: RowId,
        error: RowError,
    },
    /// A physical row route disagrees with its canonical selected scalar key.
    ForeignRowHome { relation: RelationId, row: RowId },
    /// The global ordinal allocator cannot assign one rank twice.
    DuplicateRowId { relation: RelationId, row: RowId },
    /// A live row has no membership entry under its exact fingerprint.
    MissingMembership { relation: RelationId, row: RowId },
    /// A relation uses its selected scalar index for exact membership, but
    /// retains a redundant fingerprint membership entry.
    UnexpectedMembership { relation: RelationId, row: RowId },
    /// A membership entry references a row that does not exist.
    DanglingMembership { relation: RelationId, row: RowId },
    /// A membership entry's bucket disagrees with the row's recomputed
    /// fingerprint.
    ForeignMembership { relation: RelationId, row: RowId },
    /// A determinant entry references no live row of its projection's
    /// relation.
    DanglingDeterminant {
        projection: ProjectionId,
        row: RowId,
    },
    /// A determinant entry names a projection the compiled theory does not
    /// intern (auxiliary leftovers included — neither belongs in a store
    /// the production write path built).
    UnknownDeterminantProjection { projection: ProjectionId },
    /// A live row lacks a schema-derived determinant entry under its
    /// recomputed routing — keyed reads and judgment enumeration would not
    /// see this row for that projection.
    MissingDeterminant {
        projection: ProjectionId,
        row: RowId,
    },
    /// The selected home is the row tree itself, never a redundant index.
    UnexpectedHomeDeterminant {
        projection: ProjectionId,
        row: RowId,
    },
    /// A secondary locator has the wrong width or disagrees with its row home.
    ForeignDeterminantHome {
        projection: ProjectionId,
        row: RowId,
    },
    /// A determinant entry's bucket disagrees with the row's recomputed
    /// routing for that projection.
    ForeignDeterminant {
        projection: ProjectionId,
        row: RowId,
    },
    /// The stored per-relation row count disagrees with the counted rows.
    RowCountMismatch {
        relation: RelationId,
        stored: u64,
        counted: u64,
    },
    /// The next-row-id ratchet is at or below an allocated row id.
    RowIdRatchetBehind { next: u64, max_seen: u64 },
    /// A required meta entry is absent or malformed.
    MetaMissing { what: &'static str },
    /// The stored family/layout/schema identity disagrees with the open
    /// store's own identity.
    IdentityMismatch { what: &'static str },
    /// A host record key exceeds the bounded host-key width.
    HostKeyTooLong { actual: usize },
}

/// One observed desync: physical corruption, or a statement the complete
/// re-judgment finds violated by the committed state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyFinding {
    Judgment(JudgedViolation),
    Corruption(VerifyCorruption),
}

/// Sweep one coherent snapshot. Returns every observed desync in pass
/// order; an empty vector is coherence. Resource exhaustion and storage
/// failure are errors, never a shorter report.
#[expect(
    clippy::too_many_lines,
    reason = "the sweep's passes stay one auditable walk over the trees"
)]
pub(crate) fn sweep(
    snapshot: &OwnedSnapshot,
    schema: &Schema,
    work: &WorkContext,
) -> StoreResult<Vec<VerifyFinding>> {
    let mut findings = Vec::new();
    let mut tallies: std::collections::BTreeMap<RelationId, u64> =
        std::collections::BTreeMap::new();
    let mut max_row_id = 0u64;
    let mut judgment_safe = true;
    let mut ordinals =
        crate::exec::scratch::ScratchRelation::new(work, crate::exec::scratch::DEFAULT_RAM_BYTES);
    let inner = snapshot.store_inner();
    let txn = snapshot.read_txn();

    // Pass 1: rows.
    {
        let mut decode = crate::canonical::DecodeScratch::new(work);
        let prefix = [keys::TAG_ROW];
        let range = inner
            .data
            .prefix_iter(txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, row_bytes) = entry.map_err(StoreError::from_heed)?;
            let Ok((relation, locator)) = inner.keys.decode_row(key) else {
                judgment_safe = false;
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "row key width",
                }));
                continue;
            };
            let row = locator.id;
            max_row_id = max_row_id.max(row.0);
            if !ordinals
                .insert_if_absent(&row.0.to_be_bytes(), &[])
                .map_err(scratch_error)?
            {
                judgment_safe = false;
                findings.push(corrupt(VerifyCorruption::DuplicateRowId { relation, row }));
            }
            *tallies.entry(relation).or_default() += 1;
            let Some(view) = schema.relation_checked(relation) else {
                findings.push(corrupt(VerifyCorruption::UnknownRelation { relation }));
                continue;
            };
            if locator.home().len() != inner.det.home_width(relation) {
                judgment_safe = false;
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "row home width",
                }));
            }
            if view.body().closed_rows().is_some() {
                findings.push(corrupt(VerifyCorruption::ClosedRelationRow {
                    relation,
                    row,
                }));
                continue;
            }
            if let Err(error) = crate::canonical::validate(view.fields(), row_bytes, work) {
                match error {
                    RowError::Work(work_error) => return Err(StoreError::Work(work_error)),
                    other => {
                        judgment_safe = false;
                        findings.push(corrupt(VerifyCorruption::MalformedRow {
                            relation,
                            row,
                            error: other,
                        }));
                        continue;
                    }
                }
            }
            if inner.det.membership_projection(relation).is_none() {
                let fp = inner.fingerprinter.row(relation, row_bytes);
                let membership = inner.keys.membership_key(relation, &fp, row)?;
                let present = inner
                    .data
                    .get(txn, membership.as_slice())
                    .map_err(StoreError::from_heed)?
                    .is_some();
                if !present {
                    findings.push(corrupt(VerifyCorruption::MissingMembership {
                        relation,
                        row,
                    }));
                }
            }
            // Every schema-derived determinant entry, recomputed with the
            // engine's own projection convention and fingerprint, must be
            // present — index completeness is load-bearing for keyed reads
            // and judgment enumeration.
            inner.det.emit_row(
                relation,
                row_bytes,
                &mut decode,
                &mut |projection, projected| {
                    let routing = rows::routing_for_projected(inner, projection, projected)?;
                    if inner.det.is_home(projection) {
                        if locator.home() != routing.as_slice() {
                            findings
                                .push(corrupt(VerifyCorruption::ForeignRowHome { relation, row }));
                        }
                        return Ok(());
                    }
                    let entry = inner.keys.determinant_key(projection, &routing, row)?;
                    let present = inner
                        .data
                        .get(txn, entry.as_slice())
                        .map_err(StoreError::from_heed)?;
                    match present {
                        None => findings.push(corrupt(VerifyCorruption::MissingDeterminant {
                            projection,
                            row,
                        })),
                        Some(home) if home != locator.home() => {
                            findings.push(corrupt(VerifyCorruption::ForeignDeterminantHome {
                                projection,
                                row,
                            }));
                        }
                        Some(_) => {}
                    }
                    Ok(())
                },
            )?;
        }
    }

    // Ordinal uniqueness is complete. Release its resident/spill ownership
    // before index verification and full judgment consume the same budget.
    drop(ordinals);

    // Pass 2: membership entries resolve back, fingerprint-verified.
    {
        let prefix = [keys::TAG_MEMBERSHIP];
        let range = inner
            .data
            .prefix_iter(txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, value) = entry.map_err(StoreError::from_heed)?;
            let Ok((relation, fp, row)) = inner.keys.decode_membership(key) else {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "membership key width",
                }));
                continue;
            };
            if !value.is_empty() {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "membership value must be empty",
                }));
            }
            if inner.det.membership_projection(relation).is_some() {
                findings.push(corrupt(VerifyCorruption::UnexpectedMembership {
                    relation,
                    row,
                }));
                continue;
            }
            match rows::fetch_row(inner, txn, relation, RowLocator::unclustered(row))? {
                None => {
                    findings.push(corrupt(VerifyCorruption::DanglingMembership {
                        relation,
                        row,
                    }));
                }
                Some(bytes) => {
                    if inner.fingerprinter.row(relation, bytes) != fp {
                        findings.push(corrupt(VerifyCorruption::ForeignMembership {
                            relation,
                            row,
                        }));
                    }
                }
            }
        }
    }

    // Pass 3: determinant entries resolve back to live rows.
    {
        let prefix = [keys::TAG_DETERMINANT];
        let range = inner
            .data
            .prefix_iter(txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, home) = entry.map_err(StoreError::from_heed)?;
            let Ok((projection, stored_payload, row)) = inner.keys.decode_determinant(key) else {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "determinant key shape",
                }));
                continue;
            };
            let Some(compiled) = inner.det.projection(projection) else {
                findings.push(corrupt(VerifyCorruption::UnknownDeterminantProjection {
                    projection,
                }));
                continue;
            };
            if inner.det.is_home(projection) {
                findings.push(corrupt(VerifyCorruption::UnexpectedHomeDeterminant {
                    projection,
                    row,
                }));
                continue;
            }
            if home.len() != inner.det.home_width(compiled.relation) {
                findings.push(corrupt(VerifyCorruption::ForeignDeterminantHome {
                    projection,
                    row,
                }));
                continue;
            }
            let locator = RowLocator::new(row, home)?;
            let Some(row_bytes) = rows::fetch_row(inner, txn, compiled.relation, locator)? else {
                findings.push(corrupt(VerifyCorruption::DanglingDeterminant {
                    projection,
                    row,
                }));
                continue;
            };
            // Exactness: the entry's bucket must be the row's recomputed
            // determinant fingerprint under the one projection convention.
            let Some(fields) = inner.det.fields_of(compiled.relation) else {
                findings.push(corrupt(VerifyCorruption::UnknownRelation {
                    relation: compiled.relation,
                }));
                continue;
            };
            match crate::canonical::decode(fields, row_bytes, work) {
                Err(RowError::Work(work_error)) => return Err(StoreError::Work(work_error)),
                Err(_) => {
                    // The row pass already convicted the malformed row.
                }
                Ok(decoded) => {
                    if let Some(selected) = inner.det.membership_projection(compiled.relation) {
                        let values = selected.scalar_values(decoded.values());
                        let expected_home =
                            super::det_index::determinant_bytes(selected, &values, work)?;
                        if home != expected_home.as_slice() {
                            findings.push(corrupt(VerifyCorruption::ForeignDeterminantHome {
                                projection,
                                row,
                            }));
                        }
                    }
                    let values = compiled.scalar_values(decoded.values());
                    let projected = super::det_index::determinant_bytes(compiled, &values, work)?;
                    let expected = rows::routing_for_projected(inner, projection, &projected)?;
                    if stored_payload != expected.as_slice() {
                        findings.push(corrupt(VerifyCorruption::ForeignDeterminant {
                            projection,
                            row,
                        }));
                    }
                }
            }
        }
    }

    // Pass 4: meta identity, counters, ratchet, host key bounds.
    {
        match inner
            .meta
            .get(txn, K_FAMILY)
            .map_err(StoreError::from_heed)?
        {
            Some(bytes) if bytes == FAMILY => {}
            Some(_) => findings.push(corrupt(VerifyCorruption::IdentityMismatch {
                what: "family",
            })),
            None => findings.push(corrupt(VerifyCorruption::MetaMissing { what: "family" })),
        }
        match inner
            .meta
            .get(txn, K_LAYOUT)
            .map_err(StoreError::from_heed)?
        {
            Some(bytes) => match <[u8; 4]>::try_from(bytes) {
                // A wrong-width layout word is the same refusal as a wrong
                // value: the stored identity does not name this layout.
                Ok(raw) if u32::from_be_bytes(raw) == LAYOUT => {}
                Ok(_) | Err(_) => {
                    findings.push(corrupt(VerifyCorruption::IdentityMismatch {
                        what: "layout",
                    }));
                }
            },
            None => findings.push(corrupt(VerifyCorruption::MetaMissing { what: "layout" })),
        }
        match inner
            .meta
            .get(txn, K_SCHEMA)
            .map_err(StoreError::from_heed)?
        {
            Some(bytes) if bytes == inner.schema_fp.0 => {}
            Some(_) => findings.push(corrupt(VerifyCorruption::IdentityMismatch {
                what: "schema fingerprint",
            })),
            None => findings.push(corrupt(VerifyCorruption::MetaMissing {
                what: "schema fingerprint",
            })),
        }
        if inner
            .meta
            .get(txn, K_STORE_ID)
            .map_err(StoreError::from_heed)?
            .is_none()
        {
            findings.push(corrupt(VerifyCorruption::MetaMissing { what: "store id" }));
        }
        if inner
            .meta
            .get(txn, K_GENERATION)
            .map_err(StoreError::from_heed)?
            .is_none()
        {
            findings.push(corrupt(VerifyCorruption::MetaMissing {
                what: "generation",
            }));
        }
        match format::read_u64(&inner.meta, txn, K_NEXT_ROW_ID, "next row id") {
            Ok(next) => {
                if max_row_id > 0 && next <= max_row_id {
                    findings.push(corrupt(VerifyCorruption::RowIdRatchetBehind {
                        next,
                        max_seen: max_row_id,
                    }));
                }
            }
            Err(_) => findings.push(corrupt(VerifyCorruption::MetaMissing {
                what: "next row id",
            })),
        }
        // Stored per-relation counts against the counted rows — both ways:
        // every stored counter and every counted relation must agree.
        let mut stored_counts: std::collections::BTreeMap<RelationId, u64> =
            std::collections::BTreeMap::new();
        let prefix = [K_ROW_COUNT_TAG];
        let range = inner
            .meta
            .prefix_iter(txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, value) = entry.map_err(StoreError::from_heed)?;
            if key.len() != 5 {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "row count key width",
                }));
                continue;
            }
            let relation = RelationId(u32::from_be_bytes(
                key[1..5].try_into().expect("checked width"),
            ));
            match value.try_into().map(u64::from_be_bytes) {
                Ok(count) => {
                    stored_counts.insert(relation, count);
                }
                Err(_) => findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "row count value width",
                })),
            }
        }
        for (relation, counted) in &tallies {
            let stored = stored_counts.remove(relation).unwrap_or(0);
            if stored != *counted {
                findings.push(corrupt(VerifyCorruption::RowCountMismatch {
                    relation: *relation,
                    stored,
                    counted: *counted,
                }));
            }
        }
        for (relation, stored) in stored_counts {
            if stored != 0 {
                findings.push(corrupt(VerifyCorruption::RowCountMismatch {
                    relation,
                    stored,
                    counted: 0,
                }));
            }
        }
        // Per-relation change versions: well-formed words only. No count
        // invariant exists (a relation emptied by deletes keeps its spent
        // versions), and monotonicity is per-lineage, not per-snapshot.
        let version_prefix = [format::K_RELATION_VERSION_TAG];
        let range = inner
            .meta
            .prefix_iter(txn, version_prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, value) = entry.map_err(StoreError::from_heed)?;
            if key.len() != 5 {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "relation version key width",
                }));
                continue;
            }
            if <[u8; 8]>::try_from(value).is_err() {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "relation version value width",
                }));
            }
        }
        let host_prefix = [K_HOST_RECORD_TAG];
        let range = inner
            .meta
            .prefix_iter(txn, host_prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, _) = entry.map_err(StoreError::from_heed)?;
            let host_key_len = key.len().saturating_sub(1);
            if host_key_len > keys::HOST_KEY_MAX {
                findings.push(corrupt(VerifyCorruption::HostKeyTooLong {
                    actual: host_key_len,
                }));
            }
        }
        // The attachment key itself is fixed-width; presence is host policy.
    }

    // Structural faults can make scans undefined (wrong route widths,
    // malformed rows, duplicate logical ranks). Return those findings
    // rather than losing them to a subsequent judgment decode error.
    if !judgment_safe {
        return Ok(findings);
    }
    // Pass 5: the complete production judgment over the committed state.
    let facts = SnapshotFacts {
        snapshot,
        schema,
        work,
    };
    match judge_final_state_with_scratch(
        schema,
        &facts,
        work,
        JudgeBudget::default(),
        JudgeScratch::channel(store_fault),
    ) {
        Ok(Judgment::Admitted) => {}
        Ok(Judgment::Rejected(violations)) => {
            findings.extend(
                violations
                    .into_vec()
                    .into_iter()
                    .map(VerifyFinding::Judgment),
            );
        }
        Err(JudgeError::Work(error)) => return Err(StoreError::Work(error)),
        Err(JudgeError::State(error)) => return Err(error),
        Err(JudgeError::Allocation) => return Err(StoreError::Allocation),
        Err(JudgeError::Compile(error)) => return Err(StoreError::Compile(error)),
        Err(JudgeError::UndefinedDuration { statement }) => {
            return Err(StoreError::JudgeRefused {
                statement,
                detail: "undefined ray duration in a measured position",
            });
        }
        Err(JudgeError::MeasureOverflow { statement }) => {
            return Err(StoreError::JudgeRefused {
                statement,
                detail: "grouped measure exceeded the widened accumulator",
            });
        }
    }

    Ok(findings)
}

const fn corrupt(finding: VerifyCorruption) -> VerifyFinding {
    VerifyFinding::Corruption(finding)
}

fn scratch_error(error: crate::error::Error) -> StoreError {
    match error {
        crate::error::Error::Store(error) => *error,
        crate::error::Error::Io(error) => StoreError::Io(error),
        crate::error::Error::Lmdb(error) => StoreError::Lmdb(error),
        _ => StoreError::Lmdb(crate::error::LmdbFailure::Decoding),
    }
}

/// The committed snapshot presented as candidate facts for the global
/// re-judgment. Closed relations load from the schema inside the judge;
/// this adapter is only asked for ordinary relations.
struct SnapshotFacts<'a> {
    snapshot: &'a OwnedSnapshot,
    schema: &'a Schema,
    work: &'a WorkContext,
}

impl CandidateFacts for SnapshotFacts<'_> {
    type Error = StoreError;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[crate::Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        self.visit_ranked_rows(relation, &mut |_, row| visit(row))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: RankedRowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        let Some(view) = self.schema.relation_checked(relation) else {
            // Unknown relations were already reported as corruption; the
            // judge only asks for sealed relations, so this is unreachable
            // in practice and refuses loudly if reached.
            return Err(StoreError::Corruption(
                super::error::StoreCorruption::MalformedKey("judged relation unknown to schema"),
            ));
        };
        let fields = view.fields();
        for entry in self.snapshot.rows(relation)? {
            let (id, bytes) = entry?;
            let decoded = crate::canonical::decode(fields, bytes, self.work)?;
            if !visit(id.id.0, decoded.values())? {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ValidateDescriptor as _;
    use crate::storage::store::judge_bridge::UnindexedRows;
    use crate::storage::store::tests::{
        NOTE, change_set, create_default, note, schema, store_dir, work,
    };

    #[test]
    fn scalar_home_refuses_redundant_indexes_without_requiring_them() {
        let schema = schema();
        let work = work();
        let (_dir, path) = store_dir("verify-membership-elision");
        let store = create_default(&path);
        let changes = change_set(&schema, &[(NOTE, note(7, "seven"))], &[]);
        store
            .writer(&work)
            .unwrap()
            .ingest(&changes, &UnindexedRows)
            .unwrap();
        let (id, fingerprint, determinant) = {
            let snapshot = store.snapshot(&work).unwrap();
            assert!(sweep(&snapshot, &schema, &work).unwrap().is_empty());
            let (id, bytes) = snapshot.rows(NOTE).unwrap().next().unwrap().unwrap();
            let fingerprint = store.inner.fingerprinter.row(NOTE, bytes);
            let projection = store.inner.det.membership_projection(NOTE).unwrap();
            let mut key = None;
            let mut scratch = crate::canonical::DecodeScratch::new(&work);
            store
                .inner
                .det
                .emit_row(NOTE, bytes, &mut scratch, &mut |p, route| {
                    if p == projection.id {
                        key = Some(store.inner.keys.determinant_key(p, route, id.id)?.to_vec());
                    }
                    Ok(())
                })
                .unwrap();
            (id, fingerprint, key.unwrap())
        };
        {
            let mut txn = store.gated_write_txn(&work).unwrap();
            assert!(
                store
                    .inner
                    .data
                    .get(&txn.txn, &determinant)
                    .unwrap()
                    .is_none()
            );
            store
                .inner
                .data
                .put(&mut txn.txn, &determinant, id.home())
                .unwrap();
            let redundant = store
                .inner
                .keys
                .membership_key(NOTE, &fingerprint, id.id)
                .unwrap();
            store
                .inner
                .data
                .put(&mut txn.txn, &redundant, b"not-empty")
                .unwrap();
            txn.commit().unwrap();
        }
        let snapshot = store.snapshot(&work).unwrap();
        let findings = sweep(&snapshot, &schema, &work).unwrap();
        assert!(findings.contains(&corrupt(VerifyCorruption::MalformedKey {
            what: "membership value must be empty",
        })));
        assert!(
            findings.contains(&corrupt(VerifyCorruption::UnexpectedHomeDeterminant {
                projection: store.inner.det.membership_projection(NOTE).unwrap().id,
                row: id.id,
            }))
        );
        assert!(
            findings.contains(&corrupt(VerifyCorruption::UnexpectedMembership {
                relation: NOTE,
                row: id.id,
            }))
        );
        assert!(!findings.iter().any(|finding| matches!(
            finding,
            VerifyFinding::Corruption(VerifyCorruption::MissingMembership { .. })
        )));
    }

    #[test]
    fn verifier_ranked_rows_keep_retained_row_ids_after_deletion() {
        let schema = schema();
        let work = work();
        let (_dir, path) = store_dir("verifier-ranked-rows");
        let store = create_default(&path);
        let changes = change_set(
            &schema,
            &[
                (NOTE, note(1, "one")),
                (NOTE, note(2, "two")),
                (NOTE, note(3, "three")),
            ],
            &[],
        );
        store
            .writer(&work)
            .unwrap()
            .ingest(&changes, &UnindexedRows)
            .unwrap();
        let remove = change_set(&schema, &[], &[(NOTE, note(1, "one"))]);
        store
            .writer(&work)
            .unwrap()
            .ingest(&remove, &UnindexedRows)
            .unwrap();
        let snapshot = store.snapshot(&work).unwrap();
        let expected: Vec<_> = snapshot
            .rows(NOTE)
            .unwrap()
            .map(|entry| entry.unwrap().0.id.0)
            .collect();
        let facts = SnapshotFacts {
            snapshot: &snapshot,
            schema: &schema,
            work: &work,
        };
        let mut ranks = Vec::new();
        facts
            .visit_ranked_rows(NOTE, &mut |rank, row| {
                assert_ne!(row[0], crate::Value::U64(1));
                ranks.push(rank);
                Ok(true)
            })
            .unwrap();
        assert_eq!(ranks, expected);
        assert_eq!(ranks.len(), 2);
        let mut visited = 0;
        facts
            .visit_ranked_rows(NOTE, &mut |_, _| {
                visited += 1;
                Ok(false)
            })
            .unwrap();
        assert_eq!(visited, 1);
        assert_eq!(
            facts.visit_ranked_rows(NOTE, &mut |_, _| Err(StoreError::ForeignSchema)),
            Err(StoreError::ForeignSchema)
        );
    }

    #[test]
    fn row_home_corruption_and_duplicate_ordinals_are_independently_detected() {
        for (case, home, keep_original) in [
            ("wrong-home", 8u64.to_be_bytes().to_vec(), false),
            ("wrong-width", vec![7], false),
            ("duplicate-rank", 8u64.to_be_bytes().to_vec(), true),
        ] {
            let schema = schema();
            let work = work();
            let (_dir, path) = store_dir(case);
            let store = create_default(&path);
            store
                .writer(&work)
                .unwrap()
                .ingest(
                    &change_set(&schema, &[(NOTE, note(7, "seven"))], &[]),
                    &UnindexedRows,
                )
                .unwrap();
            let (locator, bytes) = {
                let snapshot = store.snapshot(&work).unwrap();
                assert!(sweep(&snapshot, &schema, &work).unwrap().is_empty());
                let (locator, bytes) = snapshot.rows(NOTE).unwrap().next().unwrap().unwrap();
                (locator, bytes.to_vec())
            };
            {
                let mut txn = store.gated_write_txn(&work).unwrap();
                if !keep_original {
                    let old = store.inner.keys.row_key(NOTE, locator).unwrap();
                    assert!(store.inner.data.delete(&mut txn.txn, &old).unwrap());
                }
                let altered = RowLocator::new(locator.id, &home).unwrap();
                let key = store.inner.keys.row_key(NOTE, altered).unwrap();
                store.inner.data.put(&mut txn.txn, &key, &bytes).unwrap();
                txn.commit().unwrap();
            }
            let snapshot = store.snapshot(&work).unwrap();
            let findings = sweep(&snapshot, &schema, &work).unwrap();
            let expected = if keep_original {
                VerifyCorruption::DuplicateRowId {
                    relation: NOTE,
                    row: locator.id,
                }
            } else if home.len() != 8 {
                VerifyCorruption::MalformedKey {
                    what: "row home width",
                }
            } else {
                VerifyCorruption::ForeignRowHome {
                    relation: NOTE,
                    row: locator.id,
                }
            };
            assert!(
                findings.contains(&corrupt(expected)),
                "{case}: {findings:?}"
            );
        }
    }

    #[test]
    fn secondary_home_and_index_completeness_are_independently_verified() {
        use bumbledb_theory::schema::{
            FieldDescriptor, FieldId, RelationDescriptor, SchemaDescriptor, StatementDescriptor,
            ValueType,
        };
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "entry".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "label".into(),
                        value_type: ValueType::String,
                    },
                ],
                extension: None,
            }],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: NOTE,
                    projection: Box::from([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: NOTE,
                    projection: Box::from([FieldId(1)]),
                },
            ],
        }
        .validate()
        .unwrap();
        for home in [None, Some(vec![]), Some(8u64.to_be_bytes().to_vec())] {
            let work = work();
            let (_dir, path) = store_dir("verify-secondary-home");
            let (store, _) =
                super::super::Store::create(&path, &schema, super::super::MapPolicy::default())
                    .unwrap();
            store
                .writer(&work)
                .unwrap()
                .ingest(
                    &change_set(&schema, &[(NOTE, note(7, "seven"))], &[]),
                    &UnindexedRows,
                )
                .unwrap();
            let (projection, row, key) = {
                let snapshot = store.snapshot(&work).unwrap();
                assert!(sweep(&snapshot, &schema, &work).unwrap().is_empty());
                let mut range = store
                    .inner
                    .data
                    .prefix_iter(snapshot.read_txn(), &[keys::TAG_DETERMINANT])
                    .unwrap();
                let (key, value) = range.next().unwrap().unwrap();
                assert_eq!(value, 7u64.to_be_bytes());
                let (projection, _, row) = store.inner.keys.decode_determinant(key).unwrap();
                (projection, row, key.to_vec())
            };
            {
                let mut txn = store.gated_write_txn(&work).unwrap();
                if let Some(home) = &home {
                    store.inner.data.put(&mut txn.txn, &key, home).unwrap();
                } else {
                    store.inner.data.delete(&mut txn.txn, &key).unwrap();
                }
                txn.commit().unwrap();
            }
            let snapshot = store.snapshot(&work).unwrap();
            let findings = sweep(&snapshot, &schema, &work).unwrap();
            let expected = if home.is_none() {
                VerifyCorruption::MissingDeterminant { projection, row }
            } else {
                VerifyCorruption::ForeignDeterminantHome { projection, row }
            };
            assert!(findings.contains(&corrupt(expected)), "{findings:?}");
            if home.as_ref().is_some_and(|home| home.len() == 8) {
                assert!(
                    findings.contains(&corrupt(VerifyCorruption::DanglingDeterminant {
                        projection,
                        row
                    }))
                );
            }
        }
    }
}
