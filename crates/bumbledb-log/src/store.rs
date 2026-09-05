//! The backend seam (C07): typed object identity, the one object-key
//! namespace, and verified immutable object I/O over the conditional-store
//! verbs declared in [`crate::writer::verbs`].
//!
//! P05 owns this composition: the concrete adapters ([`mem`], [`fs`], [`s3`]),
//! their durability ordering and fault taxonomy, the object-key grammar, and
//! the verification rule every reader applies before interpreting bytes. The
//! publication machine (P04) consumes only the verb trait and its three-way
//! conditional grammar.
//!
//! Deleted mechanisms from the 0.x store: the generic five-verb `ObjectStore`
//! framework, numeric token/etag CAS leases, fresh-id lease counters, expiring
//! mutation leases, age-based temp sweeping, whole-body `receive_whole` /
//! `receive_head_whole` production reads, the uncharged three-argument
//! `get_verified`, and `get_verified_ctx`. Local exclusion is a kernel-held
//! lock ([`fence`]); object mutation and directory ownership are distinct
//! scopes even though both use the same kernel mechanism. Production
//! HEAD/object reads are only [`ReceivingStore::receive_head`] /
//! [`ReceivingStore::receive_object`]. Those verbs report
//! [`TransportObservation`]; they do not emit a publication verdict.
//! [`get_verified`] keeps the receive reservation on [`ChargedBytes`] until
//! the caller decodes or drops it.

pub mod fence;
pub mod fs;
pub mod mem;
pub mod receive;
#[cfg(feature = "store")]
pub mod s3;

pub use bumbledb::work::ChargedBytes;
pub use fence::{
    DirectoryLock, HeldLock, LockIdentity, RepositoryLock, acquire_directory,
    acquire_repository_lock,
};
pub use receive::{
    ObservedError, ReceiveAccumulator, ReceiveFault, ReceiveLimits, ReceivedBody, ReceivedHead,
    ReceivingStore, TransportContext, TransportObservation, RECEIVE_CHUNK_BYTES,
};

use std::fmt;

use crate::history::{DecisionDigest, FrameError};
pub use crate::writer::verbs::{
    ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion, ListPage, ObjectRead, PutOutcome,
};

/// Reserved first path segments no object key may spell. Ownership locks and
/// staging temps live here, disjoint from every honest object name.
pub const LEASE_NAMESPACE: &str = "~lease";
pub const TEMP_NAMESPACE: &str = "~tmp";

/// The head object's name under a database prefix. Never deleted or reused
/// during an incarnation's operational life; `Deleted` is a tombstone state.
pub const HEAD_NAME: &str = "HEAD";

/// The kinds of immutable protocol objects. The kind is part of the storage
/// name and of the digest domain: the same bytes under another kind are a
/// different object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectKind {
    /// One immutable terminal decision (P04 frames its bytes).
    Decision,
    /// One fixed-target checkpoint stream chunk.
    Chunk,
    /// One streamed checkpoint manifest / snapshot certificate.
    Checkpoint,
    /// One immutable GC mark manifest.
    Mark,
}

impl ObjectKind {
    /// The path segment spelling of this kind. Lower-case ASCII, protocol
    /// generated — never a user label.
    #[must_use]
    pub const fn segment(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Chunk => "chunk",
            Self::Checkpoint => "ckpt",
            Self::Mark => "mark",
        }
    }

    /// The digest domain separating this kind's content addresses.
    #[must_use]
    pub const fn digest_domain(self) -> &'static str {
        match self {
            // Exactly the domain the history machine stamps decisions with.
            Self::Decision => "bumbledb.decision.v1/decision-digest",
            Self::Chunk => "bumbledb.object.v1/chunk",
            Self::Checkpoint => "bumbledb.object.v1/ckpt",
            Self::Mark => "bumbledb.object.v1/mark",
        }
    }

    #[must_use]
    pub fn parse_segment(segment: &str) -> Option<Self> {
        match segment {
            "decision" => Some(Self::Decision),
            "chunk" => Some(Self::Chunk),
            "ckpt" => Some(Self::Checkpoint),
            "mark" => Some(Self::Mark),
            _ => None,
        }
    }
}

/// Wire width: 8 epoch + 1 kind + 32 digest + 8 length = 49 (C6).
/// An option adds one tag: absent 1, present 50.
pub const OBJECT_REF_WIRE_BYTES: usize = 49;
pub const OBJECT_REF_OPTION_PRESENT_BYTES: usize = 50;

/// Typed, verified immutable object identity: epoch, kind, full 32-byte
/// content digest and expected length. The digest alone is not a storage key;
/// the same digest at another epoch is a distinct storage name. Readers
/// verify length and domain-separated digest before interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRef {
    pub epoch: u64,
    pub kind: ObjectKind,
    pub digest: [u8; 32],
    pub length: u64,
}

impl ObjectRef {
    /// The reference for exactly these bytes staged under `epoch`.
    #[must_use]
    pub fn of(epoch: u64, kind: ObjectKind, bytes: &[u8]) -> Self {
        Self {
            epoch,
            kind,
            digest: object_digest(kind, bytes),
            length: bytes.len() as u64,
        }
    }

    /// The storage key of this object under a database prefix.
    #[must_use]
    pub fn key(&self, prefix: &str) -> String {
        object_key(prefix, self.epoch, self.kind, &self.digest)
    }
}

/// The domain-separated content digest for one object kind.
#[must_use]
pub fn object_digest(kind: ObjectKind, bytes: &[u8]) -> [u8; 32] {
    blake3::derive_key(kind.digest_domain(), bytes)
}

/// `<prefix>/HEAD` — the one mutable object of a database incarnation.
#[must_use]
pub fn head_key(prefix: &str) -> String {
    format!("{prefix}/{HEAD_NAME}")
}

/// `<prefix>/objects/<epoch>/<kind>/<digest-hex>` — canonical lower-case
/// ASCII encoding of binary identities, never a user tenant name.
#[must_use]
pub fn object_key(prefix: &str, epoch: u64, kind: ObjectKind, digest: &[u8; 32]) -> String {
    format!(
        "{prefix}/objects/{epoch}/{}/{}",
        kind.segment(),
        hex32(digest)
    )
}

/// The listing prefix under which every immutable object of a database lives.
#[must_use]
pub fn objects_prefix(prefix: &str) -> String {
    format!("{prefix}/objects/")
}

/// Parse an `objects/` key back into its identity. Returns `None` for keys
/// outside the recognized namespace — sweep never deletes what it cannot
/// parse.
#[must_use]
pub fn parse_object_key(prefix: &str, key: &str) -> Option<(u64, ObjectKind, [u8; 32])> {
    let rest = key.strip_prefix(prefix)?.strip_prefix('/')?;
    let rest = rest.strip_prefix("objects/")?;
    let mut parts = rest.split('/');
    let epoch_text = parts.next()?;
    // Canonical decimal only: no signs, no leading zeros beyond "0" itself.
    if epoch_text.is_empty()
        || (epoch_text.len() > 1 && epoch_text.starts_with('0'))
        || !epoch_text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let epoch: u64 = epoch_text.parse().ok()?;
    let kind = ObjectKind::parse_segment(parts.next()?)?;
    let hex = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let digest = parse_hex32(hex)?;
    Some((epoch, kind, digest))
}

/// Lower-case hex of a 32-byte digest.
#[must_use]
pub fn hex32(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[must_use]
fn parse_hex32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut digest = [0u8; 32];
    for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
        let high = hex_nibble(chunk[0])?;
        let low = hex_nibble(chunk[1])?;
        digest[index] = (high << 4) | low;
    }
    Some(digest)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        // Lower-case only: the canonical encoding, not a case-folding alias.
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// The decision object key spelled exactly like the publication machine's.
#[must_use]
pub fn decision_key(prefix: &str, epoch: u64, digest: &DecisionDigest) -> String {
    object_key(prefix, epoch, ObjectKind::Decision, digest.as_bytes())
}

/// The one shared object-layer failure taxonomy over any backend. `Backend`
/// wraps the adapter's own infrastructure failure and never claims a definite
/// protocol outcome; every other arm is a definite verified refusal.
#[derive(Debug)]
pub enum ObjectError {
    /// Transport/auth/IO with no definite observation.
    Backend(Box<dyn std::error::Error + Send + Sync>),
    /// A referenced object is definitely absent.
    Missing { key: String },
    /// The fetched bytes disagree with the reference's expected length.
    WrongLength {
        key: String,
        expected: u64,
        got: u64,
    },
    /// The fetched bytes disagree with the reference's content digest.
    WrongDigest { key: String },
    /// An immutable name already holds conflicting bytes; creation refused.
    ImmutableConflict { key: String },
    /// A frame-level grammar refusal from a nested record.
    Frame(FrameError),
    /// A stored object could not be proven durable within the retry budget.
    Unverified { key: String },
    /// Access was refused. Not a publication verdict.
    Denied { key: String },
    /// The bucket is not addressable. Not a missing object.
    Bucket { key: String },
    /// Region or endpoint mismatch. Not a missing object.
    Region { key: String },
}

impl fmt::Display for ObjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "object backend: {error}"),
            Self::Missing { key } => write!(f, "object missing: {key}"),
            Self::WrongLength { key, expected, got } => {
                write!(f, "object {key} length {got}, expected {expected}")
            }
            Self::WrongDigest { key } => write!(f, "object digest mismatch: {key}"),
            Self::ImmutableConflict { key } => {
                write!(f, "immutable object holds conflicting bytes: {key}")
            }
            Self::Frame(error) => write!(f, "object frame: {error:?}"),
            Self::Unverified { key } => write!(f, "object not proven durable: {key}"),
            Self::Denied { key } => write!(f, "object denied: {key}"),
            Self::Bucket { key } => write!(f, "object bucket: {key}"),
            Self::Region { key } => write!(f, "object region: {key}"),
        }
    }
}

impl std::error::Error for ObjectError {}

impl From<FrameError> for ObjectError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

/// Adapter-error bound all P05 drivers use. The C07 trait leaves `Error`
/// open; the composition requires a real error value it can retain as cause.
pub trait BackendError: std::error::Error + Send + Sync + 'static {}
impl<T: std::error::Error + Send + Sync + 'static> BackendError for T {}

pub(crate) fn backend<E: BackendError>(error: E) -> ObjectError {
    ObjectError::Backend(Box::new(error))
}

/// Store one immutable content-addressed object and prove it durable.
///
/// Immutable objects may use verified content equality as publication
/// evidence: an ambiguous PUT (or a re-PUT of identical bytes) is resolved by
/// reading the key back and comparing content. Conflicting existing bytes at
/// the same name refuse — creation never overwrites a colliding payload.
///
/// # Errors
/// Backend failure with no definite outcome, an immutable conflict, or an
/// unresolved ambiguous store.
pub fn put_verified<B: ReceivingStore>(
    backend_store: &B,
    prefix: &str,
    epoch: u64,
    kind: ObjectKind,
    bytes: &[u8],
) -> Result<ObjectRef, ObjectError>
where
    B::Error: BackendError + ObservedError,
{
    let reference = ObjectRef::of(epoch, kind, bytes);
    let key = reference.key(prefix);
    match backend_store.put_object(&key, bytes).map_err(backend)? {
        PutOutcome::Stored => {}
        PutOutcome::Indeterminate => {
            match backend_store.receive_object(
                &key,
                TransportContext {
                    work: None,
                    receive: ReceiveLimits::exact(bytes.len() as u64),
                },
            ) {
                Ok(body) if body.as_bytes() == bytes => {}
                Ok(_) => return Err(ObjectError::ImmutableConflict { key }),
                Err(error) => match error.observation() {
                    TransportObservation::Missing => {
                        return Err(ObjectError::Unverified { key });
                    }
                    TransportObservation::Capped | TransportObservation::Conflict => {
                        return Err(ObjectError::ImmutableConflict { key });
                    }
                    _ => return Err(backend(error)),
                },
            }
        }
    }
    Ok(reference)
}

/// Fetch one referenced object and verify its length and domain-separated
/// digest. The caller's [`TransportContext`] work and envelope bind the
/// read; the receive reservation stays on the returned [`ChargedBytes`]
/// until the caller decodes or drops it.
///
/// # Errors
/// Missing work context, backend failure, definite absence, cap overrun,
/// or verification refusal.
pub fn get_verified<B: ReceivingStore>(
    backend_store: &B,
    prefix: &str,
    reference: &ObjectRef,
    ctx: TransportContext<'_>,
) -> Result<ChargedBytes, ObjectError>
where
    B::Error: BackendError + ObservedError,
{
    if ctx.work.is_none() {
        return Err(backend(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "get_verified requires WorkContext",
        )));
    }
    let key = reference.key(prefix);
    let receive = ReceiveLimits::capped(ctx.receive.max_bytes.min(reference.length));
    let body = backend_store
        .receive_object(
            &key,
            TransportContext {
                work: ctx.work,
                receive,
            },
        )
        .map_err(|error| map_receive(&key, error))?;
    receive::verify_body(&key, reference.kind, reference, body.as_bytes())?;
    body.into_charged().ok_or_else(|| {
        backend(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "receive dropped its reservation before decode",
        ))
    })
}

fn map_receive<E: BackendError + ObservedError>(key: &str, error: E) -> ObjectError {
    match error.observation() {
        TransportObservation::Missing => ObjectError::Missing {
            key: key.to_string(),
        },
        TransportObservation::Denied => ObjectError::Denied {
            key: key.to_string(),
        },
        TransportObservation::Bucket => ObjectError::Bucket {
            key: key.to_string(),
        },
        TransportObservation::Region => ObjectError::Region {
            key: key.to_string(),
        },
        _ => backend(error),
    }
}

/// Read one head under an explicit byte cap during receive.
///
/// # Errors
/// Backend failure, cap overrun, or transport refusal.
pub fn read_head_bounded<B: ReceivingStore>(
    backend_store: &B,
    head_key: &str,
    ctx: TransportContext<'_>,
) -> Result<ReceivedHead, ObjectError>
where
    B::Error: BackendError + ObservedError,
{
    backend_store
        .receive_head(head_key, ctx)
        .map_err(|error| map_receive(head_key, error))
}

/// Fetch one decision object by its authenticated locator. One GET, bounded
/// work — no epoch probing.
///
/// # Errors
/// Backend failure, definite absence, or verification refusal.
pub fn fetch_decision_ref<B: ReceivingStore>(
    backend_store: &B,
    prefix: &str,
    reference: &ObjectRef,
    ctx: TransportContext<'_>,
) -> Result<ChargedBytes, ObjectError>
where
    B::Error: BackendError + ObservedError,
{
    if reference.kind != ObjectKind::Decision {
        return Err(ObjectError::Frame(FrameError::InvalidCount));
    }
    get_verified(backend_store, prefix, reference, ctx)
}

/// Fetch one decision object by digest, probing the bounded epoch window
/// `[floor, ceiling]` newest-first. Test-only epoch-probe helper — production
/// paths use [`fetch_decision_ref`] with authenticated locators
/// ([`crate::history::locator::walk_decision_chain`]).
///
/// # Errors
/// Backend failure or absence across the whole window.
#[cfg(test)]
pub fn fetch_decision<B: ReceivingStore>(
    backend_store: &B,
    prefix: &str,
    epoch_floor: u64,
    epoch_ceiling: u64,
    digest: &DecisionDigest,
) -> Result<(u64, Vec<u8>), ObjectError>
where
    B::Error: BackendError + ObservedError,
{
    if epoch_floor > epoch_ceiling {
        return Err(ObjectError::Frame(FrameError::InvalidEpoch));
    }
    let mut epoch = epoch_ceiling;
    loop {
        let key = decision_key(prefix, epoch, digest);
        match backend_store.receive_object(&key, TransportContext::limited(1 << 20)) {
            Ok(body) => {
                // Decisions are content addressed under their own digest
                // domain; verify before interpretation.
                if object_digest(ObjectKind::Decision, body.as_bytes()) != *digest.as_bytes() {
                    return Err(ObjectError::WrongDigest { key });
                }
                return Ok((epoch, body.as_bytes().to_vec()));
            }
            Err(error) if error.observation() == TransportObservation::Missing => {
                if epoch == epoch_floor {
                    return Err(ObjectError::Missing { key });
                }
                epoch -= 1;
            }
            Err(error) => return Err(map_receive(&key, error)),
        }
    }
}

/// One object-key path segment’s validity: nonempty, no separators/controls,
/// no dot traversal, no reserved `~` prefix, no `.lock` suffix. Adapters
/// refuse keys whose segments fail this before touching storage.
#[must_use]
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the object-key grammar is byte-exact; `.lock` is a reserved \
              literal suffix in that grammar, not a filename extension"
)]
pub fn segment_ok(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.starts_with('~')
        && !segment.ends_with(".lock")
        && !segment
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '/' || c == '\\')
}

/// A whole slash-path object key's validity.
#[must_use]
pub fn key_ok(key: &str) -> bool {
    !key.is_empty()
        && !key.starts_with('/')
        && !key.ends_with('/')
        && key.split('/').all(segment_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_keys_roundtrip_and_reserved_or_hostile_spellings_refuse() {
        let digest = [0xabu8; 32];
        let key = object_key("log/t1", 7, ObjectKind::Chunk, &digest);
        assert_eq!(
            parse_object_key("log/t1", &key),
            Some((7, ObjectKind::Chunk, digest))
        );
        // Foreign prefix, unknown kind, bad hex, non-canonical epoch.
        assert_eq!(parse_object_key("log/t2", &key), None);
        assert_eq!(
            parse_object_key("log/t1", "log/t1/objects/7/braid/aa"),
            None
        );
        assert_eq!(
            parse_object_key(
                "log/t1",
                &format!("log/t1/objects/07/chunk/{}", hex32(&digest))
            ),
            None,
            "leading-zero epochs are not canonical names"
        );
        assert_eq!(
            parse_object_key(
                "log/t1",
                &format!("log/t1/objects/7/chunk/{}", hex32(&digest).to_uppercase())
            ),
            None,
            "upper-case hex is not the canonical encoding"
        );
        assert!(key_ok(&key));
        for bad in [
            "~tmp/x",
            "a/~lease/b",
            "a//b",
            "a/./b",
            "x.lock",
            "a b",
            "/a",
            "a/",
        ] {
            assert!(!key_ok(bad), "{bad}");
        }
    }

    #[test]
    fn digests_are_domain_separated_by_kind() {
        let bytes = b"same bytes";
        assert_ne!(
            object_digest(ObjectKind::Chunk, bytes),
            object_digest(ObjectKind::Checkpoint, bytes)
        );
        let reference = ObjectRef::of(3, ObjectKind::Mark, bytes);
        assert_eq!(reference.length, bytes.len() as u64);
        assert_eq!(reference.digest, object_digest(ObjectKind::Mark, bytes));
    }

    #[test]
    fn same_digest_at_another_epoch_is_a_distinct_storage_name() {
        let digest = [1u8; 32];
        assert_ne!(
            object_key("p", 1, ObjectKind::Chunk, &digest),
            object_key("p", 2, ObjectKind::Chunk, &digest)
        );
    }
}
