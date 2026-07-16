//! The one raw-LMDB-open chokepoint. Unsafe policy (the 40-storage doc
//! amendment): this module holds the sanctioned `unsafe` of the storage
//! layer — `heed 0.22` marks environment opening unsafe (double-opening
//! one path in a process is LMDB UB) and marks env-flag setting unsafe
//! (the flags can break durability or aliasing guarantees). Both are
//! confined here; the flags are DERIVED from the store kind — no caller
//! can pass a flag, so the durable paths structurally cannot reach
//! `WRITE_MAP`/`NO_SYNC`.

use std::path::Path;

use heed::{EnvFlags, EnvOpenOptions, WithoutTls};

use crate::error::Result;

use super::{MAP_SIZE, MAX_READERS, StoreKind};

/// Opens the raw LMDB environment at `path`, with the environment flags
/// the store kind dictates and nothing else.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operations have documented safety invariants"
)]
pub(super) fn open_env(path: &Path, kind: StoreKind) -> Result<heed::Env<WithoutTls>> {
    // MDB_NOTLS: reader slots belong to transaction objects, not threads —
    // a thread may pin an old snapshot while opening new ones (long-lived
    // readers across commits are a designed-for pattern, 40-storage).
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options
        .map_size(MAP_SIZE)
        .max_dbs(3)
        .max_readers(MAX_READERS);
    if kind == StoreKind::Ephemeral {
        // SAFETY: WRITE_MAP|NO_SYNC trade machine-crash durability away,
        // which is the ephemeral store kind's on-disk claim
        // (docs/architecture/50-storage.md § the ephemeral store kind);
        // process-kill atomicity is preserved (the crashpoint sweep runs
        // against ephemeral stores, fuzz/tests/crash.rs). WRITE_MAP's
        // writable mapping is confined to the single-writer engine: no
        // engine surface hands out `&mut` into the map, and readers see
        // LMDB CoW pages exactly as on a durable store.
        unsafe { options.flags(EnvFlags::WRITE_MAP | EnvFlags::NO_SYNC) };
    }
    // SAFETY: bumbledb opens each environment through exactly this function,
    // and heed itself refuses (Error::EnvAlreadyOpened) to open a path that
    // is already open in this process, upholding LMDB's single-open rule.
    let env = unsafe { options.open(path)? };
    if kind == StoreKind::Ephemeral {
        preallocate(&path.join("data.mdb"))?;
    }
    Ok(env)
}

/// Enforces the ephemeral kind's capacity contract on SPARSE filesystems
/// (`docs/architecture/50-storage.md` § the ephemeral store kind: the
/// volume must hold map size + slack or open refuses typed). Under
/// `WRITEMAP` the open ftruncates `data.mdb` to the full map, which
/// allocates the blocks on a non-sparse filesystem (HFS+ — an undersized
/// volume refuses inside LMDB's own open) but allocates NOTHING on a
/// sparse one (APFS): the store would then report `Ok` for commits past
/// the volume's physical capacity with no write path left to surface
/// `ENOSPC` — `NOSYNC` never writes at commit, and the dirty pages the
/// kernel cannot write back are silently unbackable state a clean
/// process handoff may still lose. Allocating the map's blocks here
/// makes the refusal uniform across filesystems: capacity is judged
/// ONCE, at open, as the same `Lmdb(Io(StorageFull))` shape the
/// non-sparse path produces — never a silent overcommit.
fn preallocate(data: &std::path::Path) -> Result<()> {
    let full_map = || {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(data)?;
        preallocate_blocks(&file, super::MAP_SIZE as u64)
    };
    // The same typed shape LMDB's own open produces when a non-sparse
    // filesystem refuses the ftruncate: a StorageFull-carrying Lmdb error.
    full_map().map_err(|err| crate::error::Error::Lmdb(heed::Error::Io(err)))
}

/// Allocates the file's blocks up to `len` bytes — `fcntl(F_PREALLOCATE)`,
/// macOS's only block-reservation call (`posix_fallocate` does not exist
/// here). `F_PEOFPOSMODE` allocates from the physical end of file, so the
/// request is the map size minus what the file already holds — a reopen
/// of a fully allocated store requests nothing.
#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operations have documented safety invariants"
)]
fn preallocate_blocks(file: &std::fs::File, len: u64) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::MetadataExt;
    let allocated = file.metadata()?.blocks().saturating_mul(512);
    if allocated >= len {
        return Ok(());
    }
    let mut store = libc::fstore_t {
        fst_flags: libc::F_ALLOCATEALL,
        fst_posmode: libc::F_PEOFPOSMODE,
        fst_offset: 0,
        fst_length: i64::try_from(len - allocated).expect("the map size fits i64"),
        fst_bytesalloc: 0,
    };
    // SAFETY: `fcntl(F_PREALLOCATE)` reads the initialized `fstore_t`
    // through a valid pointer and writes only `fst_bytesalloc`; the fd
    // stays owned by `file` for the whole call.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_PREALLOCATE, &raw mut store) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Allocates the file's blocks over `[0, len)` — `posix_fallocate`, which
/// is idempotent over already-allocated ranges and returns its error
/// directly instead of through `errno`.
#[cfg(all(unix, not(target_os = "macos")))]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operations have documented safety invariants"
)]
fn preallocate_blocks(file: &std::fs::File, len: u64) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the fd stays owned by `file` for the whole call; the offset
    // and length are in range for the just-truncated map file.
    let ret = unsafe {
        libc::posix_fallocate(
            file.as_raw_fd(),
            0,
            libc::off_t::try_from(len).expect("the map size fits off_t"),
        )
    };
    if ret != 0 {
        return Err(std::io::Error::from_raw_os_error(ret));
    }
    Ok(())
}

/// No block-allocation call on this platform: a non-sparse filesystem
/// already enforced capacity at LMDB's ftruncate, and a sparse one keeps
/// the filesystem's own (lazy) refusal.
#[cfg(not(unix))]
fn preallocate_blocks(_file: &std::fs::File, _len: u64) -> std::io::Result<()> {
    Ok(())
}
