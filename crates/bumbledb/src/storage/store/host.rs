//! Host records and the head: opaque bytes a host (the log) seals into the
//! same transaction as the facts they describe. The engine never interprets
//! them; it only bounds key width and keeps keys strictly ordered.

use heed::RwTxn;

use super::format::{K_HEAD, K_HOST};
use super::keys::HOST_KEY_MAX;
use super::store_env::StoreInner;
use crate::error::{Error, HostKeyFault, Result};
use crate::work::WorkContext;

/// One host record change. Keys in one [`HostChanges`] are strictly
/// increasing and at most [`crate::host::MAX_KEY`] bytes.
#[derive(Debug, Clone, Copy)]
pub enum HostRecord<'a> {
    Put { key: &'a [u8], value: &'a [u8] },
    Delete { key: &'a [u8] },
}

/// The change to the head: the host's single opaque position record.
#[derive(Debug, Clone, Copy)]
pub enum Head<'a> {
    Keep,
    Put(&'a [u8]),
    Clear,
}

/// Everything a seal writes besides facts.
#[derive(Debug, Clone, Copy)]
pub struct HostChanges<'a> {
    pub records: &'a [HostRecord<'a>],
    pub head: Head<'a>,
}

impl HostChanges<'static> {
    /// No host records and an unchanged head.
    pub const NONE: Self = Self {
        records: &[],
        head: Head::Keep,
    };
}

/// The physical meta key of one host key.
pub(crate) fn host_key(key: &[u8], buffer: &mut [u8; 1 + HOST_KEY_MAX]) -> Result<usize> {
    if key.len() > HOST_KEY_MAX {
        return Err(Error::HostKey(HostKeyFault::TooLong { actual: key.len() }));
    }
    buffer[0] = K_HOST;
    buffer[1..=key.len()].copy_from_slice(key);
    Ok(1 + key.len())
}

fn validate(host: &HostChanges<'_>) -> Result<()> {
    let mut previous: Option<&[u8]> = None;
    for record in host.records {
        let (HostRecord::Put { key, .. } | HostRecord::Delete { key }) = *record;
        if key.len() > HOST_KEY_MAX {
            return Err(Error::HostKey(HostKeyFault::TooLong { actual: key.len() }));
        }
        if previous.is_some_and(|previous| previous >= key) {
            return Err(Error::HostKey(HostKeyFault::NotStrictlyOrdered));
        }
        previous = Some(key);
    }
    Ok(())
}

fn put(inner: &StoreInner, txn: &mut RwTxn<'_>, key: &[u8], value: &[u8]) -> Result<bool> {
    if inner.meta.get(txn, key).map_err(Error::from)? == Some(value) {
        return Ok(false);
    }
    inner
        .meta
        .put(txn, key, value)
        .map_err(|error| inner.txn_error(error))?;
    Ok(true)
}

fn delete(inner: &StoreInner, txn: &mut RwTxn<'_>, key: &[u8]) -> Result<bool> {
    inner
        .meta
        .delete(txn, key)
        .map_err(|error| inner.txn_error(error))
}

/// Apply host changes; true when any stored byte changed. The grammar is
/// checked before the first write.
pub(crate) fn apply(
    inner: &StoreInner,
    txn: &mut RwTxn<'_>,
    host: HostChanges<'_>,
    work: &WorkContext,
) -> Result<bool> {
    validate(&host)?;
    let mut buffer = [0u8; 1 + HOST_KEY_MAX];
    let mut mutated = false;
    for (index, record) in host.records.iter().enumerate() {
        work.checkpoint()?;
        #[cfg(test)]
        inner.fail_host_write(index)?;
        #[cfg(not(test))]
        let _ = index;
        mutated |= match *record {
            HostRecord::Put { key, value } => {
                let len = host_key(key, &mut buffer)?;
                put(inner, txn, &buffer[..len], value)?
            }
            HostRecord::Delete { key } => {
                let len = host_key(key, &mut buffer)?;
                delete(inner, txn, &buffer[..len])?
            }
        };
    }
    mutated |= match host.head {
        Head::Keep => false,
        Head::Put(bytes) => put(inner, txn, K_HEAD, bytes)?,
        Head::Clear => delete(inner, txn, K_HEAD)?,
    };
    Ok(mutated)
}
