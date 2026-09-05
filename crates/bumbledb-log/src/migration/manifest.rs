//! The ordered migration manifest and its acyclic prefix chain (C11).
//!
//! The manifest records the repo's ordered chain of plan identities. Prefix
//! hashing is acyclic and domain-separated: the base digest hashes a framed
//! record of the codec versions and the empty-base schema; each next prefix
//! hashes the previous prefix plus the canonical entry EXCLUDING its own
//! prefix field. A manifest is verified by recomputation, never trusted from
//! text. `plan_set_digest` identifies one exact contiguous pending suffix
//! plus its starting prefix and source/final schemas — no tenant path or
//! local directory ever enters a hash.

use crate::history::{FrameError, SchemaId};

use super::frame::{
    self, KIND_MANIFEST_BASE, KIND_MANIFEST_ENTRY, KIND_PLAN_SET, PLAN_SET_DIGEST_DOMAIN,
    PREFIX_DIGEST_DOMAIN, keyed_digest,
};
use super::json::{Json, parse_u64, push_hex, push_string, read_tree, unhex_exact};
use super::plan::{Plan, PlanError, StepLabel, canonical_plan_bytes, parse_schema_id, plan_digest};

/// One manifest entry: the identity of one recorded plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub sequence: u64,
    pub label: StepLabel,
    pub from_schema: SchemaId,
    pub to_schema: SchemaId,
    pub plan_digest: [u8; 32],
    pub prefix_digest: [u8; 32],
}

/// The ordered chain rooted at the declared empty base schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub base_schema: SchemaId,
    pub entries: Vec<ManifestEntry>,
}

/// Why a manifest refused verification. Every arm names the exact drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError {
    Json(&'static str),
    Shape(&'static str),
    Frame(FrameError),
    /// `sequence` fields are not contiguous from zero.
    SequenceGap {
        at: u64,
    },
    /// An entry's `fromSchemaId` does not chain from its predecessor.
    SchemaChainBroken {
        at: u64,
    },
    /// Two entries reuse one stable label.
    DuplicateLabel {
        at: u64,
    },
    /// A recorded prefix digest does not recompute (edited/reordered entry).
    PrefixMismatch {
        at: u64,
    },
    /// A supplied plan's canonical bytes do not hash to the recorded digest.
    PlanDigestMismatch {
        at: u64,
    },
    /// A supplied plan's sequence/label/schemas disagree with its entry.
    PlanEntryMismatch {
        at: u64,
    },
    /// The requested suffix is not a contiguous tail of the manifest.
    NotASuffix,
}

impl From<FrameError> for ManifestError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<PlanError> for ManifestError {
    fn from(error: PlanError) -> Self {
        match error {
            PlanError::Json(why) => Self::Json(why),
            PlanError::Shape(why) => Self::Shape(why),
            PlanError::Frame(frame) => Self::Frame(frame),
        }
    }
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "migration manifest: {self:?}")
    }
}

impl std::error::Error for ManifestError {}

/// The chain's base digest: codec versions plus the empty-base schema.
/// # Errors
/// Refuses only allocation/limit failure.
pub fn base_prefix_digest(base_schema: &SchemaId, cap: usize) -> Result<[u8; 32], FrameError> {
    let mut out = frame::begin(KIND_MANIFEST_BASE, cap)?;
    out.bytes(&1u16.to_be_bytes())?; // manifest version
    out.bytes(&frame::LAYOUT.to_be_bytes())?; // plan codec layout
    out.bytes(&base_schema.0)?;
    Ok(keyed_digest(PREFIX_DIGEST_DOMAIN, &out.finish()))
}

/// The framed entry record EXCLUDING its own prefix digest (self-exclusion).
fn entry_frame(entry: &ManifestEntry, cap: usize) -> Result<Vec<u8>, FrameError> {
    let mut out = frame::begin(KIND_MANIFEST_ENTRY, cap)?;
    out.u64(entry.sequence)?;
    out.span(entry.label.as_str().as_bytes())?;
    out.bytes(&entry.from_schema.0)?;
    out.bytes(&entry.to_schema.0)?;
    out.bytes(&entry.plan_digest)?;
    Ok(out.finish())
}

/// One prefix step: hash the previous prefix and the canonical entry frame.
/// # Errors
/// Refuses only allocation/limit failure.
pub fn next_prefix_digest(
    previous: &[u8; 32],
    entry: &ManifestEntry,
    cap: usize,
) -> Result<[u8; 32], FrameError> {
    let frame = entry_frame(entry, cap)?;
    let mut preimage = Vec::new();
    preimage
        .try_reserve_exact(32 + frame.len())
        .map_err(|_| FrameError::Allocation)?;
    preimage.extend_from_slice(previous);
    preimage.extend_from_slice(&frame);
    Ok(keyed_digest(PREFIX_DIGEST_DOMAIN, &preimage))
}

/// Verify a manifest completely: contiguous sequences, chained schemas,
/// unique labels and every recorded prefix digest recomputed from the base.
/// Returns the final prefix digest (the base digest for an empty manifest).
/// # Errors
/// The exact drift, before anything trusts the chain.
pub fn verify_manifest(manifest: &Manifest, cap: usize) -> Result<[u8; 32], ManifestError> {
    let mut prefix = base_prefix_digest(&manifest.base_schema, cap)?;
    let mut previous_schema = manifest.base_schema;
    for (index, entry) in manifest.entries.iter().enumerate() {
        let at = index as u64;
        if entry.sequence != at {
            return Err(ManifestError::SequenceGap { at });
        }
        if entry.from_schema != previous_schema {
            return Err(ManifestError::SchemaChainBroken { at });
        }
        if manifest.entries[..index]
            .iter()
            .any(|earlier| earlier.label == entry.label)
        {
            return Err(ManifestError::DuplicateLabel { at });
        }
        prefix = next_prefix_digest(&prefix, entry, cap)?;
        if prefix != entry.prefix_digest {
            return Err(ManifestError::PrefixMismatch { at });
        }
        previous_schema = entry.to_schema;
    }
    Ok(prefix)
}

/// The prefix digest after the first `through` entries of a VERIFIED
/// manifest (the base digest for `through == 0`).
/// # Errors
/// Refuses a `through` beyond the manifest.
pub fn prefix_at(
    manifest: &Manifest,
    through: usize,
    cap: usize,
) -> Result<[u8; 32], ManifestError> {
    if through > manifest.entries.len() {
        return Err(ManifestError::NotASuffix);
    }
    let mut prefix = base_prefix_digest(&manifest.base_schema, cap)?;
    for entry in &manifest.entries[..through] {
        prefix = next_prefix_digest(&prefix, entry, cap)?;
    }
    Ok(prefix)
}

/// Bind supplied plan bytes to their manifest entries: recompute each plan's
/// canonical digest and check sequence/label/schema agreement. `plans[i]`
/// must be entry `first + i`. Nothing here trusts a label.
/// # Errors
/// The exact mismatch position.
pub fn bind_plans(
    manifest: &Manifest,
    first: usize,
    plans: &[&Plan],
    cap: usize,
) -> Result<(), ManifestError> {
    let end = first
        .checked_add(plans.len())
        .ok_or(ManifestError::NotASuffix)?;
    if end > manifest.entries.len() {
        return Err(ManifestError::NotASuffix);
    }
    for (offset, plan) in plans.iter().enumerate() {
        let entry = &manifest.entries[first + offset];
        let at = entry.sequence;
        let bytes = canonical_plan_bytes(plan, cap)?;
        if plan_digest(&bytes) != entry.plan_digest {
            return Err(ManifestError::PlanDigestMismatch { at });
        }
        if plan.sequence != entry.sequence
            || plan.label != entry.label
            || plan.from_schema != entry.from_schema
            || plan.to_schema != entry.to_schema
        {
            return Err(ManifestError::PlanEntryMismatch { at });
        }
    }
    Ok(())
}

/// The identity of one exact ordered pending suffix: starting prefix, source
/// schema, final target schema and each entry's identity in order. This is
/// what a freeze intent and an Applied record cite.
/// # Errors
/// Refuses a suffix that is not a contiguous manifest tail region.
pub fn plan_set_digest(
    manifest: &Manifest,
    first: usize,
    count: usize,
    cap: usize,
) -> Result<[u8; 32], ManifestError> {
    let end = first.checked_add(count).ok_or(ManifestError::NotASuffix)?;
    if count == 0 || end > manifest.entries.len() {
        return Err(ManifestError::NotASuffix);
    }
    let starting_prefix = prefix_at(manifest, first, cap)?;
    let source_schema = if first == 0 {
        manifest.base_schema
    } else {
        manifest.entries[first - 1].to_schema
    };
    let target_schema = manifest.entries[end - 1].to_schema;
    let mut out = frame::begin(KIND_PLAN_SET, cap)?;
    out.bytes(&starting_prefix)?;
    out.bytes(&source_schema.0)?;
    out.bytes(&target_schema.0)?;
    out.u64(count as u64)?;
    let mut preimage = out.finish();
    for entry in &manifest.entries[first..end] {
        let frame = entry_frame(entry, cap)?;
        preimage
            .try_reserve(frame.len())
            .map_err(|_| ManifestError::Frame(FrameError::Allocation))?;
        preimage.extend_from_slice(&frame);
    }
    Ok(keyed_digest(PLAN_SET_DIGEST_DOMAIN, &preimage))
}

// ---------------------------------------------------------------------------
// Repo JSON boundary.
// ---------------------------------------------------------------------------

/// Parse `manifest.json`. The recorded base prefix digest is verified here;
/// the entry chain is verified by [`verify_manifest`].
/// # Errors
/// Grammar refusals and a wrong recorded base digest.
pub fn parse_manifest(raw: &str, cap: usize) -> Result<Manifest, ManifestError> {
    let json = read_tree(raw).map_err(ManifestError::Json)?;
    if json["manifestVersion"].as_u64() != Some(1) {
        return Err(ManifestError::Shape("unsupported manifestVersion"));
    }
    if json["planVersion"].as_u64() != Some(u64::from(frame::LAYOUT)) {
        return Err(ManifestError::Shape("unsupported planVersion"));
    }
    let base_schema = parse_schema_id(&json["baseSchemaId"])?;
    let recorded_base = unhex_exact::<32>(
        json["basePrefixDigest"]
            .as_str()
            .ok_or(ManifestError::Shape("basePrefixDigest"))?,
    )
    .map_err(ManifestError::Shape)?;
    if recorded_base != base_prefix_digest(&base_schema, cap)? {
        return Err(ManifestError::PrefixMismatch { at: 0 });
    }
    let entries = json["entries"]
        .as_array()
        .ok_or(ManifestError::Shape("entries"))?
        .iter()
        .map(parse_entry)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Manifest {
        base_schema,
        entries,
    })
}

fn parse_entry(json: &Json) -> Result<ManifestEntry, ManifestError> {
    Ok(ManifestEntry {
        sequence: parse_u64(&json["sequence"]).map_err(ManifestError::Shape)?,
        label: StepLabel::new(json["id"].as_str().ok_or(ManifestError::Shape("id"))?)?,
        from_schema: parse_schema_id(&json["fromSchemaId"])?,
        to_schema: parse_schema_id(&json["toSchemaId"])?,
        plan_digest: unhex_exact::<32>(
            json["planDigest"]
                .as_str()
                .ok_or(ManifestError::Shape("planDigest"))?,
        )
        .map_err(ManifestError::Shape)?,
        prefix_digest: unhex_exact::<32>(
            json["prefixDigest"]
                .as_str()
                .ok_or(ManifestError::Shape("prefixDigest"))?,
        )
        .map_err(ManifestError::Shape)?,
    })
}

/// Render deterministic `manifest.json`.
/// # Errors
/// Refuses only allocation/limit failure computing the base digest.
pub fn render_manifest(manifest: &Manifest, cap: usize) -> Result<String, FrameError> {
    let base = base_prefix_digest(&manifest.base_schema, cap)?;
    let mut out = String::new();
    out.push_str("{\n  \"manifestVersion\": 1,\n  \"planVersion\": 1,\n  \"baseSchemaId\": \"");
    push_hex(&mut out, &manifest.base_schema.0);
    out.push_str("\",\n  \"basePrefixDigest\": \"");
    push_hex(&mut out, &base);
    out.push_str("\",\n  \"entries\": [");
    for (index, entry) in manifest.entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("\n    {\"sequence\":\"");
        out.push_str(&entry.sequence.to_string());
        out.push_str("\",\"id\":");
        push_string(&mut out, entry.label.as_str());
        out.push_str(",\"fromSchemaId\":\"");
        push_hex(&mut out, &entry.from_schema.0);
        out.push_str("\",\"toSchemaId\":\"");
        push_hex(&mut out, &entry.to_schema.0);
        out.push_str("\",\"planDigest\":\"");
        push_hex(&mut out, &entry.plan_digest);
        out.push_str("\",\"prefixDigest\":\"");
        push_hex(&mut out, &entry.prefix_digest);
        out.push_str("\"}");
    }
    if manifest.entries.is_empty() {
        out.push_str("]\n}\n");
    } else {
        out.push_str("\n  ]\n}\n");
    }
    Ok(out)
}

/// Append one plan to a verified manifest, computing its digests. This is
/// the generation-side helper the TS generator calls through the native
/// boundary so no digest is ever computed twice in two languages.
/// # Errors
/// Chain violations and frame failures, exactly as verification would name.
pub fn append_entry(
    manifest: &mut Manifest,
    plan: &Plan,
    cap: usize,
) -> Result<ManifestEntry, ManifestError> {
    let previous_prefix = verify_manifest(manifest, cap)?;
    let expected_from = manifest
        .entries
        .last()
        .map_or(manifest.base_schema, |entry| entry.to_schema);
    let at = manifest.entries.len() as u64;
    if plan.sequence != at {
        return Err(ManifestError::SequenceGap { at });
    }
    if plan.from_schema != expected_from {
        return Err(ManifestError::SchemaChainBroken { at });
    }
    if manifest
        .entries
        .iter()
        .any(|entry| entry.label == plan.label)
    {
        return Err(ManifestError::DuplicateLabel { at });
    }
    let bytes = canonical_plan_bytes(plan, cap)?;
    let mut entry = ManifestEntry {
        sequence: plan.sequence,
        label: plan.label.clone(),
        from_schema: plan.from_schema,
        to_schema: plan.to_schema,
        plan_digest: plan_digest(&bytes),
        prefix_digest: [0; 32],
    };
    entry.prefix_digest = next_prefix_digest(&previous_prefix, &entry, cap)?;
    manifest.entries.push(entry.clone());
    Ok(entry)
}
