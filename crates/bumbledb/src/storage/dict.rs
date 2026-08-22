//! The interning dictionary: the
//! compression representation for repeated text — **str-only**. Digest-
//! shaped values (`bytes<N>`) live inline in facts and never touch it, so the key hash carries no type tag: with one
//! interned type there is nothing to segregate.
//! Facts carry 8-byte intern ids; the `_dict` database holds both maps:
//! ```text
//! 0x00 | blake3(raw_bytes)   -> id (u64 BE)   forward
//! 0x01 | id (u64 BE)         -> raw_bytes     reverse
//! ```
//! Ids are monotonic, never reused, append-only; interning happens only
//! interned values (accepted design: the leak is scoped to repeated text,
//! the population interning compresses).

use crate::encoding::InternId;
use crate::error::{CorruptionError, Error, Result};
use crate::storage::catalog::{LmdbReadCatalog, LmdbWriteCatalog};
use crate::storage::env::{ReadTxn, WriteTxn};

pub(crate) const FORWARD: u8 = 0x00;
pub(crate) const REVERSE: u8 = 0x01;

pub(crate) fn forward_key(raw: &[u8]) -> [u8; 33] {
    forward_key_from_hash(blake3::hash(raw).as_bytes())
}

pub(crate) fn forward_key_from_hash(hash: &[u8; 32]) -> [u8; 33] {
    let mut key = [0u8; 33];
    key[0] = FORWARD;
    key[1..].copy_from_slice(hash);
    key
}

pub(crate) fn reverse_key(id: InternId) -> [u8; 9] {
    let mut key = [0u8; 9];
    key[0] = REVERSE;
    key[1..].copy_from_slice(&id.raw().to_be_bytes());
    key
}

pub(crate) fn intern_id_from_stored(bytes: &[u8]) -> Result<InternId> {
    let id: [u8; 8] = bytes
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue("dict forward id")))?;
    Ok(InternId::from_raw(u64::from_be_bytes(id)))
}

pub(crate) const SENTINEL_ID: u64 = InternId::SENTINEL.raw();

/// Read-only lookup of a string's id. `None` means the value was never
/// interned — on the query path that means "cannot match any fact": an
/// empty result, never an insert, never an error.
/// # Errors
/// `Lmdb` on storage failure, `Corruption` on a malformed stored id.
#[cfg(test)]
pub fn lookup_str(txn: &ReadTxn<'_>, value: &str) -> Result<Option<InternId>> {
    lookup(txn, value.as_bytes())
}

pub(crate) fn lookup(txn: &ReadTxn<'_>, raw: &[u8]) -> Result<Option<InternId>> {
    lookup_by_hash(txn, blake3::hash(raw).as_bytes())
}

pub(crate) fn lookup_by_hash(txn: &ReadTxn<'_>, hash: &[u8; 32]) -> Result<Option<InternId>> {
    match txn
        .env()
        .dict()
        .get(txn.raw(), &forward_key_from_hash(hash))?
    {
        None => Ok(None),
        Some(bytes) => intern_id_from_stored(bytes).map(Some),
    }
}

pub(crate) fn put_pending(txn: &mut WriteTxn<'_>, raw: &[u8], id: InternId) -> Result<()> {
    LmdbWriteCatalog::new(txn).dict_put_pending(raw, id)
}

/// Thin delegate of [`LmdbReadCatalog::dict_resolve`]: raw bytes borrowed
/// from the LMDB page for the transaction's lifetime.
/// # Errors
/// `Corruption(DanglingInternId)` when the id has no reverse entry — a fact
/// referencing it is corrupt; never a skip.
pub fn resolve<'txn>(txn: &'txn ReadTxn<'_>, id: InternId) -> Result<&'txn [u8]> {
    LmdbReadCatalog::new(txn).dict_resolve(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;
    use crate::schema::ValidateDescriptor as _;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use bumbledb_theory::schema::SchemaDescriptor;

    fn empty_schema() -> Schema {
        SchemaDescriptor {
            relations: vec![],
            statements: vec![],
        }
        .validate()
        .expect("valid fixture")
    }

    fn env(dir: &TempDir) -> Environment {
        Environment::create(dir.path(), &empty_schema()).expect("create")
    }

    fn seed(env: &Environment, schema: &Schema, values: &[&str]) -> Vec<InternId> {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        let ids: Vec<InternId> = values
            .iter()
            .map(|value| delta.intern_str(&view, value).expect("intern"))
            .collect();
        drop(view);
        let mut wtxn = env.write_txn().expect("txn");
        if let Some(interns) = delta.interns() {
            for (raw, id) in interns.entries() {
                put_pending(&mut wtxn, raw, id).expect("flush pending intern");
            }
            wtxn.put_dict_next_id(interns.next_id())
                .expect("advance next-id");
        }
        wtxn.commit().expect("commit");
        ids
    }

    #[test]
    fn interning_twice_returns_the_same_id() {
        let dir = TempDir::new("dict-idempotent");
        let schema = empty_schema();
        let env = env(&dir);

        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let first = delta.intern_str(&view, "hello").expect("intern");
        assert_eq!(delta.intern_str(&view, "hello").expect("intern"), first);
        drop(view);
        drop(delta);
        // Across commits: the committed forward map answers before any mint.
        let ids = seed(&env, &schema, &["hello"]);
        assert_eq!(ids, vec![first]);
        let view = env.read_txn().expect("txn");
        let mut later = WriteDelta::new(&schema);
        assert_eq!(later.intern_str(&view, "hello").expect("intern"), first);
        assert_eq!(later.dict_next(), None, "a committed hit mints nothing");
    }

    #[test]
    fn lookup_of_never_interned_value_is_none() {
        let dir = TempDir::new("dict-lookup-miss");
        let env = env(&dir);
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(lookup_str(&rtxn, "ghost").expect("lookup"), None);
    }

    #[test]
    fn resolve_round_trips_interned_values() {
        let dir = TempDir::new("dict-resolve");
        let schema = empty_schema();
        let env = env(&dir);
        let s = seed(&env, &schema, &["posting"])[0];

        let rtxn = env.read_txn().expect("txn");
        assert_eq!(lookup_str(&rtxn, "posting").expect("lookup"), Some(s));
        assert_eq!(resolve(&rtxn, s).expect("resolve"), b"posting");
    }

    #[test]
    fn reverse_entries_carry_raw_bytes_with_no_tag() {
        let dir = TempDir::new("dict-untagged");
        let schema = empty_schema();
        let env = env(&dir);
        let id = seed(&env, &schema, &["A"])[0];
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(resolve(&rtxn, id).expect("resolve"), b"A");
        assert_eq!(resolve(&rtxn, id).expect("resolve").len(), 1);
    }

    #[test]
    fn resolve_of_fabricated_id_is_corruption() {
        let dir = TempDir::new("dict-dangling");
        let env = env(&dir);
        let rtxn = env.read_txn().expect("txn");
        let err = resolve(&rtxn, InternId::from_raw(12345)).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::DanglingInternId(id))
                    if id == InternId::from_raw(12345)
            ),
            "{err:?}"
        );
    }

    #[test]
    fn ids_strictly_increase_across_interns() {
        let dir = TempDir::new("dict-monotonic");
        let schema = empty_schema();
        let env = env(&dir);
        let ids = seed(&env, &schema, &["a", "b", "c", "d"]);
        let e = seed(&env, &schema, &["e"])[0];
        for pair in ids.windows(2) {
            assert!(pair[0] < pair[1]);
        }
        assert!(e > ids[3]);
    }

    #[test]
    fn a_dropped_delta_leaves_no_dictionary_entries() {
        // with it — LMDB never saw them, and the counter never advanced,

        let dir = TempDir::new("dict-abort");
        let schema = empty_schema();
        let env = env(&dir);
        let id = {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta.intern_str(&view, "phantom").expect("intern")
        };

        let rtxn = env.read_txn().expect("txn");
        assert_eq!(lookup_str(&rtxn, "phantom").expect("lookup"), None);
        assert!(resolve(&rtxn, id).is_err());
        drop(rtxn);

        assert_eq!(seed(&env, &schema, &["real"]), vec![id]);
    }
}
