//! The replication equality oracle: blake3 over the canonical enumeration
//! of the logical content — every relation in declaration order, its
//! canonical rows sorted by full canonical bytes, each entry as relation id
//! (u32 BE), row length (u64 BE), row bytes. Physical row ids, fingerprint
//! buckets and LMDB page layout never enter the digest, so equal digests
//! mean identical judged content regardless of allocation history — the one
//! question this answers. Harness-tier (the [`Db::verify_store`] class): a
//! bounded number of sequential passes, off any hot path.
//!
//! `_core_meta` stays out on purpose — generation and schema fingerprint
//! are carried and verified by their own consumers.

use super::{Db, OwnedInstance, embedded_work};
use crate::error::{Error, Result};
use bumbledb_theory::schema::RelationId;

impl<S> Db<S> {
    /// # Errors
    /// Storage failure or stopped work.
    #[doc(hidden)]
    pub fn catalog_digest(&self) -> Result<[u8; 32]> {
        let work = embedded_work()?;
        let snapshot = self.store.snapshot(&work).map_err(Error::from_store)?;
        let mut digest = crate::digest::Digest::new();
        for (index, relation) in self.schema.relations().iter().enumerate() {
            if relation.body().closed_rows().is_some() {
                continue; // sealed in the schema, never stored content
            }
            let id = RelationId(u32::try_from(index).expect("sealed relation ids fit u32"));
            let mut rows: Vec<&[u8]> = Vec::new();
            let iterator = snapshot.rows(id).map_err(Error::from_store)?;
            for entry in iterator {
                let (_, bytes) = entry.map_err(Error::from_store)?;
                rows.push(bytes);
            }
            rows.sort_unstable();
            fold_relation(&mut digest, id, rows.iter().copied());
        }
        Ok(digest.finalize())
    }
}

impl<S> OwnedInstance<S> {
    /// # Errors
    /// None today; kept fallible to match the store-backed digest.
    #[doc(hidden)]
    pub fn catalog_digest(&self) -> Result<[u8; 32]> {
        let mut digest = crate::digest::Digest::new();
        for (index, relation) in self.schema().relations().iter().enumerate() {
            if relation.body().closed_rows().is_some() {
                continue;
            }
            let id = RelationId(u32::try_from(index).expect("sealed relation ids fit u32"));
            // Admitted rows are already sorted by canonical bytes.
            fold_relation(
                &mut digest,
                id,
                self.relation_rows(id).iter().map(AsRef::as_ref),
            );
        }
        Ok(digest.finalize())
    }
}

fn fold_relation<'r>(
    digest: &mut crate::digest::Digest,
    relation: RelationId,
    rows: impl Iterator<Item = &'r [u8]>,
) {
    for row in rows {
        digest.update(&relation.0.to_be_bytes());
        digest.update(&(row.len() as u64).to_be_bytes());
        digest.update(row);
    }
}
