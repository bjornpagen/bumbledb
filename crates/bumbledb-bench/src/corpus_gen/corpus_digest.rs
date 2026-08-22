use bumbledb::{RelationId, Value};

use crate::corpus_gen::{GenConfig, relation_rows};
use crate::schema::ids;

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
        Value::String(raw) => {
            digest.update(&[4]);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(raw.as_bytes());
        }
        Value::FixedBytes(raw) => {
            digest.update(&[5]);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(raw);
        }
        Value::IntervalU64(interval) => {
            digest.update(&[6]);
            digest.update(&interval.start().to_le_bytes());
            digest.update(&interval.end().to_le_bytes());
        }
        Value::IntervalI64(interval) => {
            digest.update(&[7]);
            digest.update(&interval.start().to_le_bytes());
            digest.update(&interval.end().to_le_bytes());
        }
    }
}

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
    for rel in 0..crate::calendar::ids::RELATIONS {
        let rel = RelationId(rel);
        digest.update(b"cal");
        digest.update(&rel.0.to_le_bytes());
        for row in crate::calendar::corpus_gen::relation_rows(cfg, rel) {
            for value in &row {
                value_bytes(&mut digest, value);
            }
        }
    }
    digest.finalize()
}
