use bumbledb::{RelationId, Value};

use crate::gen::{relation_rows, GenConfig};
use crate::schema::ids;

/// Canonical bytes of one value, for the corpus digest (length-prefixed
/// variable content; fixed-width scalars little-endian).
fn value_bytes(digest: &mut bumbledb::digest::Digest, value: &Value) {
    match value {
        Value::Bool(v) => digest.update(&[0, u8::from(*v)]),
        Value::U64(v) => {
            digest.update(&[1]);
            digest.update(&v.to_le_bytes());
        }
        Value::I64(v) => {
            digest.update(&[2]);
            digest.update(&v.to_le_bytes());
        }
        Value::Enum(v) => digest.update(&[3, *v]),
        Value::String(raw) => {
            digest.update(&[4]);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(raw);
        }
        Value::FixedBytes(raw) => {
            digest.update(&[5]);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(raw);
        }
        Value::IntervalU64(start, end) => {
            digest.update(&[6]);
            digest.update(&start.to_le_bytes());
            digest.update(&end.to_le_bytes());
        }
        Value::IntervalI64(start, end) => {
            digest.update(&[7]);
            digest.update(&start.to_le_bytes());
            digest.update(&end.to_le_bytes());
        }
        // Masks never appear in corpus rows (not a field type).
        Value::AllenMask(_) => unreachable!("mask values are not corpus data"),
    }
}

/// The corpus identity: a blake3 over the engine's storage format
/// version and every relation's streamed rows. Stamps, cache
/// directories, and reports key on this. The format version is a live
/// ingredient by decision: a cached corpus is a *store*, and a store
/// written under an older format must be regenerated, never reused —
/// the ALG 17 dictionary cutover left a v1 cache silently mis-decoding
/// until the two-oracle run caught it.
#[must_use]
pub fn corpus_digest(cfg: GenConfig) -> [u8; 32] {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(&bumbledb::STORAGE_FORMAT_VERSION.to_le_bytes());
    digest.update(&cfg.seed.to_le_bytes());
    digest.update(cfg.scale.label().as_bytes());
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        digest.update(&rel.0.to_le_bytes());
        for row in relation_rows(cfg, rel) {
            for value in &row {
                value_bytes(&mut digest, value);
            }
        }
    }
    digest.finalize()
}
