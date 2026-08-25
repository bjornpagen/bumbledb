//! The object-store capability: the protocol's demand, not a vendor's
//! offer. Five verbs, all outcomes sums; `Err` carries infrastructure
//! failure (network, 5xx, auth, io) and nothing else. The verbs are
//! synchronous: an impl that drives an async runtime refuses a call
//! from an async context instead of `block_on`-panicking.

pub mod fence;
pub mod fs;
pub mod mem;
pub mod s3;

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// A segment wearing this suffix is not a key. Old lockfile names stay
/// unaddressable; the mutation lock is a fenced CAS lease, not a path.
pub const LOCK_SUFFIX: &str = ".lock";

/// Reserved first-segment names no [`StoreKey`] can spell. Temps and
/// leases live here, disjoint from every honest key.
pub const TEMP_NAMESPACE: &str = "~tmp";
pub const LEASE_NAMESPACE: &str = "~lease";

/// A slash-path object key, parsed once. Empty segments, dot segments,
/// a leading or trailing slash, a reserved `~` segment, a control
/// character, and a segment wearing the lockfile suffix are
/// unrepresentable — the verbs take the proof and never re-check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreKey(String);

/// A key spelling the parse refused.
#[derive(Debug)]
pub struct KeyError {
    pub key: String,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "store key is not a slash path: {}", self.key)
    }
}

impl std::error::Error for KeyError {}

impl StoreKey {
    /// Parse at the boundary. Every later verb takes the proof.
    pub fn parse(raw: &str) -> std::result::Result<Self, KeyError> {
        let well_formed = !raw.is_empty()
            && !raw.starts_with('/')
            && !raw.ends_with('/')
            && raw.split('/').all(segment_ok);
        if well_formed {
            Ok(Self(raw.to_string()))
        } else {
            Err(KeyError {
                key: raw.to_string(),
            })
        }
    }

    /// Protocol and fixture assembly: a well-formed key is a
    /// programming error to get wrong, not a runtime outcome.
    #[must_use]
    pub fn of(raw: &str) -> Self {
        Self::parse(raw).unwrap_or_else(|err| panic!("{err}"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One path segment of a key or a tenant id: the same grammar.
#[must_use]
pub fn segment_ok(seg: &str) -> bool {
    !seg.is_empty()
        && !seg.contains('/')
        && seg != "."
        && seg != ".."
        && !seg.starts_with('~')
        && !seg.ends_with(LOCK_SUFFIX)
        && !seg.chars().any(char::is_control)
}

/// A store prefix: empty, or a [`StoreKey`] spelling (the same segment
/// grammar, no leading or trailing slash).
pub fn parse_prefix(raw: &str) -> std::result::Result<String, KeyError> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    let trimmed = raw.trim_matches('/');
    StoreKey::parse(trimmed).map(|key| key.0)
}

impl AsRef<str> for StoreKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StoreKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Process- or writer-scoped identity carried on a [`Lease`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WriterId(pub u64);

/// Whether a foreign process can be treated as gone. `Unknown` never
/// breaks a lease — expiry of the lease's own bytes is the only break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    Alive,
    Dead,
    Unknown,
}

/// A fenced CAS lease: identity is the token, not a path. Acquired and
/// broken only through exclusive create of the next token; a contender
/// takes the next token iff the current lease is expired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub holder: WriterId,
    pub token: u64,
    pub expires: u64,
}

impl Lease {
    #[must_use]
    pub fn expired(&self, now_ms: u64) -> bool {
        self.expires <= now_ms
    }

    /// Expiry of the lease's own bytes. The lock is not a probe.
    #[must_use]
    pub fn breakable(&self, now_ms: u64) -> bool {
        self.expired(now_ms)
    }

    /// A foreign-process probe never breaks on [`Liveness::Unknown`]
    /// or [`Liveness::Alive`]. Only `Dead` plus expiry yields a break,
    /// and the mutation lock does not call this — it uses expiry alone.
    #[must_use]
    pub fn break_on_probe(&self, now_ms: u64, liveness: Liveness) -> bool {
        matches!(liveness, Liveness::Dead) && self.expired(now_ms)
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        format!(
            "LEASE/1\n{}\n{}\n{}\n",
            self.holder.0, self.token, self.expires
        )
        .into_bytes()
    }

    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut lines = text.lines();
        if lines.next()? != "LEASE/1" {
            return None;
        }
        let holder = WriterId(lines.next()?.parse().ok()?);
        let token: u64 = lines.next()?.parse().ok()?;
        let expires: u64 = lines.next()?.parse().ok()?;
        if lines.next().is_some() {
            return None;
        }
        Some(Self {
            holder,
            token,
            expires,
        })
    }
}

/// Unix epoch milliseconds, the lease clock.
#[must_use]
pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// An object version tag as the store reports it. `FsStore` renders the
/// blake3 of the object bytes as lowercase hex; HTTP stores carry the
/// vendor's `ETag` header value verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etag(pub String);

impl fmt::Display for Etag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A fetched object: its bytes and the version tag they arrived under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub bytes: Vec<u8>,
    pub etag: Etag,
}

/// Outcome of a conditional GET.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    Unchanged,
    Changed(Fetched),
}

/// Outcome of a create-only PUT. `Ambiguous` is an unproved transport
/// result (S3 409, a retried PUT); the GET-verify law resolves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Create {
    Created(Etag),
    Exists,
    Ambiguous,
}

/// Outcome of a compare-and-swap PUT. `Ambiguous` is an unproved
/// transport result; the GET-verify law resolves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Swap {
    Swapped(Etag),
    Moved,
    Ambiguous,
}

/// An infrastructure failure from the store: the transport or the
/// filesystem, never a protocol outcome. Every store failure path,
/// including a body-stream read, wraps this.
#[derive(Debug)]
pub struct StoreError {
    pub op: &'static str,
    pub key: String,
    pub source: std::io::Error,
}

/// The store-error brand the contract names.
pub type ErrStore = StoreError;

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "object store {} on `{}`: {}",
            self.op, self.key, self.source
        )
    }
}

impl std::error::Error for StoreError {}

pub type Result<T> = std::result::Result<T, StoreError>;

/// The five operations the protocol needs; nothing a vendor offers beyond
/// them appears. Consumers monomorphize over `S: ObjectStore`. Every
/// method is synchronous. An impl must not `block_on` from an async
/// context — it returns `Err` instead.
pub trait ObjectStore: Send + Sync {
    /// GET. `Ok(None)` on 404.
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>>;

    /// GET with `If-None-Match: <etag>`. `Ok(Unchanged)` on 304 — the
    /// cheap manifest poll.
    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll>;

    /// PUT with `If-None-Match: "*"`. `Ok(Created(etag))` or `Ok(Exists)`
    /// on a proved occupation; `Ok(Ambiguous)` when the transport cannot
    /// prove the result. The log-slot arbitration primitive.
    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> Result<Create>;

    /// PUT with `If-Match: <etag>`. `Ok(Swapped(etag))` or `Ok(Moved)` on
    /// a proved mismatch; `Ok(Ambiguous)` when the transport cannot
    /// prove the result. The manifest CAS primitive.
    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> Result<Swap>;

    /// DELETE (unconditional). The gc verb's tool. Success means the
    /// parent directory is durable.
    fn delete(&self, key: &StoreKey) -> Result<()>;
}

/// What a follow-up GET proved about an ambiguous `put_create`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateProbe {
    /// The key holds exactly the bytes we tried to write: our create
    /// landed, and this is its tag.
    Landed(Etag),
    /// The key holds someone else's bytes: we lost the slot; the loser
    /// algebra takes the winner's object from here.
    Lost(Fetched),
    /// The key does not exist: the ambiguous request never landed and the
    /// create may be reissued.
    Absent,
}

/// What a follow-up GET proved about an ambiguous `put_swap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapProbe {
    /// The key holds exactly the bytes we tried to write: our swap landed,
    /// and this is the tag it produced.
    Landed(Etag),
    /// The key holds other bytes: our swap did not take effect (or was
    /// itself swapped over); re-read and re-decide from this state.
    Lost(Fetched),
    /// The key does not exist.
    Absent,
}

/// Collapse an `Ambiguous` create through the GET-verify law.
pub fn prove_create<S: ObjectStore>(
    store: &S,
    key: &StoreKey,
    attempted: &[u8],
    outcome: Create,
) -> Result<Create> {
    match outcome {
        Create::Created(_) | Create::Exists => Ok(outcome),
        Create::Ambiguous => match resolve_ambiguous_create(store, key, attempted)? {
            CreateProbe::Landed(etag) => Ok(Create::Created(etag)),
            CreateProbe::Lost(_) => Ok(Create::Exists),
            CreateProbe::Absent => Ok(Create::Ambiguous),
        },
    }
}

/// Collapse an `Ambiguous` swap through the GET-verify law.
pub fn prove_swap<S: ObjectStore>(
    store: &S,
    key: &StoreKey,
    attempted: &[u8],
    outcome: Swap,
) -> Result<Swap> {
    match outcome {
        Swap::Swapped(_) | Swap::Moved => Ok(outcome),
        Swap::Ambiguous => match resolve_ambiguous_swap(store, key, attempted)? {
            SwapProbe::Landed(etag) => Ok(Swap::Swapped(etag)),
            SwapProbe::Lost(_) | SwapProbe::Absent => Ok(Swap::Moved),
        },
    }
}

/// The retry law for `put_create`: a conditional write is never blindly
/// retried after an ambiguous outcome (a timeout after the request may
/// have landed). The follow-up is a GET of the target key comparing
/// content — byte-equal means the operation succeeded.
pub fn resolve_ambiguous_create<S: ObjectStore>(
    store: &S,
    key: &StoreKey,
    attempted: &[u8],
) -> Result<CreateProbe> {
    match retry_read(|| store.get(key))? {
        None => Ok(CreateProbe::Absent),
        Some(fetched) if fetched.bytes == attempted => Ok(CreateProbe::Landed(fetched.etag)),
        Some(fetched) => Ok(CreateProbe::Lost(fetched)),
    }
}

/// The retry law for `put_swap`: never a blind retry — the follow-up is a
/// GET of the target key re-reading its etag. Bytes equal to the attempted
/// body prove the swap landed under that fresh tag; anything else is the
/// state to re-decide from.
pub fn resolve_ambiguous_swap<S: ObjectStore>(
    store: &S,
    key: &StoreKey,
    attempted: &[u8],
) -> Result<SwapProbe> {
    match retry_read(|| store.get(key))? {
        None => Ok(SwapProbe::Absent),
        Some(fetched) if fetched.bytes == attempted => Ok(SwapProbe::Landed(fetched.etag)),
        Some(fetched) => Ok(SwapProbe::Lost(fetched)),
    }
}

/// Read attempts, jittered exponential backoff base 50 ms cap 2 s, six
/// attempts total, then the last failure surfaces as `Err`.
pub fn retry_read<T, F: FnMut() -> Result<T>>(mut op: F) -> Result<T> {
    const ATTEMPTS: u32 = 6;
    const BASE_MS: u64 = 50;
    const CAP_MS: u64 = 2_000;
    let mut attempt: u32 = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) => {
                attempt += 1;
                if attempt == ATTEMPTS {
                    return Err(err);
                }
                let ceiling_ms = CAP_MS.min(BASE_MS << (attempt - 1));
                std::thread::sleep(jittered(Duration::from_millis(ceiling_ms)));
            }
        }
    }
}

/// Full jitter: a uniform-ish duration in `[0, ceiling]`, from a process
/// xorshift stream seeded off the clock and pid. Decorrelation across
/// retriers is the whole requirement; distribution quality is not.
pub(crate) fn jittered(ceiling: Duration) -> Duration {
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(1, |d| {
                u64::try_from(d.as_nanos() & u128::from(u64::MAX)).unwrap_or(1)
            });
        x = nanos ^ (u64::from(std::process::id()) << 32) | 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    let ceiling_nanos = u64::try_from(ceiling.as_nanos()).unwrap_or(u64::MAX);
    Duration::from_nanos(x % ceiling_nanos.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_and_control_segments_are_not_keys() {
        for key in [
            "~tmp/x",
            "~lease/manifest.json",
            "a/~tmp",
            "log/\u{0001}/1",
            "manifest.json.lock",
        ] {
            assert!(StoreKey::parse(key).is_err(), "{key}");
        }
        assert!(StoreKey::parse("log/c00000000/1").is_ok());
        assert!(segment_ok("_shared"));
        assert!(!segment_ok("~tmp"));
        assert!(!segment_ok("a/b"));
    }

    #[test]
    fn lease_round_trips_and_expiry_is_the_only_break() {
        let lease = Lease {
            holder: WriterId(7),
            token: 3,
            expires: 100,
        };
        let parsed = Lease::parse(&lease.encode()).expect("parse");
        assert_eq!(parsed, lease);
        assert!(lease.expired(100));
        assert!(!lease.expired(99));
        assert!(lease.breakable(100));
        assert!(!lease.breakable(99));
        assert!(!lease.break_on_probe(100, Liveness::Unknown));
        assert!(!lease.break_on_probe(100, Liveness::Alive));
        assert!(lease.break_on_probe(100, Liveness::Dead));
        assert!(!lease.break_on_probe(99, Liveness::Dead));
    }

    #[test]
    fn prefix_grammar_matches_the_key_grammar() {
        assert_eq!(parse_prefix("").expect("empty"), "");
        assert_eq!(parse_prefix("/smoke/run/").expect("trim"), "smoke/run");
        assert!(parse_prefix("~tmp").is_err());
        assert!(parse_prefix("a//b").is_err());
    }
}
