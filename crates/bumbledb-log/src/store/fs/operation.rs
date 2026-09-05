//! Shared implementation for legacy calls and WorkContext-aware native jobs.
use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::work::{ByteKind, ByteReservation, WorkContext};

use super::{Accounted, FsStore, FsWorkError};
use crate::store::fence::{WorkIoError, acquire_mutation_checked, sync_parent};
use crate::store::{
    Create, Etag, Fenced, Fetched, Lease, Poll, StoreError, StoreKey, Swap, TEMP_NAMESPACE,
    WriterId,
};

const QUANTUM: usize = 4096;
const MAX_LEASE: usize = 71;
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);
type IoWork<T> = Result<T, WorkIoError>;

pub(super) struct Product<T> {
    pub value: T,
    reservation: Option<ByteReservation>,
}

impl<T> Product<T> {
    pub(super) fn accounted(self) -> Accounted<T> {
        Accounted {
            value: self.value,
            reservation: self
                .reservation
                .expect("tracked entry supplies one reservation"),
        }
    }
}

#[derive(Clone, Copy)]
struct Budget<'a>(Option<&'a WorkContext>);

impl Budget<'_> {
    fn step(self, units: u64) -> IoWork<()> {
        if let Some(work) = self.0 {
            work.step(units)?;
        }
        Ok(())
    }

    fn reserve(self, kind: ByteKind, bytes: u64) -> IoWork<Option<ByteReservation>> {
        self.0
            .map(|work| work.reserve(kind, bytes))
            .transpose()
            .map_err(Into::into)
    }

    fn result<T>(self, value: T, bytes: u64) -> IoWork<Product<T>> {
        Ok(Product {
            value,
            reservation: self.reserve(ByteKind::Result, bytes)?,
        })
    }

    fn paths(self, root: &Path, key: &StoreKey) -> IoWork<Option<ByteReservation>> {
        let root = root.as_os_str().len() as u64;
        let key = key.as_str().len() as u64;
        let bytes = root
            .checked_add(key)
            .and_then(|bytes| bytes.checked_mul(4))
            .and_then(|bytes| bytes.checked_add(128))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path size overflow"))?;
        self.reserve(ByteKind::Working, bytes)
    }
}

fn failure(op: &'static str, key: &StoreKey, error: WorkIoError) -> FsWorkError {
    match error {
        WorkIoError::Work(error) => FsWorkError::Work(error),
        WorkIoError::Io(source) => FsWorkError::Store(StoreError {
            op,
            key: key.to_string(),
            source,
        }),
    }
}

fn allocation() -> io::Error {
    io::Error::new(
        io::ErrorKind::OutOfMemory,
        "filesystem result allocation failed",
    )
}

fn open_object(path: &Path, budget: Budget<'_>) -> IoWork<Option<File>> {
    budget.step(1)?;
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Open once, reserve from that inode's length before allocation, and read/hash
/// only bounded chunks. A concurrent path rename cannot switch the open inode.
/// A file that changes length in place refuses rather than growing its buffer.
fn fetched(path: &Path, kind: ByteKind, budget: Budget<'_>) -> IoWork<Product<Option<Fetched>>> {
    let Some(mut file) = open_object(path, budget)? else {
        return Ok(Product {
            value: None,
            reservation: budget.reserve(kind, 0)?,
        });
    };
    budget.step(1)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "key is not a regular file").into(),
        );
    }
    let length = metadata.len();
    let charged = length
        .checked_add(64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "object length overflow"))?;
    let reservation = budget.reserve(kind, charged)?;
    let length = usize::try_from(length).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "object length exceeds address space",
        )
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| allocation())?;
    let mut hash = blake3::Hasher::new();
    while bytes.len() < length {
        let begin = bytes.len();
        let count = (length - begin).min(QUANTUM);
        budget.step(count as u64)?;
        bytes.resize(begin + count, 0);
        file.read_exact(&mut bytes[begin..])?;
        budget.step(count as u64)?;
        hash.update(&bytes[begin..]);
    }
    budget.step(1)?;
    let mut extra = [0];
    if file.read(&mut extra)? != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "object grew during read").into());
    }
    Ok(Product {
        value: Some(Fetched {
            bytes,
            etag: Etag(hash.finalize().to_hex().to_string()),
        }),
        reservation,
    })
}

fn hash_bytes(bytes: &[u8], budget: Budget<'_>) -> IoWork<Etag> {
    let mut hash = blake3::Hasher::new();
    for chunk in bytes.chunks(QUANTUM) {
        budget.step(chunk.len() as u64)?;
        hash.update(chunk);
    }
    Ok(Etag(hash.finalize().to_hex().to_string()))
}

/// CAS only needs the old content hash, not a second whole-body allocation.
fn hash_object(path: &Path, budget: Budget<'_>) -> IoWork<Option<blake3::Hash>> {
    let Some(mut file) = open_object(path, budget)? else {
        return Ok(None);
    };
    let _working = budget.reserve(ByteKind::Working, QUANTUM as u64)?;
    let mut chunk = [0; QUANTUM];
    let mut hash = blake3::Hasher::new();
    loop {
        budget.step(1)?;
        let count = file.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        budget.step(count as u64)?;
        hash.update(&chunk[..count]);
    }
    Ok(Some(hash.finalize()))
}

/// Legacy token sidecars are a bounded grammar, never arbitrary whole reads.
/// Preserve the existing malformed/missing-sidecar-as-zero legacy meaning.
fn generation(path: &Path, budget: Budget<'_>) -> IoWork<u64> {
    let _working = budget.reserve(ByteKind::Working, 256)?;
    let Some(mut file) = (match open_object(path, budget) {
        Ok(file) => file,
        Err(WorkIoError::Io(_)) => None,
        Err(error) => return Err(error),
    }) else {
        return Ok(0);
    };
    let mut bytes = [0; MAX_LEASE + 1];
    let mut used = 0;
    while used < bytes.len() {
        budget.step(1)?;
        match file.read(&mut bytes[used..]) {
            Ok(0) => break,
            Ok(count) => used += count,
            Err(_) => return Ok(0),
        }
    }
    budget.step(used as u64)?;
    if used > MAX_LEASE {
        return Ok(0);
    }
    Ok(Lease::parse(&bytes[..used]).map_or(0, |lease| lease.token))
}

/// Owns both staged disk bytes and its path until publication or cleanup.
struct Temporary {
    path: PathBuf,
    _scratch: Option<ByteReservation>,
    _paths: Option<ByteReservation>,
}

impl Temporary {
    fn stage(root: &Path, bytes: &[u8], budget: Budget<'_>) -> IoWork<Self> {
        let scratch = budget.reserve(ByteKind::Scratch, bytes.len() as u64)?;
        let path_bytes = (root.as_os_str().len() as u64)
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(192))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "temp path size overflow")
            })?;
        let paths = budget.reserve(ByteKind::Working, path_bytes)?;
        budget.step(1)?;
        let dir = root.join(TEMP_NAMESPACE);
        fs::create_dir_all(&dir)?;
        let seq = TEMP_SEQ
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| io::Error::other("temporary sequence exhausted"))?;
        // Separate spelling from fence::synced_temp; both can run in one process.
        let path = dir.join(format!("{}.fs.{seq}", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let staged = Self {
            path,
            _scratch: scratch,
            _paths: paths,
        };
        for chunk in bytes.chunks(QUANTUM) {
            budget.step(chunk.len() as u64)?;
            file.write_all(chunk)?;
        }
        budget.step(1)?;
        file.sync_all()?;
        Ok(staged)
    }

    fn generation(root: &Path, token: u64, budget: Budget<'_>) -> IoWork<Self> {
        let _working = budget.reserve(ByteKind::Working, MAX_LEASE as u64)?;
        let body = Lease {
            holder: WriterId(0),
            token,
            expires: u64::MAX,
        }
        .encode();
        Self::stage(root, &body, budget)
    }
}

impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::AlreadyExists | io::ErrorKind::NotFound
    ) || matches!(error.raw_os_error(), Some(2 | 22))
}

fn ensure_dir(path: &Path, budget: Budget<'_>) -> IoWork<()> {
    let mut last = None;
    for _ in 0..8 {
        budget.step(1)?;
        match fs::create_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(error) if retryable(&error) => {
                if path.is_dir() {
                    return Ok(());
                }
                last = Some(error);
            }
            Err(error) => return Err(error.into()),
        }
    }
    if path.is_dir() {
        Ok(())
    } else {
        Err(last
            .unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "parent directory raced"))
            .into())
    }
}

fn ensure_parent(root: &Path, path: &Path, budget: Budget<'_>) -> IoWork<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "key has no parent"))?;
    ensure_dir(parent, budget)?;
    let mut current = Some(parent);
    while let Some(dir) = current {
        budget.step(1)?;
        File::open(dir)?.sync_all()?;
        if dir == root {
            break;
        }
        current = dir.parent();
    }
    Ok(())
}

fn key_shape(path: &Path, budget: Budget<'_>) -> IoWork<()> {
    budget.step(1)?;
    if fs::metadata(path).is_ok_and(|meta| meta.is_dir()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "key names a directory").into());
    }
    Ok(())
}

/// After publication, complete required durability work without cancellation
/// checkpoints. A failed fsync/sidecar rename remains an I/O failure (uncertain
/// publication), never a prepublication WorkError or a fabricated rollback.
fn finish_generation(staged: &Temporary, dest: &Path) -> io::Result<()> {
    fs::rename(&staged.path, dest)?;
    sync_parent(dest)
}

impl FsStore {
    pub(super) fn get_work(
        &self,
        key: &StoreKey,
        work: Option<&WorkContext>,
    ) -> Result<Product<Option<Fetched>>, FsWorkError> {
        let budget = Budget(work);
        let run = || {
            let _paths = budget.paths(&self.root, key)?;
            fetched(&self.object_path(key), ByteKind::Result, budget)
        };
        run().map_err(|error| failure("get", key, error))
    }

    pub(super) fn poll_work(
        &self,
        key: &StoreKey,
        etag: &Etag,
        work: Option<&WorkContext>,
    ) -> Result<Product<Poll>, FsWorkError> {
        let budget = Budget(work);
        let run = || {
            let _paths = budget.paths(&self.root, key)?;
            let output = fetched(&self.object_path(key), ByteKind::Result, budget)?;
            match output.value {
                Some(current) if current.etag == *etag => {
                    drop(current);
                    drop(output.reservation);
                    budget.result(Poll::Unchanged, 0)
                }
                Some(current) => Ok(Product {
                    value: Poll::Changed(current),
                    reservation: output.reservation,
                }),
                None => Err(io::Error::from(io::ErrorKind::NotFound).into()),
            }
        };
        run().map_err(|error| failure("get_if_changed", key, error))
    }

    pub(super) fn create_work(
        &self,
        key: &StoreKey,
        body: Fenced<'_>,
        work: Option<&WorkContext>,
    ) -> Result<Product<Create>, FsWorkError> {
        let budget = Budget(work);
        let published = Cell::new(false);
        let run = || {
            published.set(false);
            let _paths = budget.paths(&self.root, key)?;
            let path = self.object_path(key);
            let generation = self.generation_path(key);
            key_shape(&path, budget)?;
            let _owner = acquire_mutation_checked(&self.root, key, work)?;
            key_shape(&path, budget)?;
            match fs::metadata(&path) {
                Ok(_) => return budget.result(Create::Exists, 0),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let reservation = budget.reserve(ByteKind::Result, 64)?;
            let tag = hash_bytes(body.bytes, budget)?;
            ensure_parent(&self.root, &path, budget)?;
            let staged = Temporary::stage(&self.root, body.bytes, budget)?;
            let staged_generation = Temporary::generation(&self.root, body.token, budget)?;
            budget.step(1)?;
            match fs::hard_link(&staged.path, &path) {
                Ok(()) => {
                    published.set(true);
                    sync_parent(&path)?;
                    finish_generation(&staged_generation, &generation)?;
                    Ok(Product {
                        value: Create::Created(tag),
                        reservation,
                    })
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    key_shape(&path, budget)?;
                    drop(reservation);
                    budget.result(Create::Exists, 0)
                }
                Err(error) => Err(error.into()),
            }
        };
        // Preserve the adapter's explicit ambiguous publication recovery, but
        // never retry a work refusal or spend unbounded compare/read work.
        let mut vacant = 0;
        loop {
            match run() {
                Ok(output) => return Ok(output),
                Err(WorkIoError::Io(error)) if !published.get() && retryable(&error) => {
                    let settle = || {
                        let _paths = budget.paths(&self.root, key)?;
                        let path = self.object_path(key);
                        key_shape(&path, budget)?;
                        let current = fetched(&path, ByteKind::Working, budget)?;
                        match current.value {
                            None => budget.result(Create::Ambiguous, 0),
                            Some(current) => {
                                let mut equal = current.bytes.len() == body.bytes.len();
                                if equal {
                                    for (left, right) in current
                                        .bytes
                                        .chunks(QUANTUM)
                                        .zip(body.bytes.chunks(QUANTUM))
                                    {
                                        budget.step(left.len() as u64)?;
                                        if left != right {
                                            equal = false;
                                            break;
                                        }
                                    }
                                }
                                if equal {
                                    let reservation = budget.reserve(ByteKind::Result, 64)?;
                                    Ok(Product {
                                        value: Create::Created(current.etag),
                                        reservation,
                                    })
                                } else {
                                    budget.result(Create::Exists, 0)
                                }
                            }
                        }
                    };
                    let result = settle().map_err(|error| failure("put_create", key, error))?;
                    if result.value != Create::Ambiguous {
                        return Ok(result);
                    }
                    vacant += 1;
                    if vacant == 32 {
                        return Ok(result);
                    }
                    std::thread::yield_now();
                }
                Err(error) => return Err(failure("put_create", key, error)),
            }
        }
    }

    pub(super) fn swap_work(
        &self,
        key: &StoreKey,
        body: Fenced<'_>,
        expected: &Etag,
        work: Option<&WorkContext>,
    ) -> Result<Product<Swap>, FsWorkError> {
        let budget = Budget(work);
        let run = || {
            let _paths = budget.paths(&self.root, key)?;
            let path = self.object_path(key);
            let generation_path = self.generation_path(key);
            let _owner = acquire_mutation_checked(&self.root, key, work)?;
            let Some(current) = hash_object(&path, budget)? else {
                return budget.result(Swap::Moved, 0);
            };
            if current.to_hex().as_str() != expected.0
                || body.token < generation(&generation_path, budget)?
            {
                return budget.result(Swap::Moved, 0);
            }
            let reservation = budget.reserve(ByteKind::Result, 64)?;
            let tag = hash_bytes(body.bytes, budget)?;
            let staged = Temporary::stage(&self.root, body.bytes, budget)?;
            let staged_generation = Temporary::generation(&self.root, body.token, budget)?;
            budget.step(1)?;
            fs::rename(&staged.path, &path)?;
            sync_parent(&path)?;
            finish_generation(&staged_generation, &generation_path)?;
            Ok(Product {
                value: Swap::Swapped(tag),
                reservation,
            })
        };
        run().map_err(|error| failure("put_swap", key, error))
    }

    pub(super) fn delete_work(
        &self,
        key: &StoreKey,
        work: Option<&WorkContext>,
    ) -> Result<Product<()>, FsWorkError> {
        let budget = Budget(work);
        let run = || {
            let _paths = budget.paths(&self.root, key)?;
            let path = self.object_path(key);
            let result = budget.result((), 0)?;
            let _owner = acquire_mutation_checked(&self.root, key, work)?;
            budget.step(1)?;
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            sync_parent(&path)?;
            Ok(result)
        };
        run().map_err(|error| failure("delete", key, error))
    }
}
