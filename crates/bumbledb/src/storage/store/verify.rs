//! The offline sweeper: one coherent snapshot, one pass per physical
//! database, then complete judgment. Every derivation is the engine's own
//! (keys, homes, routings, the canonical codec, the production judge).
//!
//! ```text
//! rows   key shape, known ordinary relation, canonical row, home agreement,
//!        unique ordinals, every non-home determinant entry present
//! det    known non-home projection, live row at the stored home, routing
//!        agreement
//! meta   format, schema, identity, generation, relation counts, row-id
//!        high-water mark
//! judge  every statement over the full state
//! ```

use std::collections::{BTreeMap, BTreeSet};

use bumbledb_theory::schema::RelationId;

use super::format::{
    self, FORMAT, K_DATABASE, K_FORMAT, K_GENERATION, K_NEXT_ROW_ID, K_SCHEMA, RowId,
};
use super::keys;
use super::rows;
use super::snapshot::OwnedSnapshot;
use crate::canonical::RowError;
use crate::error::{Error, Result};
use crate::schema::judge::JudgedViolation;
use crate::schema::{ProjectionId, Schema};
use crate::work::WorkContext;

/// One physical desync inside a recognized store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyCorruption {
    /// A physical key has an impossible shape.
    MalformedKey { what: &'static str },
    /// A row is stored under a relation the schema does not declare.
    UnknownRelation { relation: RelationId },
    /// A row is stored under a closed relation, whose rows live in the schema.
    ClosedRelationRow { relation: RelationId, row: u64 },
    /// A stored row is not a canonical row of its relation.
    MalformedRow {
        relation: RelationId,
        row: u64,
        error: RowError,
    },
    /// A row's key home disagrees with its recomputed home.
    ForeignRowHome { relation: RelationId, row: u64 },
    /// One ordinal is allocated twice.
    DuplicateRowId { relation: RelationId, row: u64 },
    /// A live row lacks a determinant entry under its recomputed routing.
    MissingDeterminant { projection: ProjectionId, row: u64 },
    /// A determinant entry names no live row.
    DanglingDeterminant { projection: ProjectionId, row: u64 },
    /// A determinant entry names a projection that is unknown or is a home.
    UnknownDeterminantProjection { projection: ProjectionId },
    /// A determinant entry's routing disagrees with its row's.
    ForeignDeterminant { projection: ProjectionId, row: u64 },
    /// A stored relation count disagrees with the counted rows.
    RowCountMismatch {
        relation: RelationId,
        stored: u64,
        counted: u64,
    },
    /// The next-row-id high-water mark is at or below an allocated ordinal.
    RowIdRatchetBehind { next: u64, max_seen: u64 },
    /// A required meta entry is absent, malformed, or names another format
    /// or schema.
    MetaMissing { what: &'static str },
}

/// One observed desync: physical corruption, or a statement the complete
/// judgment finds violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VerifyFinding {
    Judgment(JudgedViolation),
    Corruption(VerifyCorruption),
}

const fn corrupt(finding: VerifyCorruption) -> VerifyFinding {
    VerifyFinding::Corruption(finding)
}

/// Sweep one coherent snapshot. An empty result is coherence; storage
/// failure and cancellation are errors, never a shorter report.
pub(crate) fn sweep(
    snapshot: &OwnedSnapshot,
    schema: &Schema,
    work: &WorkContext,
) -> Result<Vec<VerifyFinding>> {
    let inner = snapshot.store_inner();
    let txn = snapshot.read_txn();
    let mut findings = Vec::new();
    let mut tallies: BTreeMap<RelationId, u64> = BTreeMap::new();
    let mut ordinals = BTreeSet::new();
    let mut max_row = 0u64;
    // Structural faults make judgment undefined; report them instead.
    let mut judgment_safe = true;

    for entry in inner.rows.iter(txn).map_err(Error::from)? {
        work.checkpoint()?;
        let (key, bytes) = entry.map_err(Error::from)?;
        let Ok(parsed) = keys::parse(key) else {
            judgment_safe = false;
            findings.push(corrupt(VerifyCorruption::MalformedKey { what: "row key" }));
            continue;
        };
        let relation = RelationId(u32::from(u16::from_be_bytes(parsed.prefix)));
        let row = parsed.row.0;
        max_row = max_row.max(row);
        if !ordinals.insert(row) {
            judgment_safe = false;
            findings.push(corrupt(VerifyCorruption::DuplicateRowId { relation, row }));
        }
        *tallies.entry(relation).or_default() += 1;
        let Some(view) = schema.relation_checked(relation) else {
            judgment_safe = false;
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
        let decoded = match crate::canonical::decode(view.fields(), bytes, work) {
            Ok(decoded) => decoded,
            Err(RowError::Work(error)) => return Err(Error::from(error)),
            Err(error) => {
                judgment_safe = false;
                findings.push(corrupt(VerifyCorruption::MalformedRow {
                    relation,
                    row,
                    error,
                }));
                continue;
            }
        };
        if rows::home_of_row(inner, relation, bytes, work)? != parsed.route {
            judgment_safe = false;
            findings.push(corrupt(VerifyCorruption::ForeignRowHome { relation, row }));
        }
        inner.det.emit_decoded(
            relation,
            decoded.values(),
            work,
            &mut |compiled, projected| {
                if inner.det.is_home(compiled) {
                    return Ok(());
                }
                let route = rows::routing(inner, compiled, projected)?;
                let key = keys::entry(keys::det_prefix(compiled.id), &route, parsed.row);
                if inner.dets.get(txn, &key).map_err(Error::from)? != Some(parsed.route.as_slice())
                {
                    findings.push(corrupt(VerifyCorruption::MissingDeterminant {
                        projection: compiled.id,
                        row,
                    }));
                }
                Ok(())
            },
        )?;
    }
    drop(ordinals);

    for entry in inner.dets.iter(txn).map_err(Error::from)? {
        work.checkpoint()?;
        let (key, home) = entry.map_err(Error::from)?;
        let Ok(parsed) = keys::parse(key) else {
            findings.push(corrupt(VerifyCorruption::MalformedKey {
                what: "determinant key",
            }));
            continue;
        };
        let projection = ProjectionId(u16::from_be_bytes(parsed.prefix));
        let row = parsed.row.0;
        let Some(compiled) = inner
            .det
            .projection(projection)
            .filter(|compiled| !inner.det.is_home(compiled))
        else {
            findings.push(corrupt(VerifyCorruption::UnknownDeterminantProjection {
                projection,
            }));
            continue;
        };
        let Ok(home) = <&keys::Route>::try_from(home) else {
            findings.push(corrupt(VerifyCorruption::MalformedKey {
                what: "determinant home",
            }));
            continue;
        };
        let Some(bytes) = rows::fetch(inner, txn, compiled.relation, home, RowId(row))? else {
            findings.push(corrupt(VerifyCorruption::DanglingDeterminant {
                projection,
                row,
            }));
            continue;
        };
        let fields = schema.relation(compiled.relation).fields();
        match crate::canonical::decode(fields, bytes, work) {
            Err(RowError::Work(error)) => return Err(Error::from(error)),
            Err(_) => {}
            Ok(decoded) => {
                let values = compiled.scalar_values(decoded.values());
                let projected = super::det_index::determinant_bytes(compiled, &values, work)?;
                if rows::routing(inner, compiled, &projected)? != parsed.route {
                    findings.push(corrupt(VerifyCorruption::ForeignDeterminant {
                        projection,
                        row,
                    }));
                }
            }
        }
    }

    let meta_entry = |key: &[u8]| inner.meta.get(txn, key).map_err(Error::from);
    if meta_entry(K_FORMAT)? != Some(FORMAT.as_slice()) {
        findings.push(corrupt(VerifyCorruption::MetaMissing { what: "format" }));
    }
    if meta_entry(K_SCHEMA)? != Some(inner.schema_fp.0.as_slice()) {
        findings.push(corrupt(VerifyCorruption::MetaMissing { what: "schema" }));
    }
    if meta_entry(K_DATABASE)?.is_none_or(|id| id.len() != 16) {
        findings.push(corrupt(VerifyCorruption::MetaMissing {
            what: "database id",
        }));
    }
    if format::read_u64(&inner.meta, txn, K_GENERATION, "generation").is_err() {
        findings.push(corrupt(VerifyCorruption::MetaMissing {
            what: "generation",
        }));
    }
    match format::read_u64(&inner.meta, txn, K_NEXT_ROW_ID, "next row id") {
        Ok(next) if !tallies.is_empty() && next <= max_row => {
            findings.push(corrupt(VerifyCorruption::RowIdRatchetBehind {
                next,
                max_seen: max_row,
            }));
        }
        Ok(_) => {}
        Err(_) => findings.push(corrupt(VerifyCorruption::MetaMissing {
            what: "next row id",
        })),
    }
    for relation in inner.det.relations() {
        work.checkpoint()?;
        let counted = tallies.remove(&relation).unwrap_or(0);
        match format::read_relation_meta(&inner.meta, txn, relation) {
            Ok(meta) if meta.count == counted => {}
            Ok(meta) => findings.push(corrupt(VerifyCorruption::RowCountMismatch {
                relation,
                stored: meta.count,
                counted,
            })),
            Err(_) => findings.push(corrupt(VerifyCorruption::MetaMissing {
                what: "relation meta",
            })),
        }
    }

    if judgment_safe
        && let Some(violations) = super::judge_bridge::judge_snapshot(schema, snapshot, work)?
    {
        findings.extend(
            violations
                .into_vec()
                .into_iter()
                .map(VerifyFinding::Judgment),
        );
    }
    Ok(findings)
}
