//! `FsStore`: the five verbs over a local directory. Production tier —
//! the whole backend of the local-fleet deployment and the macOS sync
//! target — not a test double. Create-only publishes an exclusive
//! synced temp (under the reserved `~tmp` namespace) with link(2). The
//! etag is the blake3 of the content. The mutation lock is a fenced
//! CAS lease under `~lease`, broken only on expiry of its own bytes.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::fence::{
    HeldLease, acquire_mutation, sweep_reserved, sync_ancestors, sync_parent, synced_temp,
};
use super::{
    Create, ErrStore, Etag, Fenced, Fetched, LEASE_NAMESPACE, Lease, ObjectStore, Poll, Result,
    StoreKey, Swap, WriterId,
};

/// The one etag scheme: the blake3 of the object bytes, rendered
/// lowercase hex. Derivable from content alone, so every read and every
/// crash recovery re-derives it instead of trusting a stored copy.
#[must_use]
pub fn content_etag(bytes: &[u8]) -> Etag {
    Etag(blake3::hash(bytes).to_hex().to_string())
}

/// A store rooted at a local directory. Keys are slash-separated paths
/// under the root; parent directories appear as needed. Open sweeps
/// reserved temp and expired-lease namespaces.
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let _ = sweep_reserved(&root);
        Self { root }
    }

    fn object_path(&self, key: &StoreKey) -> PathBuf {
        self.root.join(key.as_str())
    }

    fn holder() -> WriterId {
        WriterId(u64::from(std::process::id()))
    }

    fn fence(&self, key: &StoreKey) -> io::Result<HeldLease> {
        acquire_mutation(&self.root, key.as_str(), Self::holder())
    }

    fn generation_path(&self, key: &StoreKey) -> PathBuf {
        self.root
            .join(LEASE_NAMESPACE)
            .join(key.as_str())
            .join("gen")
    }
}

/// Generation sidecar under `~lease/{key}/gen`: 20's lease document,
/// token field is the fencing generation a later swap can lose to.
/// The filename is not a u64, so `current_lease` ignores it; `expires`
/// is `u64::MAX` so sweep does not treat it as litter.
fn read_generation(path: &Path) -> u64 {
    fs::read(path)
        .ok()
        .and_then(|bytes| Lease::parse(&bytes))
        .map(|lease| lease.token)
        .unwrap_or(0)
}

fn write_generation(root: &Path, dest: &Path, token: u64) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
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

fn infra<'k>(op: &'static str, key: &'k str) -> impl FnOnce(io::Error) -> ErrStore + 'k {
    move |source| ErrStore {
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
    fs::create_dir_all(dir)?;
    sync_ancestors(dest, root)
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
            None => Err(ErrStore {
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
            let lease = self.fence(key)?;
            if !lease.still_current()? {
                return Ok(Create::Ambiguous);
            }
            key_shape_fault(&path)?;
            ensure_parent(&self.root, &path)?;
            let temp = synced_temp(&self.root, body.bytes)?;
            if !lease.still_current()? {
                let _ = fs::remove_file(&temp);
                return Ok(Create::Ambiguous);
            }
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
        run().map_err(infra("put_create", key.as_str()))
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
            let lease = self.fence(key)?;
            if !lease.still_current()? {
                return Ok(Swap::Ambiguous);
            }
            let Some(current) = read_fetched(&path)? else {
                return Ok(Swap::Moved);
            };
            if current.etag != *etag {
                return Ok(Swap::Moved);
            }
            // 20: a stale holder's write is the token the CAS no longer wins.
            if body.token < read_generation(&generation) {
                return Ok(Swap::Moved);
            }
            let temp = synced_temp(&self.root, body.bytes)?;
            if !lease.still_current()? {
                let _ = fs::remove_file(&temp);
                return Ok(Swap::Ambiguous);
            }
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
            let lease = self.fence(key)?;
            if !lease.still_current()? {
                return Err(io::Error::other("delete lost the fencing token"));
            }
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

/// Write `bytes` through an exclusive temp and fsync the object plus its
/// parent — the checkpoint-seed durability law, available to any
/// caller that materializes an mdb beside a sidecar. Newly created
/// ancestors are dir-fsynced before the ack.
pub fn durable_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or(path);
    fs::create_dir_all(parent)?;
    if parent.parent().is_some() {
        sync_parent(parent)?;
    }
    let temp = {
        let seq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let name = format!(".{}.{}", std::process::id(), seq);
        let temp = parent.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        if let Err(err) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        temp
    };
    if let Err(err) = fs::rename(&temp, path).and_then(|()| sync_parent(path)) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Create, ObjectStore, StoreKey};

    #[test]
    fn put_create_against_a_directory_is_a_key_shape_fault() {
        let root = std::env::temp_dir().join(format!("fs_store_dir_shape_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root");
        let key = StoreKey::of("manifest.json");
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
}
