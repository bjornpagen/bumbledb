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
//!             canonical validity, membership backing with the exact
//!             fingerprint, per-relation tallies and the max row id
//! membership  resolves to a live row whose recomputed fingerprint is the
//!             stored bucket
//! determinant resolves to a live row of the statement's relation
//! meta        family/layout/schema identity, generation presence, stored
//!             row counts against the tallies, the next-row-id ratchet,
//!             host-record key bounds
//! judgment    every sealed statement re-judged over the full final state
//! ```

use bumbledb_theory::schema::{RelationId, StatementId};

use super::error::{StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::{
    self, FAMILY, K_FAMILY, K_GENERATION, K_HOST_RECORD_TAG, K_LAYOUT, K_NEXT_ROW_ID,
    K_ROW_COUNT_TAG, K_SCHEMA, K_STORE_ID, LAYOUT, RowId,
};
use super::keys;
use super::rows;
use super::snapshot::OwnedSnapshot;
use crate::canonical::RowError;
use crate::schema::Schema;
use crate::schema::judge::{
    CandidateFacts, JudgeBudget, JudgeError, JudgedViolation, Judgment, judge_final_state,
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
    /// A live row has no membership entry under its exact fingerprint.
    MissingMembership { relation: RelationId, row: RowId },
    /// A membership entry references a row that does not exist.
    DanglingMembership { relation: RelationId, row: RowId },
    /// A membership entry's bucket disagrees with the row's recomputed
    /// fingerprint.
    ForeignMembership { relation: RelationId, row: RowId },
    /// A determinant entry references no live row of its statement's
    /// relation.
    DanglingDeterminant { statement: StatementId, row: RowId },
    /// A determinant entry names a statement the schema does not seal as a
    /// key statement.
    UnknownDeterminantStatement { statement: StatementId },
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
    let inner = snapshot.store_inner();
    let txn = snapshot.read_txn();

    // Pass 1: rows.
    {
        let prefix = [keys::TAG_ROW];
        let range = inner
            .data
            .prefix_iter(txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, row_bytes) = entry.map_err(StoreError::from_heed)?;
            if key.len() != keys::ROW_KEY_LEN {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "row key width",
                }));
                continue;
            }
            let relation = RelationId(u32::from_be_bytes(
                key[1..5].try_into().expect("checked width"),
            ));
            let row = RowId(u64::from_be_bytes(
                key[5..13].try_into().expect("checked width"),
            ));
            max_row_id = max_row_id.max(row.0);
            *tallies.entry(relation).or_default() += 1;
            let Some(view) = schema.relation_checked(relation) else {
                findings.push(corrupt(VerifyCorruption::UnknownRelation { relation }));
                continue;
            };
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
                        findings.push(corrupt(VerifyCorruption::MalformedRow {
                            relation,
                            row,
                            error: other,
                        }));
                        continue;
                    }
                }
            }
            let fp = inner.fingerprinter.row(relation, row_bytes);
            let membership = keys::membership_key(relation, &fp, row);
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
    }

    // Pass 2: membership entries resolve back, fingerprint-verified.
    {
        let prefix = [keys::TAG_MEMBERSHIP];
        let range = inner
            .data
            .prefix_iter(txn, prefix.as_slice())
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.step(1)?;
            let (key, _) = entry.map_err(StoreError::from_heed)?;
            if key.len() != keys::MEMBERSHIP_KEY_LEN {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "membership key width",
                }));
                continue;
            }
            let relation = RelationId(u32::from_be_bytes(
                key[1..5].try_into().expect("checked width"),
            ));
            let mut fp = [0u8; FP_LEN];
            fp.copy_from_slice(&key[5..5 + FP_LEN]);
            let row = keys::row_id_from_suffix(key, keys::MEMBERSHIP_KEY_LEN)?;
            match rows::fetch_row(inner, txn, relation, row)? {
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
            let (key, _) = entry.map_err(StoreError::from_heed)?;
            if key.len() != keys::DETERMINANT_KEY_LEN {
                findings.push(corrupt(VerifyCorruption::MalformedKey {
                    what: "determinant key width",
                }));
                continue;
            }
            let statement = StatementId(u16::from_be_bytes(
                key[1..3].try_into().expect("checked width"),
            ));
            let row = keys::row_id_from_suffix(key, keys::DETERMINANT_KEY_LEN)?;
            let Some(crate::schema::StatementView::Key(_, sealed)) =
                schema.statement_checked(statement)
            else {
                findings.push(corrupt(VerifyCorruption::UnknownDeterminantStatement {
                    statement,
                }));
                continue;
            };
            if rows::fetch_row(inner, txn, sealed.relation, row)?.is_none() {
                findings.push(corrupt(VerifyCorruption::DanglingDeterminant {
                    statement,
                    row,
                }));
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

    // Pass 5: the complete production judgment over the committed state.
    let facts = SnapshotFacts {
        snapshot,
        schema,
        work,
    };
    match judge_final_state(schema, &facts, work, JudgeBudget::default()) {
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

    fn rows(
        &self,
        relation: RelationId,
    ) -> Box<dyn Iterator<Item = Result<Box<[crate::Value]>, Self::Error>> + '_> {
        let Some(view) = self.schema.relation_checked(relation) else {
            // Unknown relations were already reported as corruption; the
            // judge only asks for sealed relations, so this is unreachable
            // in practice and refuses loudly if reached.
            return Box::new(std::iter::once(Err(StoreError::Corruption(
                super::error::StoreCorruption::MalformedKey("judged relation unknown to schema"),
            ))));
        };
        let fields = view.fields();
        match self.snapshot.rows(relation) {
            Err(error) => Box::new(std::iter::once(Err(error))),
            Ok(iterator) => Box::new(iterator.map(move |entry| {
                let (_, bytes) = entry?;
                let decoded = crate::canonical::decode(fields, bytes, self.work)?;
                Ok(decoded.values.into_boxed_slice())
            })),
        }
    }
}
