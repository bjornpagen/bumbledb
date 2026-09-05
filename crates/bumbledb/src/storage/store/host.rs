//! Opaque transaction adjunct grammar for native wrappers (C04 seal input).
//!
//! Core does not interpret a receipt, request, history stamp, or migration
//! record: these are host bytes riding the same durable transaction as the
//! facts they describe. The grammar moved here from the deleted transitional
//! `storage::env::host` module; the exported symbol roster
//! (`bumbledb::integration::{HostChanges, HostRecordChange, AttachmentChange,
//! HostSealError}`) is unchanged for the log/native bridge (P04/P06).

use crate::error::Error;
use crate::work::WorkError;

pub use super::keys::HOST_KEY_MAX as MAX_HOST_KEY;

/// A host-grammar refusal or the storage failure a host read/seal hit.
/// Sealing failures consume the prepared capability: any error drops the
/// whole private transaction, never a prefix.
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

impl From<super::error::StoreError> for HostSealError {
    fn from(error: super::error::StoreError) -> Self {
        match error {
            super::error::StoreError::HostKey(fault) => seal_error_of(fault),
            super::error::StoreError::Work(work) => Self::Work(work),
            super::error::StoreError::GenerationExhausted => Self::GenerationExhausted,
            other => Self::Storage(Error::from_store(other)),
        }
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

/// Exclusive resume after one charged host window. Holds only the last
/// visited key — never the remaining set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostResume {
    key: [u8; MAX_HOST_KEY],
    len: u16,
}

impl HostResume {
    /// # Errors
    /// Key longer than [`MAX_HOST_KEY`].
    pub fn from_key(key: &[u8]) -> Result<Self, super::error::StoreError> {
        if key.len() > MAX_HOST_KEY {
            return Err(super::error::StoreError::HostKey(
                super::error::HostKeyFault::TooLong { actual: key.len() },
            ));
        }
        let mut stored = [0u8; MAX_HOST_KEY];
        stored[..key.len()].copy_from_slice(key);
        Ok(Self {
            key: stored,
            len: key.len() as u16,
        })
    }

    #[must_use]
    pub fn as_key(&self) -> &[u8] {
        &self.key[..usize::from(self.len)]
    }
}

/// One charged host window. Peak RAM is this window, not every matching key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostWindow {
    /// More keys may exist after [`HostResume`].
    More {
        resume: HostResume,
        records: u64,
        bytes: u64,
    },
    /// This prefix is exhausted in the scanned range.
    Done { records: u64, bytes: u64 },
}

impl HostWindow {
    #[must_use]
    pub fn resume(&self) -> Option<&HostResume> {
        match self {
            Self::More { resume, .. } => Some(resume),
            Self::Done { .. } => None,
        }
    }

    #[must_use]
    pub const fn records(&self) -> u64 {
        match *self {
            Self::More { records, .. } | Self::Done { records, .. } => records,
        }
    }

    #[must_use]
    pub const fn bytes(&self) -> u64 {
        match *self {
            Self::More { bytes, .. } | Self::Done { bytes, .. } => bytes,
        }
    }
}

/// The exact seal-side grammar refusal for one structured store fault:
/// the integration facade surfaces the same [`HostSealError`] the grammar
/// has always spoken.
#[must_use]
pub(crate) fn seal_error_of(fault: super::error::HostKeyFault) -> HostSealError {
    match fault {
        super::error::HostKeyFault::TooLong { actual } => HostSealError::KeyTooLong {
            actual,
            limit: MAX_HOST_KEY,
        },
        super::error::HostKeyFault::NotStrictlyOrdered => HostSealError::KeysNotStrictlyOrdered,
        super::error::HostKeyFault::LengthOverflow => HostSealError::LengthOverflow,
    }
}
