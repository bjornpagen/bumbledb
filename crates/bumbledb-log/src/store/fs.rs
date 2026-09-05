//! `FsStore`: the conditional-store verbs over one local directory. This is
//! the ONE implementation of every filesystem operation — its entire
//! read/compare/write/flush/rename/cleanup critical section runs here, in
//! Rust, under a kernel-held mutation lock. No JS layer carries a lock across
//! an await; TS delegates through the shared native executor (C07).
//!
//! Deleted 0.x mechanisms: numeric token/`~lease/<key>/gen` sidecar CAS,
//! `Lease` parsing, age-based temp sweeping and the unconditional-rename TS
//! authority. The head version token is the content hash of the current head
//! bytes; every proposed head body differs through its monotone head
//! revision, so equal-bytes ABA is impossible (chapter 20).
//!
//! Durable ordering per mutation: stage under `~tmp` → fsync file → rename
//! into place → fsync parent directory. A crash leaves the old or the new
//! complete bytes, never a torn head. This adapter is deterministic-test
//! support and a backup destination; it is not a second production database
//! engine — `LocalHistory`'s authority is LMDB.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::fence::{acquire_mutation, sync_parent, synced_temp};
use super::key_ok;
use super::receive::{
    ObservedError, ReceiveAccumulator, ReceiveFault, ReceiveLimits, ReceivedBody, ReceivedHead,
    ReceivingStore, TransportContext, TransportObservation, RECEIVE_CHUNK_BYTES,
};
use crate::writer::verbs::{
    ConditionalOutcome, ConditionalStore, HeadVersion, ListPage, PutOutcome,
};

const PAGE: usize = 1_000;

/// One phase boundary a deterministic schedule can intercept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The critical section holds the mutation lock and has read the current
    /// state, before staging any bytes.
    HeadObserved,
    /// The replacement bytes are staged and fsynced, before the rename.
    Staged,
    /// The rename landed and the directory is durable, before the response.
    Published,
}

/// What an intercepted phase does to the in-flight operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inject {
    Continue,
    /// Fail with a transport error at this boundary. Before `Published` the
    /// mutation is dropped; at `Published` it has already landed.
    Error,
    /// Report `Indeterminate` at this boundary. Before `Published` nothing
    /// landed; at `Published` the response is lost after the effect.
    Indeterminate,
}

type Hook = Box<dyn FnMut(Phase, &str) -> Inject + Send>;

/// The version token of exactly these head bytes.
#[must_use]
pub fn content_version(bytes: &[u8]) -> HeadVersion {
    HeadVersion(Box::from(blake3::hash(bytes).to_hex().as_bytes()))
}

/// The filesystem backend's infrastructure failure.
#[derive(Debug)]
pub struct FsError {
    pub op: &'static str,
    pub key: String,
    pub source: io::Error,
    pub observation: TransportObservation,
}

impl ObservedError for FsError {
    fn observation(&self) -> TransportObservation {
        self.observation
    }
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "filesystem store {} on `{}`: {}",
            self.op, self.key, self.source
        )
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Construction does no I/O and cannot infer abandonment from file age.
pub struct FsStore {
    root: PathBuf,
    hook: Mutex<Option<Hook>>,
}

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            hook: Mutex::new(None),
        }
    }

    /// Install a deterministic schedule hook. Test support only: production
    /// callers never install one.
    pub fn set_hook(&self, hook: impl FnMut(Phase, &str) -> Inject + Send + 'static) {
        *self
            .hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
    }

    fn inject(&self, phase: Phase, key: &str) -> Inject {
        let mut slot = self
            .hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match slot.as_mut() {
            Some(hook) => hook(phase, key),
            None => Inject::Continue,
        }
    }

    fn path_of(&self, op: &'static str, key: &str) -> Result<PathBuf, FsError> {
        if !key_ok(key) {
            return Err(FsError {
                op,
                key: key.to_string(),
                source: io::Error::new(io::ErrorKind::InvalidInput, "invalid store key"),
                observation: TransportObservation::Indeterminate,
            });
        }
        Ok(self.root.join(key))
    }

    fn fail(op: &'static str, key: &str, source: io::Error) -> FsError {
        Self::fail_obs(op, key, source, observe_io(&source))
    }

    fn fail_obs(
        op: &'static str,
        key: &str,
        source: io::Error,
        observation: TransportObservation,
    ) -> FsError {
        FsError {
            op,
            key: key.to_string(),
            source,
            observation,
        }
    }

    fn fail_receive(op: &'static str, key: &str, fault: ReceiveFault) -> FsError {
        let observation = fault.observation();
        Self::fail_obs(op, key, fault.into_io(key), observation)
    }
}

fn observe_io(error: &io::Error) -> TransportObservation {
    match error.kind() {
        io::ErrorKind::NotFound => TransportObservation::Missing,
        io::ErrorKind::PermissionDenied => TransportObservation::Denied,
        _ => TransportObservation::Indeterminate,
    }
}

/// Refuse a symlinked object path: redirection at the owned store boundary
/// is a hostile shape, not a storage location.
fn refuse_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "object path is a symlink",
        )),
        Ok(meta) if meta.is_dir() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "key names a directory",
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// Open a regular file for incremental receive. `None` is definite absence.
/// Stat length is never treated as a receiving bound.
fn open_object(path: &Path) -> io::Result<Option<File>> {
    refuse_symlink(path)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "key is not a regular file",
        ));
    }
    Ok(Some(file))
}

fn receive_file(file: &mut File, acc: &mut ReceiveAccumulator<'_>) -> Result<(), ReceiveFault> {
    let mut chunk = [0u8; RECEIVE_CHUNK_BYTES];
    loop {
        acc.checkpoint()?;
        let want = acc
            .remaining()
            .saturating_add(1)
            .min(RECEIVE_CHUNK_BYTES as u64) as usize;
        if want == 0 {
            return Err(ReceiveFault::Capped {
                cap: acc.len(),
                got: acc.len().saturating_add(1),
            });
        }
        let read = file.read(&mut chunk[..want]).map_err(ReceiveFault::Io)?;
        if read == 0 {
            return Ok(());
        }
        acc.push(&chunk[..read])?;
    }
}

fn receive_path(
    path: &Path,
    ctx: TransportContext<'_>,
) -> Result<Option<ReceivedBody>, ReceiveFault> {
    let Some(mut file) = open_object(path).map_err(ReceiveFault::Io)? else {
        return Ok(None);
    };
    let mut acc = ReceiveAccumulator::new(ctx);
    receive_file(&mut file, &mut acc)?;
    Ok(Some(acc.finish()?))
}

fn file_content_version(path: &Path) -> io::Result<Option<HeadVersion>> {
    let Some(mut file) = open_object(path)? else {
        return Ok(None);
    };
    let mut hasher = blake3::Hasher::new();
    let mut chunk = [0u8; RECEIVE_CHUNK_BYTES];
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        hasher.update(&chunk[..read]);
    }
    Ok(Some(HeadVersion(Box::from(
        hasher.finalize().to_hex().as_bytes(),
    ))))
}

fn contents_equal(path: &Path, expected: &[u8]) -> io::Result<Option<bool>> {
    let Some(mut file) = open_object(path)? else {
        return Ok(None);
    };
    let mut chunk = [0u8; RECEIVE_CHUNK_BYTES];
    let mut offset = 0usize;
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            return Ok(Some(offset == expected.len()));
        }
        let end = offset.saturating_add(read);
        if end > expected.len() || expected[offset..end] != chunk[..read] {
            return Ok(Some(false));
        }
        offset = end;
    }
}

fn ensure_parent(root: &Path, path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "key has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut current = Some(parent);
    while let Some(dir) = current {
        File::open(dir)?.sync_all()?;
        if dir == root {
            break;
        }
        current = dir.parent();
    }
    Ok(())
}

/// Stage-and-rename with full durability: temp under `~tmp`, fsync, rename,
/// parent fsync. The temp's owner removes it on failure; elapsed time never
/// proves another owner abandoned one.
fn publish_bytes(root: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    ensure_parent(root, path)?;
    let temp = synced_temp(root, bytes)?;
    match fs::rename(&temp, path) {
        Ok(()) => sync_parent(path),
        Err(err) => {
            let _ = fs::remove_file(&temp);
            Err(err)
        }
    }
}

impl ConditionalStore for FsStore {
    type Error = FsError;

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, FsError> {
        let path = self.path_of("create_head", head_key)?;
        // The whole critical section — observe, stage, rename, sync — runs
        // under the kernel-held mutation lock. A paused holder remains owner.
        let _lock = acquire_mutation(&self.root, head_key)
            .map_err(|e| Self::fail("create_head", head_key, e))?;
        let existing = open_object(&path)
            .map_err(|e| Self::fail("create_head", head_key, e))?
            .is_some();
        match self.inject(Phase::HeadObserved, head_key) {
            Inject::Continue => {}
            Inject::Error => {
                return Err(Self::fail(
                    "create_head",
                    head_key,
                    io::Error::other("injected transport failure"),
                ));
            }
            Inject::Indeterminate => return Ok(ConditionalOutcome::Indeterminate),
        }
        if existing {
            return Ok(ConditionalOutcome::PreconditionFailed);
        }
        publish_bytes(&self.root, &path, body)
            .map_err(|e| Self::fail("create_head", head_key, e))?;
        match self.inject(Phase::Published, head_key) {
            Inject::Continue => Ok(ConditionalOutcome::Published {
                version: content_version(body),
            }),
            Inject::Error => Err(Self::fail(
                "create_head",
                head_key,
                io::Error::other("injected lost response"),
            )),
            Inject::Indeterminate => Ok(ConditionalOutcome::Indeterminate),
        }
    }

    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, FsError> {
        let path = self.path_of("replace_head", head_key)?;
        let _lock = acquire_mutation(&self.root, head_key)
            .map_err(|e| Self::fail("replace_head", head_key, e))?;
        let current = file_content_version(&path)
            .map_err(|e| Self::fail("replace_head", head_key, e))?;
        match self.inject(Phase::HeadObserved, head_key) {
            Inject::Continue => {}
            Inject::Error => {
                return Err(Self::fail(
                    "replace_head",
                    head_key,
                    io::Error::other("injected transport failure"),
                ));
            }
            Inject::Indeterminate => return Ok(ConditionalOutcome::Indeterminate),
        }
        let Some(current) = current else {
            return Ok(ConditionalOutcome::PreconditionFailed);
        };
        if current != *expected {
            return Ok(ConditionalOutcome::PreconditionFailed);
        }
        match self.inject(Phase::Staged, head_key) {
            Inject::Continue => {}
            Inject::Error => {
                return Err(Self::fail(
                    "replace_head",
                    head_key,
                    io::Error::other("injected transport failure"),
                ));
            }
            Inject::Indeterminate => return Ok(ConditionalOutcome::Indeterminate),
        }
        publish_bytes(&self.root, &path, body)
            .map_err(|e| Self::fail("replace_head", head_key, e))?;
        match self.inject(Phase::Published, head_key) {
            Inject::Continue => Ok(ConditionalOutcome::Published {
                version: content_version(body),
            }),
            Inject::Error => Err(Self::fail(
                "replace_head",
                head_key,
                io::Error::other("injected lost response"),
            )),
            Inject::Indeterminate => Ok(ConditionalOutcome::Indeterminate),
        }
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, FsError> {
        let path = self.path_of("put_object", key)?;
        // Immutable names: an existing identical payload is idempotent
        // evidence; a conflicting payload refuses and never overwrites.
        match contents_equal(&path, body).map_err(|e| Self::fail("put_object", key, e))? {
            Some(true) => return Ok(PutOutcome::Stored),
            Some(false) => {
                return Err(Self::fail_obs(
                    "put_object",
                    key,
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "immutable object holds conflicting bytes",
                    ),
                    TransportObservation::Conflict,
                ));
            }
            None => {}
        }
        match self.inject(Phase::Staged, key) {
            Inject::Continue => {}
            Inject::Error => {
                return Err(Self::fail(
                    "put_object",
                    key,
                    io::Error::other("injected transport failure"),
                ));
            }
            Inject::Indeterminate => return Ok(PutOutcome::Indeterminate),
        }
        publish_bytes(&self.root, &path, body).map_err(|e| Self::fail("put_object", key, e))?;
        match self.inject(Phase::Published, key) {
            Inject::Continue => Ok(PutOutcome::Stored),
            Inject::Error | Inject::Indeterminate => Ok(PutOutcome::Indeterminate),
        }
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, FsError> {
        // Enumerate actual extant names in sorted order, one bounded page.
        // Reserved `~` namespaces are never object names.
        let resume: Option<String> = after.map(|token| String::from_utf8_lossy(token).into_owned());
        let mut keys = Vec::new();
        let mut stack = vec![self.root.clone()];
        let mut entries: Vec<String> = Vec::new();
        while let Some(dir) = stack.pop() {
            let listing = match fs::read_dir(&dir) {
                Ok(listing) => listing,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(Self::fail("list_objects", prefix, err)),
            };
            for entry in listing {
                let entry = entry.map_err(|e| Self::fail("list_objects", prefix, e))?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                if name.starts_with('~') {
                    continue;
                }
                let path = entry.path();
                let kind = entry
                    .file_type()
                    .map_err(|e| Self::fail("list_objects", prefix, e))?;
                if kind.is_dir() {
                    stack.push(path);
                } else if kind.is_file()
                    && let Ok(relative) = path.strip_prefix(&self.root)
                {
                    let key = relative
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    entries.push(key);
                }
            }
        }
        entries.sort();
        for key in entries {
            if !key.starts_with(prefix) {
                continue;
            }
            if let Some(resume) = &resume
                && key.as_str() <= resume.as_str()
            {
                continue;
            }
            keys.push(key);
            if keys.len() == PAGE {
                break;
            }
        }
        let next = if keys.len() == PAGE {
            keys.last().map(|last| Box::from(last.as_bytes()))
        } else {
            None
        };
        Ok(ListPage { keys, next })
    }

    fn delete_object(&self, key: &str) -> Result<(), FsError> {
        let path = self.path_of("delete_object", key)?;
        refuse_symlink(&path).map_err(|e| Self::fail("delete_object", key, e))?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(Self::fail("delete_object", key, err)),
        }
        sync_parent(&path).map_err(|e| Self::fail("delete_object", key, e))
    }
}

impl ReceivingStore for FsStore {
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, FsError> {
        let path = self.path_of("receive_object", key)?;
        match receive_path(&path, ctx) {
            Ok(Some(body)) => Ok(body),
            Ok(None) => Err(Self::fail_obs(
                "receive_object",
                key,
                io::Error::new(io::ErrorKind::NotFound, "object missing"),
                TransportObservation::Missing,
            )),
            Err(fault) => Err(Self::fail_receive("receive_object", key, fault)),
        }
    }

    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, FsError> {
        let path = self.path_of("receive_head", head_key)?;
        match receive_path(&path, ctx) {
            Ok(Some(body)) => Ok(ReceivedHead::Present {
                version: content_version(body.as_bytes()),
                body,
            }),
            Ok(None) => Ok(ReceivedHead::Absent),
            Err(fault) => Err(Self::fail_receive("receive_head", head_key, fault)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path =
            std::env::temp_dir().join(format!("bdb-log-fs2-{tag}-{}-{nanos}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("root");
        path
    }

    #[test]
    fn stale_version_replacement_is_a_definite_loss_never_an_acknowledgment() {
        let root = scratch("stale");
        let store = FsStore::new(&root);
        let v1 = match store.create_head("t/HEAD", b"rev1").unwrap() {
            ConditionalOutcome::Published { version } => version,
            other => panic!("{other:?}"),
        };
        // A second writer wins the head.
        match store.replace_head("t/HEAD", &v1, b"rev2").unwrap() {
            ConditionalOutcome::Published { .. } => {}
            other => panic!("{other:?}"),
        }
        // The paused first writer resumes with its captured old version: the
        // 0.x mixed-fleet bug acknowledged both updates; the successor loses.
        assert_eq!(
            store.replace_head("t/HEAD", &v1, b"rev1b").unwrap(),
            ConditionalOutcome::PreconditionFailed
        );
        match store
            .receive_head("t/HEAD", TransportContext::limited(64))
            .unwrap()
        {
            ReceivedHead::Present { body, .. } => assert_eq!(body.as_bytes(), b"rev2"),
            ReceivedHead::Absent => panic!("head exists"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn immutable_objects_are_idempotent_and_conflicts_refuse() {
        let root = scratch("immutable");
        let store = FsStore::new(&root);
        assert_eq!(
            store.put_object("t/objects/1/chunk/aa", b"bytes").unwrap(),
            PutOutcome::Stored
        );
        assert_eq!(
            store.put_object("t/objects/1/chunk/aa", b"bytes").unwrap(),
            PutOutcome::Stored,
            "re-putting identical bytes is idempotent evidence"
        );
        let conflict = store.put_object("t/objects/1/chunk/aa", b"other");
        assert!(conflict.is_err(), "creation never overwrites a conflict");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn hostile_lock_shapes_refuse_or_are_inert() {
        use std::os::unix::fs::symlink;
        let root = scratch("hostile");
        let store = FsStore::new(&root);
        // Garbage lock-body content is inert: the kernel lock is the
        // authority; no content parse grants or denies mutation.
        fs::create_dir_all(root.join("~lease/t/HEAD")).unwrap();
        fs::write(
            root.join("~lease/t/HEAD/mutation.lock"),
            b"POISON not-a-lease",
        )
        .unwrap();
        assert!(matches!(
            store.create_head("t/HEAD", b"rev1").unwrap(),
            ConditionalOutcome::Published { .. }
        ));
        // A symlinked mutation lock refuses the operation outright.
        let sentinel = root.join("elsewhere.bin");
        fs::write(&sentinel, b"sentinel").unwrap();
        fs::create_dir_all(root.join("~lease/u/HEAD")).unwrap();
        symlink(&sentinel, root.join("~lease/u/HEAD/mutation.lock")).unwrap();
        assert!(store.create_head("u/HEAD", b"rev1").is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"sentinel");
        // A symlinked object path refuses reads and writes.
        symlink(&sentinel, root.join("v")).unwrap();
        assert!(store
            .receive_object("v", TransportContext::limited(64))
            .is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn keys_cannot_spell_reserved_namespaces() {
        let root = scratch("reserved");
        let store = FsStore::new(&root);
        for key in ["~tmp/x", "~lease/y", "a/~tmp/z", "a/b.lock", "a//b", "../a"] {
            assert!(store.put_object(key, b"x").is_err(), "{key}");
            assert!(
                store
                    .receive_object(key, TransportContext::limited(64))
                    .is_err(),
                "{key}"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn receive_stops_at_the_envelope_without_trusting_stat_length() {
        let root = scratch("receive-cap");
        let store = FsStore::new(&root);
        store
            .put_object("t/objects/1/chunk/aa", b"0123456789")
            .unwrap();
        let error = store
            .receive_object(
                "t/objects/1/chunk/aa",
                TransportContext {
                    work: None,
                    receive: ReceiveLimits::capped(4),
                },
            )
            .expect_err("cap");
        assert_eq!(error.observation, TransportObservation::Capped);
        let missing = store
            .receive_object(
                "t/objects/1/chunk/zz",
                TransportContext {
                    work: None,
                    receive: ReceiveLimits::capped(8),
                },
            )
            .expect_err("missing");
        assert_eq!(missing.observation, TransportObservation::Missing);
        let _ = fs::remove_dir_all(&root);
    }
}
