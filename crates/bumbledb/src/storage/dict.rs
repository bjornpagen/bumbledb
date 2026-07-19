//! The interning dictionary (docs/architecture/50-storage.md): the
//! compression representation for repeated text — **str-only**. Digest-
//! shaped values (`bytes<N>`) live inline in facts and never touch it
//! (`docs/architecture/10-data-model.md`, *intern what repeats; inline
//! what identifies*), so the key hash carries no type tag: with one
//! interned type there is nothing to segregate.
//!
//! Facts carry 8-byte intern ids; the `_dict` database holds both maps:
//!
//! ```text
//! 0x00 | blake3(raw_bytes)   -> id (u64 BE)   forward
//! 0x01 | id (u64 BE)         -> raw_bytes     reverse
//! ```
//!
//! Ids are monotonic, never reused, append-only; interning happens only
//! inside write transactions. There is no GC — deleted facts leak their
//! interned values (accepted design: the leak is scoped to repeated text,
//! the population interning compresses).

use crate::error::{CorruptionError, Error, Result};
use crate::storage::env::{ReadTxn, WriteTxn};

/// `_dict` key prefixes.
const FORWARD: u8 = 0x00;
const REVERSE: u8 = 0x01;

fn forward_key(raw: &[u8]) -> [u8; 33] {
    let mut key = [0u8; 33];
    key[0] = FORWARD;
    key[1..].copy_from_slice(blake3::hash(raw).as_bytes());
    key
}

fn reverse_key(id: u64) -> [u8; 9] {
    let mut key = [0u8; 9];
    key[0] = REVERSE;
    key[1..].copy_from_slice(&id.to_be_bytes());
    key
}

/// Interns a UTF-8 string, returning its id. The `&str` boundary *is* the
/// UTF-8 validation (parse, don't validate): a `&[u8]` string entry point
/// must not exist.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed id counter.
#[cfg(test)]
pub fn intern_str(txn: &mut WriteTxn<'_>, value: &str) -> Result<u64> {
    let raw = value.as_bytes();
    let dict = txn.env().dict();
    let fwd = forward_key(raw);
    // Collision axiom (10-data-model): a forward hit returns the existing id
    // with no byte verification — hash equality is identity, 2⁻¹²⁸-scale
    // collisions are accepted, not checked for.
    if let Some(existing) = dict.get(txn.raw(), &fwd)? {
        let id: [u8; 8] = existing
            .try_into()
            .map_err(|_| Error::Corruption(CorruptionError::MalformedValue("dict forward id")))?;
        return Ok(u64::from_be_bytes(id));
    }
    // Mint the next id. This read-modify-writes the `_meta` counter directly;
    // the 50-storage doc re-homes it into the delta's in-memory-then-flush counter set.
    // A stored u64::MAX counter is typed Corruption at the read above
    // (the sentinel is never mintable), so `id` here is always valid.
    let id = txn.dict_next_id()?;
    txn.put_dict_next_id(id + 1)?;

    dict.put(txn.raw_mut(), &fwd, id.to_be_bytes().as_slice())?;
    dict.put(txn.raw_mut(), &reverse_key(id), raw)?;
    Ok(id)
}

/// The never-minted intern id: dictionary ids allocate from 0 upward and
/// the mint paths assert this value is never issued, so read paths may
/// resolve a dictionary *miss* to it. An `Eq` filter against the sentinel
/// matches nothing; an `Ne` filter matches everything — per-operator miss
/// semantics fall out of ordinary word comparison
/// (docs/architecture/20-query-ir.md).
pub(crate) const SENTINEL_ID: u64 = u64::MAX;

/// Read-only lookup of a string's id. `None` means the value was never
/// interned — on the query path that means "cannot match any fact": an
/// empty result, never an insert, never an error.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed stored id.
pub fn lookup_str(txn: &ReadTxn<'_>, value: &str) -> Result<Option<u64>> {
    lookup(txn, value.as_bytes())
}

/// Read-only raw-bytes lookup (readers: the string front above; the
/// delta's pending-intern path, which must consult the committed
/// dictionary before minting a provisional id; the sweeper's
/// committed-only selection encoding).
pub(crate) fn lookup(txn: &ReadTxn<'_>, raw: &[u8]) -> Result<Option<u64>> {
    let dict = txn.env().dict();
    match dict.get(txn.raw(), &forward_key(raw))? {
        None => Ok(None),
        Some(bytes) => {
            let id: [u8; 8] = bytes.try_into().map_err(|_| {
                Error::Corruption(CorruptionError::MalformedValue("dict forward id"))
            })?;
            Ok(Some(u64::from_be_bytes(id)))
        }
    }
}

/// Writes one pending intern entry minted by the delta (reader: the 50-storage doc's
/// commit counter flush). The provisional id was assigned from the same
/// counter this commit flushes, under the single-writer discipline.
pub(crate) fn put_pending(txn: &mut WriteTxn<'_>, raw: &[u8], id: u64) -> Result<()> {
    let dict = txn.env().dict();
    dict.put(
        txn.raw_mut(),
        &forward_key(raw),
        id.to_be_bytes().as_slice(),
    )?;
    dict.put(txn.raw_mut(), &reverse_key(id), raw)?;
    Ok(())
}

/// One `_dict` reverse-map entry as the sweeper sees it: the minted id, or
/// the raw key bytes when the key is not the codec's 9-byte shape.
pub(crate) enum ReverseId<'k> {
    Id(u64),
    Malformed(&'k [u8]),
}

/// One cursor over the reverse map, in id order (reader: `Db::verify_store`'s
/// dangling-entry statistic — ids referenced by no live fact are the
/// accepted leak, counted, never findings).
///
/// # Errors
///
/// `Lmdb` on cursor open; per-item `Lmdb` on iteration failure.
pub(crate) fn reverse_ids<'txn>(
    txn: &'txn ReadTxn<'_>,
) -> Result<impl Iterator<Item = Result<ReverseId<'txn>>>> {
    let iter = txn.env().dict().prefix_iter(txn.raw(), &[REVERSE])?;
    Ok(iter.map(|entry| {
        let (key, _) = entry?;
        Ok(match key[1..].try_into() {
            Ok(id) => ReverseId::Id(u64::from_be_bytes(id)),
            Err(_) => ReverseId::Malformed(key),
        })
    }))
}

/// Resolves an id to its raw bytes, borrowed from the LMDB page for the
/// transaction's lifetime.
///
/// # Errors
///
/// `Corruption(DanglingInternId)` when the id has no reverse entry — a fact
/// referencing it is corrupt; never a skip.
pub fn resolve<'txn>(txn: &'txn ReadTxn<'_>, id: u64) -> Result<&'txn [u8]> {
    let dict = txn.env().dict();
    dict.get(txn.raw(), &reverse_key(id))?
        .ok_or(Error::Corruption(CorruptionError::DanglingInternId(id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;
    use crate::schema::ValidateDescriptor as _;
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

    #[test]
    fn interning_twice_returns_the_same_id() {
        let dir = TempDir::new("dict-idempotent");
        let env = env(&dir);
        let mut wtxn = env.write_txn().expect("txn");
        let first = intern_str(&mut wtxn, "hello").expect("intern");
        let second = intern_str(&mut wtxn, "hello").expect("intern");
        assert_eq!(first, second);
        wtxn.commit().expect("commit");

        // And across transactions.
        let mut wtxn = env.write_txn().expect("txn");
        assert_eq!(intern_str(&mut wtxn, "hello").expect("intern"), first);
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
        let env = env(&dir);
        let mut wtxn = env.write_txn().expect("txn");
        let s = intern_str(&mut wtxn, "posting").expect("intern");
        wtxn.commit().expect("commit");

        let rtxn = env.read_txn().expect("txn");
        assert_eq!(lookup_str(&rtxn, "posting").expect("lookup"), Some(s));
        assert_eq!(resolve(&rtxn, s).expect("resolve"), b"posting");
    }

    #[test]
    fn reverse_entries_carry_raw_bytes_with_no_tag() {
        // The contraction's shape pin: with the dictionary str-only, the
        // reverse value IS the raw bytes — no tag byte survives anywhere
        // in the codec (docs/architecture/50-storage.md).
        let dir = TempDir::new("dict-untagged");
        let env = env(&dir);
        let mut wtxn = env.write_txn().expect("txn");
        let id = intern_str(&mut wtxn, "A").expect("intern");
        wtxn.commit().expect("commit");
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(resolve(&rtxn, id).expect("resolve"), b"A");
        assert_eq!(resolve(&rtxn, id).expect("resolve").len(), 1);
    }

    #[test]
    fn resolve_of_fabricated_id_is_corruption() {
        let dir = TempDir::new("dict-dangling");
        let env = env(&dir);
        let rtxn = env.read_txn().expect("txn");
        let err = resolve(&rtxn, 12345).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::DanglingInternId(12345))
            ),
            "{err:?}"
        );
    }

    #[test]
    fn ids_strictly_increase_across_interns() {
        let dir = TempDir::new("dict-monotonic");
        let env = env(&dir);
        let mut wtxn = env.write_txn().expect("txn");
        let ids: Vec<u64> = ["a", "b", "c", "d"]
            .iter()
            .map(|s| intern_str(&mut wtxn, s).expect("intern"))
            .collect();
        wtxn.commit().expect("commit");
        let mut wtxn = env.write_txn().expect("txn");
        let e = intern_str(&mut wtxn, "e").expect("intern");
        wtxn.commit().expect("commit");
        for pair in ids.windows(2) {
            assert!(pair[0] < pair[1]);
        }
        assert!(e > ids[3]);
    }

    #[test]
    fn aborted_transaction_leaves_no_dictionary_entries() {
        let dir = TempDir::new("dict-abort");
        let env = env(&dir);
        let mut wtxn = env.write_txn().expect("txn");
        let id = intern_str(&mut wtxn, "phantom").expect("intern");
        wtxn.abort();

        let rtxn = env.read_txn().expect("txn");
        assert_eq!(lookup_str(&rtxn, "phantom").expect("lookup"), None);
        assert!(resolve(&rtxn, id).is_err());
        drop(rtxn);

        // The counter did not advance either: the next intern re-issues the
        // aborted id (aborted values never existed in any committed state).
        let mut wtxn = env.write_txn().expect("txn");
        assert_eq!(intern_str(&mut wtxn, "real").expect("intern"), id);
    }
}
