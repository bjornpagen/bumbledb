//! Fenced CAS leases on the local filesystem. The lease identity is a
//! monotonic token created with exclusive `link`; a contender mints the
//! next token iff the current lease's own bytes are expired.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::{jittered, unix_ms, Lease, WriterId, LEASE_NAMESPACE, TEMP_NAMESPACE};

/// How long a mutation lease stays current, in milliseconds.
pub const MUTATION_TTL_MS: u64 = 5_000;

/// A live `synced_temp` exists only for write-then-link. Anything
/// older than this is crash litter. Sweep deletes those files and
/// never the whole `~tmp` tree: constructors share the root.
pub const TEMP_STALE_MS: u64 = 30_000;

/// How long a directory exclusivity lease stays current, in milliseconds.
pub const DIR_TTL_MS: u64 = 300_000;

/// Ceiling of the jittered wait for a live mutation lease.
const LOCK_RETRY_MS: u64 = 10;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// A held mutation or directory lease. Drop releases by writing an
/// already-expired successor so the next acquirer does not wait us out.
pub struct HeldLease {
    root: PathBuf,
    dir: PathBuf,
    token: u64,
    holder: WriterId,
}

/// Why acquire refused without waiting.
#[derive(Debug)]
pub enum LeaseBusy {
    Live,
    Io(io::Error),
}

impl From<io::Error> for LeaseBusy {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Exclusive temp under `{root}/~tmp`, fsynced.
///
/// # Errors
pub fn synced_temp(root: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let dir = root.join(TEMP_NAMESPACE);
    fs::create_dir_all(&dir)?;
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let temp = dir.join(format!("{}.{}", std::process::id(), seq));
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

/// Fsync `path`'s parent directory.
///
/// # Errors
pub fn sync_parent(path: &Path) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent"))?;
    File::open(dir)?.sync_all()
}

/// Fsync every ancestor of `path` up to and including `root`.
///
/// # Errors
pub fn sync_ancestors(path: &Path, root: &Path) -> io::Result<()> {
    let mut current = path.parent();
    while let Some(dir) = current {
        File::open(dir)?.sync_all()?;
        if dir == root {
            break;
        }
        current = dir.parent();
    }
    Ok(())
}

/// Sweep crash litter at paths this process already names: stale
/// `{pid}.{seq}` temps, and superseded tokens under `~lease` once
/// `~head` names the current exclusivity token.
///
/// # Errors
pub fn sweep_reserved(root: &Path) -> io::Result<()> {
    sweep_stale_temps(&root.join(TEMP_NAMESPACE));
    sweep_owned_predecessors(&root.join(LEASE_NAMESPACE))?;
    Ok(())
}

fn sweep_stale_temps(dir: &Path) {
    let stale = Duration::from_millis(TEMP_STALE_MS);
    let pid = std::process::id();
    let end = TEMP_SEQ.load(Ordering::Relaxed);
    for seq in 0..end {
        let path = dir.join(format!("{pid}.{seq}"));
        let Ok(modified) = fs::metadata(&path).and_then(|meta| meta.modified()) else {
            continue;
        };
        if modified.elapsed().is_ok_and(|age| age > stale) {
            let _ = fs::remove_file(&path);
        }
    }
}

/// `~head` is not a `StoreKey`. It names the current token so a
/// successor GETs `dir/{n}`.
const HEAD: &str = "~head";

fn head_path(dir: &Path) -> PathBuf {
    dir.join(HEAD)
}

fn read_head(dir: &Path) -> io::Result<Option<u64>> {
    match fs::read(head_path(dir)) {
        Ok(bytes) => Ok(std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .filter(|token| *token >= 1)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn write_head(root: &Path, dir: &Path, token: u64) -> io::Result<()> {
    let dest = head_path(dir);
    let body = token.to_string();
    let temp = synced_temp(root, body.as_bytes())?;
    if let Err(err) = fs::rename(&temp, &dest).and_then(|()| sync_parent(&dest)) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

fn sweep_owned_predecessors(dir: &Path) -> io::Result<()> {
    let Some((token, _)) = current_lease(dir)? else {
        return Ok(());
    };
    forget_predecessors(dir, token);
    Ok(())
}

/// Removes `dir/{1..=current-1}` after `~head` names `current`.
fn forget_predecessors(dir: &Path, current: u64) {
    for token in (1..current).rev() {
        let _ = fs::remove_file(token_path(dir, token));
    }
}

fn token_path(dir: &Path, token: u64) -> PathBuf {
    dir.join(token.to_string())
}

/// The current lease is `dir/{n}` for the token `~head` names, or the
/// highest `dir/{n}` at or after that hint. A mint past a stale head
/// is still visible: the probe opens `n`, `n+1`, … until a gap.
fn current_lease(dir: &Path) -> io::Result<Option<(u64, Lease)>> {
    let start = read_head(dir)?.unwrap_or(1);
    Ok(match probe_from(dir, start) {
        None if start > 1 => probe_from(dir, 1),
        found => found,
    })
}

fn probe_from(dir: &Path, start: u64) -> Option<(u64, Lease)> {
    let mut best: Option<(u64, Lease)> = None;
    let mut token = start;
    loop {
        let path = token_path(dir, token);
        if path.is_dir() {
            token = match token.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        }
        match fs::read(&path) {
            Ok(bytes) => {
                if let Some(lease) = Lease::parse(&bytes) {
                    best = Some((token, lease));
                }
                token = match token.checked_add(1) {
                    Some(next) => next,
                    None => break,
                };
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => break,
            Err(_) => {
                token = match token.checked_add(1) {
                    Some(next) => next,
                    None => break,
                };
            }
        }
    }
    best
}

fn try_mint(
    root: &Path,
    dir: &Path,
    token: u64,
    holder: WriterId,
    ttl_ms: u64,
) -> io::Result<bool> {
    fs::create_dir_all(dir)?;
    let body = Lease {
        holder,
        token,
        expires: unix_ms().saturating_add(ttl_ms),
    }
    .encode();
    let dest = token_path(dir, token);
    let temp = synced_temp(root, &body)?;
    match fs::hard_link(&temp, &dest) {
        Ok(()) => {
            let _ = fs::remove_file(&temp);
            sync_parent(&dest)?;
            if write_head(root, dir, token).is_ok() {
                forget_predecessors(dir, token);
            }
            Ok(true)
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(&temp);
            Ok(false)
        }
        Err(err) => {
            let _ = fs::remove_file(&temp);
            Err(err)
        }
    }
}

fn acquire_once(
    root: &Path,
    dir: &Path,
    holder: WriterId,
    ttl_ms: u64,
) -> Result<Option<HeldLease>, LeaseBusy> {
    match current_lease(dir)? {
        None => {
            if try_mint(root, dir, 1, holder, ttl_ms)? {
                Ok(Some(HeldLease {
                    root: root.to_path_buf(),
                    dir: dir.to_path_buf(),
                    token: 1,
                    holder,
                }))
            } else {
                Ok(None)
            }
        }
        Some((_token, lease)) if !lease.breakable(unix_ms()) => Err(LeaseBusy::Live),
        Some((token, _)) => {
            let next = token.saturating_add(1);
            if try_mint(root, dir, next, holder, ttl_ms)? {
                Ok(Some(HeldLease {
                    root: root.to_path_buf(),
                    dir: dir.to_path_buf(),
                    token: next,
                    holder,
                }))
            } else {
                Ok(None)
            }
        }
    }
}

/// Wait out a live mutation lease; take it when expired or absent.
///
/// # Errors
pub fn acquire_mutation(root: &Path, key: &str, holder: WriterId) -> io::Result<HeldLease> {
    let dir = root.join(LEASE_NAMESPACE).join(key);
    loop {
        match acquire_once(root, &dir, holder, MUTATION_TTL_MS) {
            Ok(Some(held)) => return Ok(held),
            Ok(None) => {}
            Err(LeaseBusy::Live) => {
                std::thread::sleep(jittered(Duration::from_millis(LOCK_RETRY_MS)));
            }
            Err(LeaseBusy::Io(err)) => return Err(err),
        }
    }
}

/// One-shot directory exclusivity: a live holder is `Live`, not waited.
///
/// # Errors
pub fn acquire_dir(root: &Path, holder: WriterId) -> Result<HeldLease, LeaseBusy> {
    let dir = root.join(LEASE_NAMESPACE);
    loop {
        match acquire_once(root, &dir, holder, DIR_TTL_MS) {
            Ok(Some(held)) => return Ok(held),
            Ok(None) => {}
            Err(busy) => return Err(busy),
        }
    }
}

impl HeldLease {
    /// The fencing token this holder's writes carry.
    #[must_use]
    pub const fn token(&self) -> u64 {
        self.token
    }

    /// True iff this token is still the max — a stale holder lost the
    /// CAS and must not publish.
    ///
    /// # Errors
    pub fn still_current(&self) -> io::Result<bool> {
        match current_lease(&self.dir)? {
            Some((token, _)) => Ok(token == self.token),
            None => Ok(false),
        }
    }

    /// Push `expires` forward under the same token (directory heartbeat).
    ///
    /// # Errors
    pub fn refresh(&self, ttl_ms: u64) -> io::Result<()> {
        if !self.still_current()? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "lease token is no longer current",
            ));
        }
        let body = Lease {
            holder: self.holder,
            token: self.token,
            expires: unix_ms().saturating_add(ttl_ms),
        }
        .encode();
        let dest = token_path(&self.dir, self.token);
        let temp = synced_temp(&self.root, &body)?;
        if let Err(err) = fs::rename(&temp, &dest).and_then(|()| sync_parent(&dest)) {
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for HeldLease {
    fn drop(&mut self) {
        let body = Lease {
            holder: self.holder,
            token: self.token,
            expires: 0,
        }
        .encode();
        if let Ok(temp) = synced_temp(&self.root, &body) {
            let dest = token_path(&self.dir, self.token);
            if fs::rename(&temp, &dest).is_err() {
                let _ = fs::remove_file(&temp);
            } else {
                let _ = sync_parent(&dest);
            }
        }
    }
}
