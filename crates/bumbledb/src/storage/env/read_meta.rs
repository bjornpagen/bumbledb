use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn};

use crate::error::{CorruptionError, Error, Result};

pub(super) fn read_u64(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
    key: &[u8],
) -> Result<u64> {
    let bytes: [u8; 8] = meta
        .get(rtxn, key)?
        .and_then(|b| b.try_into().ok())
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    Ok(u64::from_le_bytes(bytes))
}

pub(super) fn read_u32(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
    key: &[u8],
) -> Result<u32> {
    let bytes: [u8; 4] = meta
        .get(rtxn, key)?
        .and_then(|b| b.try_into().ok())
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    Ok(u32::from_le_bytes(bytes))
}

/// The `_meta` store-kind marker, decoded with the absent/undecodable
/// distinction the taxonomy draws: a missing key is
/// [`CorruptionError::MetaMissing`]; a present key whose value is the
/// wrong width or an unknown byte is
/// [`CorruptionError::StoreKindInvalid`] — corrupt data, not a missing
/// key. Shared by the durable open path ([`super::Environment::open`]
/// via `verify_and_open`) and the ephemeral constructor's non-mutating
/// probe ([`super::Environment::ephemeral`]).
pub(super) fn read_store_kind(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<super::StoreKind> {
    let bytes = meta
        .get(rtxn, super::META_STORE_KIND)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    <[u8; 1]>::try_from(bytes)
        .ok()
        .and_then(|[byte]| super::StoreKind::from_meta_byte(byte))
        .ok_or(Error::Corruption(CorruptionError::StoreKindInvalid))
}

/// The dictionary next-id counter, sentinel-checked once for every
/// reader: a stored `u64::MAX` — the miss sentinel, never mintable — is
/// corrupt data, typed.
pub(super) fn read_dict_next_id(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<u64> {
    let next = read_u64(meta, rtxn, super::META_DICT_NEXT_ID)?;
    if next == u64::MAX {
        return Err(Error::Corruption(CorruptionError::MalformedValue(
            "dict next id",
        )));
    }
    Ok(next)
}
