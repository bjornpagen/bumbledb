use super::{VerifyConfig, binary_fingerprint};

use crate::{corpus_gen, families};

/// The stamp value for a config: hex blake3 over the running binary's
/// fingerprint, the corpus digest (both theories — the calendar rows
/// are inside), the ledger and calendar family-list digests, the
/// randomized-case count, and the seed. Any ingredient change — any
/// rebuild — invalidates every stored stamp.
#[must_use]
pub fn stamp_value(cfg: &VerifyConfig) -> String {
    stamp_value_with(cfg, &binary_fingerprint())
}

/// [`stamp_value`] with an explicit binary fingerprint — the test seam
/// proving the fingerprint is a live ingredient.
pub(super) fn stamp_value_with(cfg: &VerifyConfig, fingerprint: &[u8; 32]) -> String {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(fingerprint);
    digest.update(&corpus_gen::corpus_digest(cfg.corpus_gen));
    digest.update(&families::digest());
    digest.update(&crate::calendar::families::digest());
    digest.update(&cfg.random_cases.to_le_bytes());
    digest.update(&cfg.corpus_gen.seed.to_le_bytes());
    corpus_gen::digest_hex(&digest.finalize())
}
