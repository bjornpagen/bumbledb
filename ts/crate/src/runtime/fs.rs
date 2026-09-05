//! Whole filesystem-store verbs on the existing executor. Rust owns the
//! mutation lock and its complete critical section; JavaScript never holds it.
use bumbledb::work::WorkContext;
use bumbledb_log::store::{Create, Etag, Fenced, Fetched, Poll, StoreKey, Swap};
use bumbledb_log::store::fs::{Accounted, FsStore, FsWorkError};

use super::RuntimeError;

pub enum FsOutput {
    Get(Accounted<Option<Fetched>>),
    Poll(Accounted<Poll>),
    Create(Accounted<Create>),
    Swap(Accounted<Swap>),
    Delete(Accounted<()>),
}

impl FsOutput {
    pub fn mutating(&self) -> bool { matches!(self, Self::Create(_) | Self::Swap(_) | Self::Delete(_)) }
}

pub enum FsVerb {
    Get,
    Poll(Etag),
    Create { bytes: Vec<u8>, token: u64 },
    Swap { bytes: Vec<u8>, token: u64, etag: Etag },
    Delete,
}

pub fn execute(root: String, key: StoreKey, verb: FsVerb, work: &WorkContext) -> Result<FsOutput, RuntimeError> {
    let store = FsStore::new(root);
    match verb {
        FsVerb::Get => store.get_with(&key, work).map(FsOutput::Get),
        FsVerb::Poll(etag) => store.get_if_changed_with(&key, &etag, work).map(FsOutput::Poll),
        FsVerb::Create { bytes, token } => store.put_create_with(&key, Fenced::new(&bytes, token), work).map(FsOutput::Create),
        FsVerb::Swap { bytes, token, etag } => store.put_swap_with(&key, Fenced::new(&bytes, token), &etag, work).map(FsOutput::Swap),
        FsVerb::Delete => store.delete_with(&key, work).map(FsOutput::Delete),
    }.map_err(|error| match error {
        FsWorkError::Work(error) => RuntimeError::Work(error),
        FsWorkError::Store(error) => super::owners::io_error(error.source),
    })
}
