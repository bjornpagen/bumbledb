//! `FsStore`: the five verbs over a local directory. Production tier —
//! the whole backend of the local-fleet deployment and the macOS sync
//! target — not a test double. One machine is load-bearing: exclusive
//! link publication and flock are the arbitration primitives, and network
//! filesystems weaken both, so a prefix on a network mount is a
//! misdeployment no syscall can detect for us.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Create, Etag, Fetched, ObjectStore, Poll, Result, StoreError, Swap};

/// The `FsStore` etag scheme: the blake3 of the object bytes, rendered
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

/// Suffix of the per-key sidecar that serves as the CAS lock file and the
/// swap record.
const ETAG_SUFFIX: &str = ".etag";

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Parse the key at the boundary: relative, slash-separated, no empty
    /// or dot segments, so it can never escape the root.
    fn object_path(&self, op: &'static str, key: &str) -> Result<PathBuf> {
        let well_formed = !key.is_empty()
            && !key.starts_with('/')
            && !key.ends_with('/')
            && key
                .split('/')
                .all(|seg| !seg.is_empty() && seg != "." && seg != "..");
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

/// Block until this process holds the exclusive flock on `file`. The lock
/// arbitrates CAS across real processes and dies with the descriptor, so
/// a crashed holder can never wedge the key.
#[allow(unsafe_code)]
fn lock_exclusive(file: &File) -> io::Result<()> {
    // SAFETY: `flock` reads and writes no user memory; it takes only the
    // raw descriptor, which `file` keeps open for the whole call.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
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
        // Publication is link(2), not rename(2): rename replaces an
        // existing destination, so it cannot arbitrate create-only —
        // link fails atomically with EEXIST across processes, which is
        // exactly the `If-None-Match: *` contract. The synced temp then
        // drops away.
        let linked = fs::hard_link(&temp, &path);
        let outcome = match linked {
            Ok(()) => sync_parent(&path).map(|()| Create::Created(content_etag(bytes))),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(Create::Exists),
            Err(err) => Err(err),
        };
        let _ = fs::remove_file(&temp);
        outcome.map_err(infra("put_create", key))
    }

    fn put_swap(&self, key: &str, bytes: &[u8], etag: &Etag) -> Result<Swap> {
        let path = self.object_path("put_swap", key)?;
        let run = || -> io::Result<Swap> {
            let dir = path.parent().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent")
            })?;
            fs::create_dir_all(dir)?;
            let sidecar_name = format!(
                "{}{ETAG_SUFFIX}",
                path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "object path has no name")
                })?
            );
            let sidecar_path = dir.join(sidecar_name);
            let mut sidecar = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&sidecar_path)?;
            lock_exclusive(&sidecar)?;
            // Under the lock, the current etag is re-derived from the
            // object bytes: content is the record the sidecar can only
            // echo, so a crash between rename and echo heals itself.
            let Some(current) = read_fetched(&path)? else {
                return Ok(Swap::Moved);
            };
            if current.etag != *etag {
                return Ok(Swap::Moved);
            }
            let temp = synced_temp(&path, bytes)?;
            let renamed = fs::rename(&temp, &path).and_then(|()| sync_parent(&path));
            if let Err(err) = renamed {
                let _ = fs::remove_file(&temp);
                return Err(err);
            }
            let next = content_etag(bytes);
            sidecar.seek(SeekFrom::Start(0))?;
            sidecar.set_len(0)?;
            sidecar.write_all(next.0.as_bytes())?;
            sidecar.sync_all()?;
            Ok(Swap::Swapped(next))
        };
        run().map_err(infra("put_swap", key))
    }

    fn delete(&self, key: &str) -> Result<()> {
        let path = self.object_path("delete", key)?;
        let mut sidecar = path.clone().into_os_string();
        sidecar.push(ETAG_SUFFIX);
        for victim in [path, PathBuf::from(sidecar)] {
            match fs::remove_file(&victim) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(infra("delete", key)(err)),
            }
        }
        Ok(())
    }
}
