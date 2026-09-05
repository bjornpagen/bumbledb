//! Legacy filesystem object-store adapter and local test support. The
//! successor's local authority is LMDB, not this object-store protocol.
//! Create-only publishes an exclusive
//! synced temp (under the reserved `~tmp` namespace) with link(2). The
//! etag is the blake3 of the content. A kernel lock under `~lease` holds
//! the complete mutation; a paused owner cannot be overtaken on a timer.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::fence::{HeldLock, acquire_mutation, sync_ancestors, sync_parent, synced_temp};
use super::{
    Create, Etag, Fenced, Fetched, LEASE_NAMESPACE, Lease, ObjectStore, Poll, Result, StoreError,
    StoreKey, Swap, TEMP_NAMESPACE, WriterId,
};

/// The one etag scheme: the blake3 of the object bytes, rendered
/// lowercase hex. Derivable from content alone, so every read and every
/// crash recovery re-derives it instead of trusting a stored copy.
#[must_use]
pub fn content_etag(bytes: &[u8]) -> Etag {
    Etag(blake3::hash(bytes).to_hex().to_string())
}

/// A store rooted at a local directory. Keys are slash-separated paths
/// under the root; parent directories appear as needed. Construction is
/// read/write-free: it cannot infer abandonment from a temp file's age.
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

    fn fence(&self, key: &StoreKey) -> io::Result<HeldLock> {
        acquire_mutation(&self.root, key)
    }

    fn generation_path(&self, key: &StoreKey) -> PathBuf {
        self.root
            .join(LEASE_NAMESPACE)
            .join(key.as_str())
            .join("gen")
    }
}

/// Legacy remote-token emulation, not local ownership. This separate
/// body/generation protocol is not crash-atomic (audit REP-010), and is
/// not the successor's local-history durability mechanism.
fn read_generation(path: &Path) -> u64 {
    fs::read(path)
        .ok()
        .and_then(|bytes| Lease::parse(&bytes))
        .map_or(0, |lease| lease.token)
}

fn write_generation(root: &Path, dest: &Path, token: u64) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        ensure_dir(parent)?;
    }
    let body = Lease {
        holder: WriterId(0),
        token,
        expires: u64::MAX,
    }
    .encode();
    let temp = synced_temp(root, &body)?;
    if let Err(err) = fs::rename(&temp, dest).and_then(|()| sync_parent(dest)) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

fn infra<'k>(op: &'static str, key: &'k str) -> impl FnOnce(io::Error) -> StoreError + 'k {
    move |source| StoreError {
        op,
        key: key.to_string(),
        source,
    }
}

/// Outcome of a link(2) publication. Link is the exclusivity primitive:
/// POSIX rename replaces an existing destination, so it cannot arbitrate
/// create-only, while link fails atomically with EEXIST across
/// processes — exactly the `If-None-Match: *` contract.
enum Publish {
    Linked,
    Occupied,
}

fn publish_link(temp: &Path, dest: &Path) -> io::Result<Publish> {
    match fs::hard_link(temp, dest) {
        Ok(()) => Ok(Publish::Linked),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(Publish::Occupied),
        Err(err) => Err(err),
    }
}

fn read_fetched(path: &Path) -> io::Result<Option<Fetched>> {
    match fs::read(path) {
        Ok(bytes) => {
            let etag = content_etag(&bytes);
            Ok(Some(Fetched { bytes, etag }))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn ensure_parent(root: &Path, dest: &Path) -> io::Result<()> {
    let dir = dest
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent"))?;
    let mut last = None;
    for _ in 0..8 {
        ensure_dir(dir)?;
        match sync_ancestors(dest, root) {
            Ok(()) => return Ok(()),
            Err(err) if sibling_create_race(&err) => last = Some(err),
            Err(err) => return Err(err),
        }
    }
    ensure_dir(dir)?;
    match sync_ancestors(dest, root) {
        Ok(()) => Ok(()),
        Err(err) => Err(last.unwrap_or(err)),
    }
}

/// Concurrent mutations can create parent directories. Constructors do
/// not remove them or sweep another mutation's temporary files.
fn ensure_dir(path: &Path) -> io::Result<()> {
    let mut last = None;
    for _ in 0..8 {
        match fs::create_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(err) if sibling_create_race(&err) => {
                if path.is_dir() {
                    return Ok(());
                }
                last = Some(err);
            }
            Err(err) => return Err(err),
        }
    }
    if path.is_dir() {
        Ok(())
    } else {
        Err(last
            .unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "parent directory raced")))
    }
}

/// Create reserved namespaces after a retryable parent-directory race.
fn remint_namespaces(root: &Path) {
    let _ = ensure_dir(&root.join(TEMP_NAMESPACE));
    let _ = ensure_dir(&root.join(LEASE_NAMESPACE));
}

/// Legacy retry classification for filesystem publication and parent creation.
/// `link`/`rename` can report ENOENT (2); Darwin also reports EINVAL (22).
/// `create_dir_all` reports EEXIST. A synthetic `InvalidInput` (key
/// names a directory) carries no `raw_os_error` and is not this race.
fn sibling_create_race(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::AlreadyExists | io::ErrorKind::NotFound
    ) || matches!(err.raw_os_error(), Some(2 | 22))
}

/// Resolve a retryable publication outcome by observing the key.
/// That is the create-or-exist sum — `Exists` when GET proves
/// a foreign occupant, `Created` when our bytes landed, `Ambiguous`
/// when the key is still vacant — never infrastructure.
fn settle_create(path: &Path, attempted: &[u8]) -> io::Result<Create> {
    key_shape_fault(path)?;
    match read_fetched(path) {
        Ok(Some(fetched)) if fetched.bytes == attempted => Ok(Create::Created(fetched.etag)),
        Ok(Some(_)) => Ok(Create::Exists),
        Ok(None) => Ok(Create::Ambiguous),
        Err(err) if sibling_create_race(&err) => Ok(Create::Ambiguous),
        Err(err) => Err(err),
    }
}

fn key_shape_fault(path: &Path) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "key names a directory",
        )),
        Ok(_) | Err(_) => Ok(()),
    }
}

impl ObjectStore for FsStore {
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>> {
        let path = self.object_path(key);
        read_fetched(&path).map_err(infra("get", key.as_str()))
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll> {
        let path = self.object_path(key);
        match read_fetched(&path).map_err(infra("get_if_changed", key.as_str()))? {
            Some(fetched) if fetched.etag == *etag => Ok(Poll::Unchanged),
            Some(fetched) => Ok(Poll::Changed(fetched)),
            None => Err(StoreError {
                op: "get_if_changed",
                key: key.to_string(),
                source: io::Error::from(io::ErrorKind::NotFound),
            }),
        }
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> Result<Create> {
        let body = body.into();
        let path = self.object_path(key);
        let generation = self.generation_path(key);
        let run = || -> io::Result<Create> {
            key_shape_fault(&path)?;
            let _ownership = self.fence(key)?;
            key_shape_fault(&path)?;
            ensure_parent(&self.root, &path)?;
            let temp = synced_temp(&self.root, body.bytes)?;
            let published = publish_link(&temp, &path);
            let _ = fs::remove_file(&temp);
            match published? {
                Publish::Linked => {
                    sync_parent(&path)?;
                    write_generation(&self.root, &generation, body.token)?;
                    Ok(Create::Created(content_etag(body.bytes)))
                }
                Publish::Occupied => {
                    if path.is_dir() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "key names a directory",
                        ));
                    }
                    Ok(Create::Exists)
                }
            }
        };
        // Preserve the legacy create outcome resolution for retryable
        // filesystem errors. Constructors themselves no longer do I/O.
        let mut vacant = 0_u8;
        loop {
            match run() {
                Ok(outcome) => return Ok(outcome),
                Err(err) if sibling_create_race(&err) => {
                    remint_namespaces(&self.root);
                    let _ = ensure_parent(&self.root, &path);
                    match settle_create(&path, body.bytes)
                        .map_err(infra("put_create", key.as_str()))?
                    {
                        Create::Ambiguous => {
                            vacant += 1;
                            if vacant == 32 {
                                return Ok(Create::Ambiguous);
                            }
                            std::thread::yield_now();
                        }
                        outcome => return Ok(outcome),
                    }
                }
                Err(err) => return Err(infra("put_create", key.as_str())(err)),
            }
        }
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> Result<Swap> {
        let body = body.into();
        let path = self.object_path(key);
        let generation = self.generation_path(key);
        let run = || -> io::Result<Swap> {
            let _ownership = self.fence(key)?;
            let Some(current) = read_fetched(&path)? else {
                return Ok(Swap::Moved);
            };
            if current.etag != *etag {
                return Ok(Swap::Moved);
            }
            // Legacy remote-token emulation rejects a lower generation.
            if body.token < read_generation(&generation) {
                return Ok(Swap::Moved);
            }
            let temp = synced_temp(&self.root, body.bytes)?;
            if let Err(err) = fs::rename(&temp, &path).and_then(|()| sync_parent(&path)) {
                let _ = fs::remove_file(&temp);
                return Err(err);
            }
            write_generation(&self.root, &generation, body.token)?;
            Ok(Swap::Swapped(content_etag(body.bytes)))
        };
        run().map_err(infra("put_swap", key.as_str()))
    }

    fn delete(&self, key: &StoreKey) -> Result<()> {
        let path = self.object_path(key);
        let run = || -> io::Result<()> {
            let _ownership = self.fence(key)?;
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
            sync_parent(&path)
        };
        run().map_err(infra("delete", key.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Create, ObjectStore, StoreKey};

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
