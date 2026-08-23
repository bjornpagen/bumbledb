//! The object-store capability: the protocol's demand, not a vendor's
//! offer. Five verbs, all outcomes sums; `Err` carries infrastructure
//! failure (network, 5xx, auth, io) and nothing else.

pub mod fs;

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

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

/// Outcome of a create-only PUT, in the `ConditionalWrite::Moved`
/// tradition: `Exists` is a proved answer, not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Create {
    Created(Etag),
    Exists,
}

/// Outcome of a compare-and-swap PUT: `Moved` is a proved answer, not an
/// error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Swap {
    Swapped(Etag),
    Moved,
}

/// An infrastructure failure from the store: the transport or the
/// filesystem, never a protocol outcome.
#[derive(Debug)]
pub struct StoreError {
    pub op: &'static str,
    pub key: String,
    pub source: std::io::Error,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "object store {} on `{}`: {}",
            self.op, self.key, self.source
        )
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// The five operations the protocol needs; nothing a vendor offers beyond
/// them appears. Consumers monomorphize over `S: ObjectStore`.
pub trait ObjectStore: Send + Sync {
    /// GET. `Ok(None)` on 404.
    fn get(&self, key: &str) -> Result<Option<Fetched>>;

    /// GET with `If-None-Match: <etag>`. `Ok(Unchanged)` on 304 — the
    /// cheap manifest poll.
    fn get_if_changed(&self, key: &str, etag: &Etag) -> Result<Poll>;

    /// PUT with `If-None-Match: "*"`. `Ok(Created(etag))` or `Ok(Exists)`
    /// on 412. The log-slot arbitration primitive.
    fn put_create(&self, key: &str, bytes: &[u8]) -> Result<Create>;

    /// PUT with `If-Match: <etag>`. `Ok(Swapped(etag))` or `Ok(Moved)` on
    /// 412. The manifest CAS primitive.
    fn put_swap(&self, key: &str, bytes: &[u8], etag: &Etag) -> Result<Swap>;

    /// DELETE (unconditional). The gc verb's tool.
    fn delete(&self, key: &str) -> Result<()>;
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

/// The retry law for `put_create`: a conditional write is never blindly
/// retried after an ambiguous outcome (a timeout after the request may
/// have landed). The follow-up is a GET of the target key comparing
/// content — byte-equal means the operation succeeded.
pub fn resolve_ambiguous_create<S: ObjectStore>(
    store: &S,
    key: &str,
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
    key: &str,
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
fn jittered(ceiling: Duration) -> Duration {
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
