//! Legacy filesystem object-store adapter and local test support.
//! One implementation serves untracked Rust callers and bounded native jobs.
//! The kernel-held mutation lock spans comparison, publication and fsync.
//! This adapter is not the successor's LMDB local-history authority.

use std::path::PathBuf;

use bumbledb::work::{ByteReservation, WorkContext, WorkError};

use super::{
    Create, Etag, Fenced, Fetched, LEASE_NAMESPACE, ObjectStore, Poll, Result, StoreError,
    StoreKey, Swap,
};

#[cfg(test)]
mod bounded_tests;
mod operation;

/// The content etag used by all filesystem and in-memory readers.
#[must_use]
pub fn content_etag(bytes: &[u8]) -> Etag {
    Etag(blake3::hash(bytes).to_hex().to_string())
}

/// An owned result retains its exact logical body/etag charge until consumed.
#[derive(Debug)]
pub struct Accounted<T> {
    pub value: T,
    pub reservation: ByteReservation,
}

/// Cooperative refusal is not erased into an I/O message or a CAS outcome.
#[derive(Debug)]
pub enum FsWorkError {
    Store(StoreError),
    Work(WorkError),
}

impl std::fmt::Display for FsWorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => error.fmt(f),
            Self::Work(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for FsWorkError {}

/// Construction does no I/O and cannot infer abandonment from file age.
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn object_path(&self, key: &StoreKey) -> PathBuf {
        self.root.join(key.as_str())
    }

    fn generation_path(&self, key: &StoreKey) -> PathBuf {
        self.root
            .join(LEASE_NAMESPACE)
            .join(key.as_str())
            .join("gen")
    }

    /// # Errors
    /// Refuses I/O failure, cancellation, deadline or insufficient work/bytes.
    pub fn get_with(
        &self,
        key: &StoreKey,
        work: &WorkContext,
    ) -> std::result::Result<Accounted<Option<Fetched>>, FsWorkError> {
        self.get_work(key, Some(work))
            .map(operation::Product::accounted)
    }

    /// # Errors
    /// Refuses I/O failure, absence, cancellation or insufficient work/bytes.
    pub fn get_if_changed_with(
        &self,
        key: &StoreKey,
        etag: &Etag,
        work: &WorkContext,
    ) -> std::result::Result<Accounted<Poll>, FsWorkError> {
        self.poll_work(key, etag, Some(work))
            .map(operation::Product::accounted)
    }

    /// # Errors
    /// Refuses I/O failure or cooperative limits before publication.
    pub fn put_create_with<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        work: &WorkContext,
    ) -> std::result::Result<Accounted<Create>, FsWorkError> {
        self.create_work(key, body.into(), Some(work))
            .map(operation::Product::accounted)
    }

    /// # Errors
    /// Refuses I/O failure or cooperative limits before publication.
    pub fn put_swap_with<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
        work: &WorkContext,
    ) -> std::result::Result<Accounted<Swap>, FsWorkError> {
        self.swap_work(key, body.into(), etag, Some(work))
            .map(operation::Product::accounted)
    }

    /// # Errors
    /// Refuses I/O failure or cooperative limits before deletion.
    pub fn delete_with(
        &self,
        key: &StoreKey,
        work: &WorkContext,
    ) -> std::result::Result<Accounted<()>, FsWorkError> {
        self.delete_work(key, Some(work))
            .map(operation::Product::accounted)
    }
}

fn legacy<T>(result: std::result::Result<operation::Product<T>, FsWorkError>) -> Result<T> {
    result
        .map(|output| output.value)
        .map_err(|error| match error {
            FsWorkError::Store(error) => error,
            // Untracked execution cannot produce this arm. Keep it a refusal,
            // rather than making an invariant violation panic across a boundary.
            FsWorkError::Work(error) => StoreError {
                op: "legacy filesystem operation",
                key: String::new(),
                source: std::io::Error::other(error),
            },
        })
}

impl ObjectStore for FsStore {
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>> {
        legacy(self.get_work(key, None))
    }
    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll> {
        legacy(self.poll_work(key, etag, None))
    }
    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> Result<Create> {
        legacy(self.create_work(key, body.into(), None))
    }
    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> Result<Swap> {
        legacy(self.swap_work(key, body.into(), etag, None))
    }
    fn delete(&self, key: &StoreKey) -> Result<()> {
        legacy(self.delete_work(key, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Create, ObjectStore, StoreKey, TEMP_NAMESPACE};
    use std::{fs, io};

    fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path =
            std::env::temp_dir().join(format!("bdb-log-fs-{tag}-{}-{nanos}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("root");
        path
    }

    #[test]
    fn put_create_against_a_directory_is_a_key_shape_fault() {
        let root = scratch("dir_shape");
        let key = StoreKey::of("manifest");
        fs::create_dir_all(root.join(key.as_str())).expect("directory at the key");
        let store = FsStore::new(&root);
        let outcome = store.put_create(&key, b"body");
        match outcome {
            Err(err) => {
                assert_eq!(err.op, "put_create");
                assert!(
                    err.source.to_string().contains("key names a directory"),
                    "directory at a key is a shape fault: {}",
                    err.source
                );
            }
            Ok(Create::Exists) => panic!("a directory at the key is not Exists"),
            Ok(other) => panic!("expected a key-shape fault, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn constructor_leaves_an_in_flight_temp() {
        let root = scratch("sweep_live");
        let temp = crate::store::fence::synced_temp(&root, b"live").expect("temp");
        let _store = FsStore::new(&root);
        assert!(
            temp.exists(),
            "a constructor must not wipe a live publish temp"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn occupied_create_does_not_stage_or_claim_a_second_equal_body_birth() {
        let root = scratch("occupied_no_staging");
        let key = StoreKey::of("ids/counter");
        let store = FsStore::new(&root);
        assert!(matches!(
            store.put_create(&key, b"4096").unwrap(),
            Create::Created(_)
        ));
        let generation = fs::read(store.generation_path(&key)).unwrap();

        // An occupied create needs no temporary file. A regular file here
        // deterministically fails an attempted staging-directory creation,
        // independently of filesystem permissions or elapsed time.
        fs::remove_dir(root.join(TEMP_NAMESPACE)).unwrap();
        fs::write(root.join(TEMP_NAMESPACE), b"staging unavailable").unwrap();
        for body in [b"4096".as_slice(), b"different".as_slice()] {
            assert_eq!(
                store.put_create(&key, Fenced::new(body, 99)).unwrap(),
                Create::Exists
            );
        }
        assert_eq!(store.get(&key).unwrap().unwrap().bytes, b"4096");
        assert_eq!(fs::read(store.generation_path(&key)).unwrap(), generation);
        assert_eq!(
            fs::read(root.join(TEMP_NAMESPACE)).unwrap(),
            b"staging unavailable"
        );
        let _ = fs::remove_dir_all(&root);
    }

    fn create_or_exist(store: &FsStore, key: &StoreKey, body: &[u8]) -> Create {
        for _ in 0..32 {
            match store.put_create(key, body) {
                Ok(Create::Created(etag)) => return Create::Created(etag),
                Ok(Create::Exists) => return Create::Exists,
                Ok(Create::Ambiguous) | Err(_) => match store.get(key) {
                    Ok(Some(fetched)) if fetched.bytes == body => {
                        return Create::Created(content_etag(body));
                    }
                    Ok(Some(_)) => return Create::Exists,
                    Ok(None) | Err(_) => std::thread::yield_now(),
                },
            }
        }
        match store.get(key) {
            Ok(Some(fetched)) if fetched.bytes == body => Create::Created(content_etag(body)),
            Ok(Some(_)) => Create::Exists,
            Ok(None) | Err(_) => Create::Ambiguous,
        }
    }

    #[test]
    fn concurrent_constructors_create_or_exist_without_enoent() {
        const WRITERS: u64 = 8;
        let root = scratch("concurrent_new");
        let outcomes: Vec<Create> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..WRITERS)
                .map(|i| {
                    let root = root.clone();
                    scope.spawn(move || {
                        let store = FsStore::new(&root);
                        let key = StoreKey::of("manifest");
                        let body = format!("body {i}");
                        create_or_exist(&store, &key, body.as_bytes())
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("thread"))
                .collect()
        });
        let created = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Create::Created(_)))
            .count();
        let exists = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Create::Exists))
            .count();
        assert_eq!(created, 1, "exactly one creator wins: {outcomes:?}");
        assert_eq!(
            exists,
            usize::try_from(WRITERS).expect("fits") - 1,
            "every other constructor sees Exists: {outcomes:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
