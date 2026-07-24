//! The exhume entry — the read-only, theory-less open
//! (`docs/architecture/70-api.md` § exhume): a self-describing store
//! (`docs/architecture/50-storage.md` § the `_meta` block) opens FROM its
//! persisted canonical descriptor, with no caller-supplied schema
//! anywhere. The sighting: reading a run store whose creating theory has
//! since evolved — the record outlives the schema, and exhume is how the
//! record is read back for ETL into a successor store.

use std::path::Path;

use crate::error::{CorruptionError, Error, Result};
use crate::schema::fingerprint::{
    SchemaFingerprint, canonical_descriptor, fingerprint_of_descriptor,
};
use crate::schema::{SchemaDescriptor, ValidateDescriptor as _, descriptor_codec};
use crate::storage::env::{Environment, StoreKind};

use super::{Db, Snapshot};

/// A store opened from its own persisted description: the declared
/// schema decoded back out of the store, and the read surface over its
/// facts — nothing else. No write surface exists on this type, no
/// prepared-query entry, and no statement is ever judged: an exhumed
/// handle reads the record verbatim (scans and point reads through
/// [`Exhumed::read`]'s [`Snapshot`]), and never takes the writer path.
pub struct Exhumed {
    /// The full engine handle, PRIVATE by design: holding it here (rather
    /// than its parts) reuses the snapshot/scan/decode machinery
    /// verbatim while exposing none of the write or prepare surface.
    db: Db<SchemaDescriptor>,
    /// The schema as declared, reconstructed from the persisted bytes —
    /// relation names, field names and types, closed-relation rosters.
    descriptor: SchemaDescriptor,
    fingerprint: SchemaFingerprint,
    kind: StoreKind,
}

/// Opens a store FROM ITS PERSISTED DESCRIPTOR — no theory in scope
/// (`docs/architecture/70-api.md` § exhume). A crate-root function rather
/// than a `Db` constructor because `Db<S>`'s whole typestate is a theory
/// and this entry's whole point is having none.
///
/// The open sequence: format version, store-kind marker (validated,
/// never compared — exhume reads both kinds), then the persisted
/// descriptor with its two integrity gates — blake3 of the stored bytes
/// must equal the stored fingerprint, and the decoded declaration must
/// validate and re-encode to the exact stored bytes (the self-verifying
/// round trip: a decoder drift can never silently misread a store).
///
/// # Errors
///
/// `Io` on a nonexistent path; `FormatMismatch` and the `_meta`
/// `Corruption` refusals exactly as `Db::open` raises them — but never
/// `EnvironmentLocked`: the lock law is a writer law (R17), and this
/// read-only lane takes none; [`Error::DescriptorMissing`] on a store not yet adopted (the
/// remedy: one `Db::open` under the creating schema);
/// `Corruption(DescriptorFingerprintDesync)` when the stored descriptor
/// hashes to something other than the stored fingerprint;
/// `Corruption(MalformedValue)` on undecodable descriptor bytes; the
/// typed `SchemaError` if the decoded declaration fails validation.
pub fn exhume(path: &Path) -> Result<Exhumed> {
    let parts = Environment::exhume(path)?;
    let descriptor_hash = fingerprint_of_descriptor(&parts.descriptor);
    if descriptor_hash.0 != parts.fingerprint {
        return Err(Error::Corruption(
            CorruptionError::DescriptorFingerprintDesync {
                fingerprint: parts.fingerprint,
                descriptor_hash: descriptor_hash.0,
            },
        ));
    }
    let declared = descriptor_codec::decode_descriptor(&parts.descriptor)
        .map_err(|what| Error::Corruption(CorruptionError::MalformedValue(what)))?;
    let schema = declared.clone().validate()?;
    // The round-trip pin: the validated schema must re-encode to the
    // exact persisted bytes. Hash equality already proved the BYTES
    // authentic; this proves the DECODE faithful — together they make
    // "the exhumed schema is the creating schema" a checked fact, never
    // an assumption.
    if canonical_descriptor(&schema) != parts.descriptor {
        return Err(Error::Corruption(CorruptionError::MalformedValue(
            "descriptor round trip",
        )));
    }
    Ok(Exhumed {
        db: Db::assemble(parts.env, schema),
        descriptor: declared,
        fingerprint: descriptor_hash,
        kind: parts.kind,
    })
}

impl Exhumed {
    /// The schema as declared — relation names, field names, field
    /// types, `fresh` marks, and closed-relation rosters (each ground
    /// axiom's handle and values), exactly what the store's creator
    /// declared. Scan rows come back in this descriptor's field
    /// declaration order, so pairing a row's positions against a
    /// relation's field names here is the name-keyed reading.
    #[must_use]
    pub fn descriptor(&self) -> &SchemaDescriptor {
        &self.descriptor
    }

    /// The store's schema fingerprint — blake3 of the persisted
    /// descriptor bytes, verified against the stored `_meta` fingerprint
    /// at exhume.
    #[must_use]
    pub fn fingerprint(&self) -> SchemaFingerprint {
        self.fingerprint
    }

    /// The store's on-disk kind marker. Exhume reads both kinds — it
    /// takes no durability decision — so the kind is reported, never
    /// judged.
    #[must_use]
    pub fn kind(&self) -> StoreKind {
        self.kind
    }

    /// Resolves a relation NAME to its id — declaration order mints
    /// every id, so the position in [`Exhumed::descriptor`] IS the id.
    /// The scan-by-name reading: `exhumed.relation(name)` then
    /// `snap.scan(id)` inside [`Exhumed::read`].
    #[must_use]
    pub fn relation(&self, name: &str) -> Option<bumbledb_theory::schema::RelationId> {
        self.descriptor
            .relations
            .iter()
            .position(|relation| relation.name.as_ref() == name)
            .and_then(|index| u32::try_from(index).ok())
            .map(bumbledb_theory::schema::RelationId)
    }

    /// Runs `f` over one read snapshot of the exhumed store — the same
    /// consistent-generation contract as `Db::read`, exposing the
    /// snapshot's read surface only: `scan` (the F-namespace row-major
    /// walk, decoding per the descriptor — str through `_dict`, closed
    /// relations from their sealed rosters), `contains_dyn`, `get_dyn`.
    /// No write path is reachable from this type, and the writer lock is
    /// never taken (readers-don't-block,
    /// `docs/architecture/50-storage.md`).
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot open; otherwise whatever `f` returns.
    pub fn read<R>(
        &self,
        f: impl FnOnce(&Snapshot<'_, SchemaDescriptor>) -> Result<R>,
    ) -> Result<R> {
        self.db.read(f)
    }
}

#[cfg(test)]
mod tests;
