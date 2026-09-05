//! 16-byte exact-checked local fingerprints (chapter 41 defaults).
//!
//! The persisted fingerprint is the first 16 bytes of a domain-separated
//! BLAKE3 digest. It selects candidate buckets; **full canonical bytes decide
//! equality**. Truncation does not reduce the compression work, only the
//! stored width; the AEGIS candidate is compared in the F3 probes before the
//! physical format freezes (C12), and switching requires a layout bump.
//!
//! Tests and the HASH-02 bench probe may force constant fingerprints to
//! exercise collision buckets through insert/contains/delete/judgment/
//! export; the forcing variant exists only under `cfg(test)` or the
//! bench-only `collision-probe` feature, and only at store construction —
//! no production constructor can select it (P14 request, recorded in
//! `implementation/packets/P02.md`).

use bumbledb_theory::schema::RelationId;

use crate::schema::ProjectionId;

/// The exact-checked local fingerprint width (chapter 41 default).
pub const FP_LEN: usize = 16;

const ROW_DOMAIN: &[u8] = b"bumbledb/1/row-fp";
const DETERMINANT_DOMAIN: &[u8] = b"bumbledb/1/det-fp";

/// The store's fingerprint function. Exactly one production variant; the
/// algorithm is fixed by the format family, never varied by CPU.
#[derive(Debug, Clone, Copy)]
pub enum Fingerprinter {
    Blake3,
    /// Forced-collision probe: every input maps to the same bucket. A store
    /// constructed with this cannot be reopened by a production constructor
    /// (its membership keys would not match); collision suites create and
    /// use the store within one process. Reachable only from tests and the
    /// bench-only `collision-probe` feature.
    #[cfg(any(test, feature = "collision-probe"))]
    Constant([u8; FP_LEN]),
}

impl Fingerprinter {
    pub(crate) fn row(self, relation: RelationId, row: &[u8]) -> [u8; FP_LEN] {
        match self {
            Self::Blake3 => {
                let mut digest = crate::digest::Digest::new();
                digest.update(ROW_DOMAIN);
                digest.update(&relation.0.to_be_bytes());
                digest.update(row);
                truncate(digest.finalize())
            }
            #[cfg(any(test, feature = "collision-probe"))]
            Self::Constant(fp) => fp,
        }
    }

    pub(crate) fn determinant(self, projection: ProjectionId, projected: &[u8]) -> [u8; FP_LEN] {
        match self {
            Self::Blake3 => {
                let mut digest = crate::digest::Digest::new();
                digest.update(DETERMINANT_DOMAIN);
                digest.update(&projection.0.to_be_bytes());
                digest.update(projected);
                truncate(digest.finalize())
            }
            #[cfg(any(test, feature = "collision-probe"))]
            Self::Constant(fp) => fp,
        }
    }
}

fn truncate(digest: [u8; 32]) -> [u8; FP_LEN] {
    let mut fp = [0u8; FP_LEN];
    fp.copy_from_slice(&digest[..FP_LEN]);
    fp
}
