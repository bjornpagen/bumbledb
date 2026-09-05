//! Opaque transaction adjunct for native wrappers. Core does not interpret a
//! receipt, request, history stamp, or migration record. Binary prefixes are
//! disjoint from core `_meta` keys 0..=3. This is a transitional substrate in
//! the existing store, not the successor storage-format freeze.

use crate::error::{Error, Result};
use crate::storage::keys::{KeyBuf, MAX_KEY};
use crate::work::{ByteKind, WorkContext, WorkError};

use super::{ReadTxn, WriteTxn};

const RECORD_PREFIX: &[u8] = &[0x80, 0];
const ATTACHMENT_KEY: &[u8] = &[0x80, 1];
// Bound application byte-copy/comparison work between cooperative polls.
// LMDB's page allocation and filesystem calls remain native safe-point gaps.
const BYTE_QUANTUM: usize = 4096;
pub(crate) const MAX_HOST_KEY: usize = MAX_KEY - RECORD_PREFIX.len();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostSealError {
    KeyTooLong { actual: usize, limit: usize },
    KeysNotStrictlyOrdered,
    LengthOverflow,
    GenerationExhausted,
    Work(WorkError),
    Storage(Error),
}

impl From<Error> for HostSealError {
    fn from(error: Error) -> Self {
        Self::Storage(error)
    }
}

impl From<WorkError> for HostSealError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl std::fmt::Display for HostSealError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyTooLong { actual, limit } => {
                write!(f, "host key has {actual} bytes; limit is {limit}")
            }
            Self::KeysNotStrictlyOrdered => {
                f.write_str("host keys must be strictly ordered and unique")
            }
            Self::LengthOverflow => f.write_str("host record byte length overflow"),
            Self::GenerationExhausted => f.write_str("core generation exhausted"),
            Self::Work(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for HostSealError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Work(error) => Some(error),
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

/// Keys are strictly increasing; a key occurs once. The caller owns these
/// bytes through seal completion. Nothing retains caller memory afterward.
#[derive(Debug, Clone, Copy)]
pub enum HostRecordChange<'a> {
    Put { key: &'a [u8], value: &'a [u8] },
    Delete { key: &'a [u8] },
}

#[derive(Debug, Clone, Copy)]
pub enum AttachmentChange<'a> {
    Keep,
    Put(&'a [u8]),
    Clear,
}

#[derive(Debug, Clone, Copy)]
pub struct HostChanges<'a> {
    pub records: &'a [HostRecordChange<'a>],
    pub attachment: AttachmentChange<'a>,
}

impl HostChanges<'_> {
    fn validate(self, work: &WorkContext) -> std::result::Result<(), HostSealError> {
        work.checkpoint()?;
        let mut previous: Option<&[u8]> = None;
        for record in self.records {
            work.step(1)?;
            let (key, value_bytes) = match *record {
                HostRecordChange::Put { key, value } => (key, value.len()),
                HostRecordChange::Delete { key } => (key, 0),
            };
            if key.len() > MAX_HOST_KEY {
                return Err(HostSealError::KeyTooLong {
                    actual: key.len(),
                    limit: MAX_HOST_KEY,
                });
            }
            if previous.is_some_and(|previous| previous >= key) {
                return Err(HostSealError::KeysNotStrictlyOrdered);
            }
            let bytes = key
                .len()
                .checked_add(value_bytes)
                .ok_or(HostSealError::LengthOverflow)?;
            work.input(u64::try_from(bytes).map_err(|_| HostSealError::LengthOverflow)?)?;
            previous = Some(key);
        }
        if let AttachmentChange::Put(bytes) = self.attachment {
            work.input(u64::try_from(bytes.len()).map_err(|_| HostSealError::LengthOverflow)?)?;
        }
        Ok(())
    }
}

fn record_key<'a>(
    key: &[u8],
    buffer: &'a mut KeyBuf,
) -> std::result::Result<&'a [u8], HostSealError> {
    if key.len() > MAX_HOST_KEY {
        return Err(HostSealError::KeyTooLong {
            actual: key.len(),
            limit: MAX_HOST_KEY,
        });
    }
    let end = RECORD_PREFIX.len() + key.len();
    buffer[..RECORD_PREFIX.len()].copy_from_slice(RECORD_PREFIX);
    buffer[RECORD_PREFIX.len()..end].copy_from_slice(key);
    Ok(&buffer[..end])
}

fn same_value(
    existing: Option<&[u8]>,
    proposed: &[u8],
    work: &WorkContext,
) -> std::result::Result<bool, HostSealError> {
    let Some(existing) = existing.filter(|value| value.len() == proposed.len()) else {
        return Ok(false);
    };
    for (left, right) in existing
        .chunks(BYTE_QUANTUM)
        .zip(proposed.chunks(BYTE_QUANTUM))
    {
        work.step(left.len() as u64)?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}

impl ReadTxn<'_> {
    /// Borrow a host row from exactly this fact/generation snapshot. Owned
    /// snapshot adapters must budget and copy before ending this read guard.
    pub(crate) fn host_record(
        &self,
        key: &[u8],
    ) -> std::result::Result<Option<&[u8]>, HostSealError> {
        let mut buffer = [0; MAX_KEY];
        Ok(self
            .env
            .meta
            .get(self.raw(), record_key(key, &mut buffer)?)
            .map_err(Error::from)?)
    }

    pub(crate) fn host_attachment(&self) -> Result<Option<&[u8]>> {
        Ok(self.env.meta.get(self.raw(), ATTACHMENT_KEY)?)
    }
}

impl WriteTxn<'_> {
    // A failed reserved write may leave a partial private value. This helper is
    // called only from consuming seal: any error drops the entire transaction,
    // never exposing or committing the partially initialized reservation.
    fn put_host_value(
        &mut self,
        key: &[u8],
        value: &[u8],
        work: &WorkContext,
    ) -> std::result::Result<(), HostSealError> {
        use std::io::Write as _;

        work.checkpoint()?;
        let mut stopped = None;
        let result = self
            .env
            .meta
            .put_reserved(&mut self.txn, key, value.len(), |space| {
                for chunk in value.chunks(BYTE_QUANTUM) {
                    work.step(chunk.len() as u64).map_err(|error| {
                        stopped = Some(error);
                        // The typed cause stays in `stopped`; constructing a
                        // bare kind avoids allocating an error box on refusal.
                        std::io::Error::from(std::io::ErrorKind::Interrupted)
                    })?;
                    space.write_all(chunk)?;
                }
                Ok(())
            });
        // The storage callback uses io::Result; preserve resource/cancellation
        // classification instead of reporting it as a generic storage failure.
        if let Some(error) = stopped {
            return Err(HostSealError::Work(error));
        }
        result.map_err(Error::from)?;
        Ok(())
    }

    /// Only prepared-write sealing calls this. Any error consumes/aborts that
    /// prepared capability, including errors after a prefix of host writes.
    pub(crate) fn apply_host_changes(
        &mut self,
        changes: HostChanges<'_>,
        work: &WorkContext,
    ) -> std::result::Result<bool, HostSealError> {
        changes.validate(work)?;
        let _scratch = work.reserve(ByteKind::Working, MAX_KEY as u64)?;
        let mut key_buffer = [0; MAX_KEY];
        let mut mutated = false;
        for (index, record) in changes.records.iter().enumerate() {
            work.step(1)?;
            #[cfg(not(test))]
            let _ = index;
            #[cfg(test)]
            if self
                .env
                .fail_host_after
                .lock()
                .expect("host fault mutex")
                .as_ref()
                == Some(&index)
            {
                return Err(HostSealError::Storage(Error::Lmdb(
                    crate::error::LmdbFailure::Mdb(heed::MdbError::MapFull),
                )));
            }
            match *record {
                HostRecordChange::Put { key, value } => {
                    let key = record_key(key, &mut key_buffer)?;
                    let existing = self.env.meta.get(&self.txn, key).map_err(Error::from)?;
                    if !same_value(existing, value, work)? {
                        self.put_host_value(key, value, work)?;
                        mutated = true;
                    }
                }
                HostRecordChange::Delete { key } => {
                    mutated |= self
                        .env
                        .meta
                        .delete(&mut self.txn, record_key(key, &mut key_buffer)?)
                        .map_err(Error::from)?;
                }
            }
        }
        work.step(1)?;
        match changes.attachment {
            AttachmentChange::Keep => {}
            AttachmentChange::Put(bytes) => {
                let existing = self
                    .env
                    .meta
                    .get(&self.txn, ATTACHMENT_KEY)
                    .map_err(Error::from)?;
                if !same_value(existing, bytes, work)? {
                    self.put_host_value(ATTACHMENT_KEY, bytes, work)?;
                    mutated = true;
                }
            }
            AttachmentChange::Clear => {
                mutated |= self
                    .env
                    .meta
                    .delete(&mut self.txn, ATTACHMENT_KEY)
                    .map_err(Error::from)?;
            }
        }
        work.checkpoint()?;
        Ok(mutated)
    }
}

#[cfg(test)]
impl super::Environment {
    pub(crate) fn fail_host_seal_after(&self, applied_records: Option<usize>) {
        *self.fail_host_after.lock().expect("host fault mutex") = applied_records;
    }
}
