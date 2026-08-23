//! `FsStore`: the five verbs over a local directory. Production tier —
//! the whole backend of the local-fleet deployment and the macOS sync
//! target — not a test double. One on-disk protocol shared with the TS
//! driver: create-only publishes an exclusive synced temp with link(2),
//! the etag is the blake3 of the content and is computed on every read
//! rather than stored anywhere, and the mutation lock is a pid-lockfile
//! beside the key. One machine is load-bearing: link exclusivity and
//! pid liveness are the arbitration primitives, and network filesystems
//! weaken the first while machine boundaries void the second, so a
//! prefix on a network mount is a misdeployment no syscall can detect
//! for us.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::{Create, Etag, Fetched, ObjectStore, Poll, Result, StoreError, Swap};

/// The one etag scheme: the blake3 of the object bytes, rendered
/// lowercase hex. Derivable from content alone, so every read and every
/// crash recovery re-derives it instead of trusting a stored copy.
#[must_use]
pub fn content_etag(bytes: &[u8]) -> Etag {
    Etag(blake3::hash(bytes).to_hex().to_string())
}

/// A store rooted at a local directory. Keys are slash-separated paths
/// under the root; parent directories appear as needed.
pub struct FsStore {
    root: PathBuf,
}

/// Suffix of the per-key pid-lockfile that serializes `put_swap`.
const LOCK_SUFFIX: &str = ".lock";

/// Ceiling of the jittered wait between probes of a live-held lock.
const LOCK_RETRY_MS: u64 = 10;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Parse the key at the boundary: relative, slash-separated, no empty
    /// or dot segments (so it can never escape the root), and no segment
    /// wearing the lockfile suffix (so it can never collide with a lock).
    fn object_path(&self, op: &'static str, key: &str) -> Result<PathBuf> {
        let well_formed = !key.is_empty()
            && !key.starts_with('/')
            && !key.ends_with('/')
            && key.split('/').all(|seg| {
                !seg.is_empty() && seg != "." && seg != ".." && !seg.ends_with(LOCK_SUFFIX)
            });
        if well_formed {
            Ok(self.root.join(key))
        } else {
            Err(StoreError {
                op,
                key: key.to_string(),
                source: io::Error::new(io::ErrorKind::InvalidInput, "malformed object key"),
            })
        }
    }
}

fn infra<'k>(op: &'static str, key: &'k str) -> impl FnOnce(io::Error) -> StoreError + 'k {
    move |source| StoreError {
        op,
        key: key.to_string(),
        source,
    }
}

/// Write `bytes` to a fresh `O_CREAT|O_EXCL` temp file beside `dest` and
/// fsync it. The caller publishes the synced temp atomically.
fn synced_temp(dest: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let dir = dest
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent"))?;
    fs::create_dir_all(dir)?;
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "object path has no name"))?;
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let temp = dir.join(format!(".{name}.tmp.{pid}.{seq}", pid = std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    if let Err(err) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(temp)
}

/// Fsync the directory holding `path`, so the rename or link that
/// published into it survives power loss before the ack does.
fn sync_parent(path: &Path) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent"))?;
    File::open(dir)?.sync_all()
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

/// kill(pid, 0) by way of the system `kill` utility, the one process
/// probe reachable with this crate's `unsafe_code = deny`: exit 0 means
/// the pid can be signalled, so its owner is alive.
fn pid_alive(pid: u32) -> io::Result<bool> {
    let status = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    Ok(status.success())
}

/// The mutation lock beside a key: a lockfile published with the same
/// exclusive temp-plus-link discipline as the objects themselves, so it
/// can never exist without its body — the owner pid. A contender probes
/// the owner's liveness and breaks the lock iff the owner is dead; a
/// live owner is waited out with jittered probes, unbounded. Pid
/// liveness is meaningful on one machine only, which is `FsStore`'s
/// load-bearing deployment law.
struct KeyLock {
    path: PathBuf,
}

impl KeyLock {
    fn acquire(object: &Path) -> io::Result<Self> {
        let name = object.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "object path has no name")
        })?;
        let dir = object.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent")
        })?;
        let path = dir.join(format!("{name}{LOCK_SUFFIX}"));
        let body = std::process::id().to_string();
        loop {
            let temp = synced_temp(&path, body.as_bytes())?;
            let published = publish_link(&temp, &path);
            let _ = fs::remove_file(&temp);
            match published? {
                Publish::Linked => return Ok(Self { path }),
                Publish::Occupied => match fs::read_to_string(&path) {
                    Ok(owner) => {
                        let pid: u32 = owner.trim().parse().map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("lockfile body is not a pid: {owner:?}"),
                            )
                        })?;
                        if pid_alive(pid)? {
                            std::thread::sleep(super::jittered(Duration::from_millis(
                                LOCK_RETRY_MS,
                            )));
                        } else {
                            let _ = fs::remove_file(&path);
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => return Err(err),
                },
            }
        }
    }
}

impl Drop for KeyLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
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

impl ObjectStore for FsStore {
    fn get(&self, key: &str) -> Result<Option<Fetched>> {
        let path = self.object_path("get", key)?;
        read_fetched(&path).map_err(infra("get", key))
    }

    fn get_if_changed(&self, key: &str, etag: &Etag) -> Result<Poll> {
        let path = self.object_path("get_if_changed", key)?;
        match read_fetched(&path).map_err(infra("get_if_changed", key))? {
            Some(fetched) if fetched.etag == *etag => Ok(Poll::Unchanged),
            Some(fetched) => Ok(Poll::Changed(fetched)),
            None => Err(StoreError {
                op: "get_if_changed",
                key: key.to_string(),
                source: io::Error::from(io::ErrorKind::NotFound),
            }),
        }
    }

    fn put_create(&self, key: &str, bytes: &[u8]) -> Result<Create> {
        let path = self.object_path("put_create", key)?;
        let temp = synced_temp(&path, bytes).map_err(infra("put_create", key))?;
        let published = publish_link(&temp, &path);
        let _ = fs::remove_file(&temp);
        let outcome = match published {
            Ok(Publish::Linked) => {
                sync_parent(&path).map(|()| Create::Created(content_etag(bytes)))
            }
            Ok(Publish::Occupied) => Ok(Create::Exists),
            Err(err) => Err(err),
        };
        outcome.map_err(infra("put_create", key))
    }

    fn put_swap(&self, key: &str, bytes: &[u8], etag: &Etag) -> Result<Swap> {
        let path = self.object_path("put_swap", key)?;
        let run = || -> io::Result<Swap> {
            let _lock = KeyLock::acquire(&path)?;
            // Under the lock, the incumbent's etag is re-derived from the
            // object bytes: content is the only record there is.
            let Some(current) = read_fetched(&path)? else {
                return Ok(Swap::Moved);
            };
            if current.etag != *etag {
                return Ok(Swap::Moved);
            }
            let temp = synced_temp(&path, bytes)?;
            if let Err(err) = fs::rename(&temp, &path).and_then(|()| sync_parent(&path)) {
                let _ = fs::remove_file(&temp);
                return Err(err);
            }
            Ok(Swap::Swapped(content_etag(bytes)))
        };
        run().map_err(infra("put_swap", key))
    }

    fn delete(&self, key: &str) -> Result<()> {
        let path = self.object_path("delete", key)?;
        let mut lockfile = path.clone().into_os_string();
        lockfile.push(LOCK_SUFFIX);
        for victim in [path, PathBuf::from(lockfile)] {
            match fs::remove_file(&victim) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(infra("delete", key)(err)),
            }
        }
        Ok(())
    }
}
