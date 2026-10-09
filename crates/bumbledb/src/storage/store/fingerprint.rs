//! 16-byte fingerprints: the first 16 bytes of a domain-separated BLAKE3
//! digest. A fingerprint selects a bucket; full canonical bytes decide
//! equality. Tests force a constant fingerprint to drive collision buckets.

use bumbledb_theory::schema::RelationId;

use crate::schema::ProjectionId;

pub(crate) const FP_LEN: usize = 16;

const ROW_DOMAIN: &[u8] = b"bdb.row-fp.v1";
const DETERMINANT_DOMAIN: &[u8] = b"bdb.det-fp.v1";

#[derive(Debug, Clone, Copy)]
pub(crate) enum Fingerprinter {
    Blake3,
    /// Every input maps to one bucket.
    #[cfg(test)]
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
            #[cfg(test)]
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
            #[cfg(test)]
            Self::Constant(fp) => fp,
        }
    }
}

fn truncate(digest: [u8; 32]) -> [u8; FP_LEN] {
    let mut fp = [0u8; FP_LEN];
    fp.copy_from_slice(&digest[..FP_LEN]);
    fp
}
