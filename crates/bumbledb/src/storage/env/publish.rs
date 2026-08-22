//! One store-birth protocol: [`PublishStep`] folded over a catalog source.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use heed::types::Bytes;
use heed::{Database, WithoutTls};

use crate::error::{Error, IoFailure, Result};
use crate::schema::Schema;
use crate::storage::catalog::{
    Bounds, CatalogMap, CatalogRead, FrozenCatalog, LmdbReadCatalog, OrderedRead, ReadCursor,
};

use super::acquire_lock::acquire_lock;
use super::open_env::{OpenLane, open_env};
use super::read_meta::write_fresh_meta;
use super::{Environment, GenerationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublishStep {
    CreateStaging,
    /// One LMDB write txn: `_data`, `_dict`, fresh `_meta`.
    WriteCatalog,

    CommitAndClose,

    SyncStagingFiles,

    Rename,

    SyncParent,
}

impl PublishStep {
    pub const ALL: [Self; 6] = [
        Self::CreateStaging,
        Self::WriteCatalog,
        Self::CommitAndClose,
        Self::SyncStagingFiles,
        Self::Rename,
        Self::SyncParent,
    ];

    #[must_use]
    pub const fn before_rename(self) -> bool {
        matches!(
            self,
            Self::CreateStaging
                | Self::WriteCatalog
                | Self::CommitAndClose
                | Self::SyncStagingFiles
        )
    }
}

pub(crate) struct PublishCatalog<'a> {
    schema: &'a Schema,
    generation: GenerationId,
    inner: PublishInner<'a>,
}

enum PublishInner<'a> {
    Frozen(&'a FrozenCatalog),
    Store(&'a Environment),
}

impl<'a> PublishCatalog<'a> {
    pub(crate) fn frozen(catalog: &'a FrozenCatalog, schema: &'a Schema) -> Self {
        Self {
            schema,
            generation: GenerationId::initial(),
            inner: PublishInner::Frozen(catalog),
        }
    }

    pub(crate) fn store(env: &'a Environment, schema: &'a Schema) -> Result<Self> {
        Ok(Self {
            schema,
            generation: env.read_txn()?.generation()?,
            inner: PublishInner::Store(env),
        })
    }
}

impl Environment {
    /// after rename is [`Error::PublishedButUnsynced`].
    pub(crate) fn publish(path: &Path, catalog: &PublishCatalog<'_>) -> Result<Self> {
        match publish_inner(path, catalog, OpenLane::Write, None, false)? {
            PublishOutcome::Done(env) => Ok(env),
            PublishOutcome::Prefix { .. } => {
                unreachable!("full publish has no stop_after")
            }
        }
    }

    pub(crate) fn publish_empty(path: &Path, schema: &Schema) -> Result<Self> {
        let catalog = FrozenCatalog::empty();
        Self::publish(path, &PublishCatalog::frozen(&catalog, schema))
    }

    #[doc(hidden)]
    pub(crate) fn publish_nosync(path: &Path, catalog: &PublishCatalog<'_>) -> Result<Self> {
        match publish_inner(path, catalog, OpenLane::Nosync, None, false)? {
            PublishOutcome::Done(env) => Ok(env),
            PublishOutcome::Prefix { .. } => {
                unreachable!("full publish has no stop_after")
            }
        }
    }
}

enum PublishOutcome {
    Done(Environment),
    #[cfg_attr(not(test), allow(dead_code))]
    Prefix {
        dest_exists: bool,
        staging: PathBuf,
    },
}

fn publish_inner(
    dest: &Path,
    catalog: &PublishCatalog<'_>,
    lane: OpenLane,
    stop_after: Option<PublishStep>,
    fail_parent_sync: bool,
) -> Result<PublishOutcome> {
    super::refuse_existing_destination(dest)?;
    ensure_parent(dest)?;

    let mut staging = None;
    let mut lock = None;
    let mut raw = None;

    for step in PublishStep::ALL {
        match step {
            PublishStep::CreateStaging => {
                staging = Some(create_staging(dest)?);
            }
            PublishStep::WriteCatalog => {
                let dir = staging
                    .as_ref()
                    .expect("CreateStaging precedes WriteCatalog");
                lock = Some(acquire_lock(dir)?);
                let env = open_env(dir, lane)?;
                raw = Some(write_catalog(env, catalog)?);
            }
            PublishStep::CommitAndClose => {
                drop(raw.take());
            }
            PublishStep::SyncStagingFiles => {
                let _s = crate::obs::span(crate::obs::names::PUBLISH_SYNC);
                sync_staging_files(staging.as_ref().expect("staging lives until Rename"))?;
            }
            PublishStep::Rename => {
                std::fs::rename(staging.as_ref().expect("staging lives until Rename"), dest)?;
            }
            PublishStep::SyncParent => {
                // destination dirent-chain fsync after the rename.
                let _s = crate::obs::span(crate::obs::names::PUBLISH_SYNC);
                if fail_parent_sync {
                    return Err(Error::PublishedButUnsynced {
                        path: dest.to_path_buf(),
                        source: IoFailure {
                            kind: std::io::ErrorKind::Other,
                            raw_os: None,
                        },
                    });
                }
                if let Err(error) = super::sync_dirent_chain(dest) {
                    return Err(Error::PublishedButUnsynced {
                        path: dest.to_path_buf(),
                        source: IoFailure::from_io(&error),
                    });
                }
            }
        }
        if stop_after == Some(step) {
            drop(raw.take());
            drop(lock.take());
            return Ok(prefix(
                !step.before_rename(),
                if step.before_rename() {
                    staging.take().expect("staging")
                } else {
                    dest.to_path_buf()
                },
            ));
        }
    }

    let env = attach(
        dest,
        lane,
        lock.take().expect("WriteCatalog acquired the lock"),
        catalog.schema,
    )?;
    Ok(PublishOutcome::Done(env))
}

fn prefix(dest_exists: bool, staging: PathBuf) -> PublishOutcome {
    PublishOutcome::Prefix {
        dest_exists,
        staging,
    }
}

fn ensure_parent(dest: &Path) -> Result<()> {
    match dest.parent() {
        Some(parent) if !parent.as_os_str().is_empty() && !parent.exists() => {
            std::fs::create_dir_all(parent)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn create_staging(dest: &Path) -> Result<PathBuf> {
    for _ in 0..16 {
        let staging = staging_path(dest, next_nonce());
        match std::fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                return Err(Error::from(error));
            }
            Err(_) => {}
        }
    }
    Err(Error::from(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "exhausted staging nonces",
    )))
}

fn staging_path(dest: &Path, nonce: u64) -> PathBuf {
    let name = dest.file_name().unwrap_or(dest.as_os_str());
    dest.with_file_name(format!("{}.staging.{nonce:016x}", name.to_string_lossy()))
}

fn next_nonce() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    u64::from(std::process::id())
        ^ NEXT
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn write_catalog(
    env: heed::Env<WithoutTls>,
    catalog: &PublishCatalog<'_>,
) -> Result<heed::Env<WithoutTls>> {
    match catalog.inner {
        PublishInner::Frozen(frozen) => {
            write_from_catalog(&env, catalog.schema, catalog.generation, frozen)?;
        }
        PublishInner::Store(source) => {
            let txn = source.read_txn()?;
            let live = LmdbReadCatalog::new(&txn);
            write_from_catalog(&env, catalog.schema, catalog.generation, &live)?;
        }
    }
    Ok(env)
}

fn write_from_catalog<C: CatalogRead>(
    env: &heed::Env<WithoutTls>,
    schema: &Schema,
    generation: GenerationId,
    catalog: &C,
) -> Result<()> {
    let mut wtxn = env.write_txn()?;
    let meta = env.create_database(&mut wtxn, Some("_meta"))?;
    let data = env.create_database(&mut wtxn, Some("_data"))?;
    let dict = env.create_database(&mut wtxn, Some("_dict"))?;

    let mut copy_span = crate::obs::span(crate::obs::names::PUBLISH_COPY);
    let data_bytes = copy_map(catalog, data, &mut wtxn, CatalogMap::Data)?;
    let dict_bytes = copy_map(catalog, dict, &mut wtxn, CatalogMap::Dictionary)?;
    copy_span.set_count(data_bytes + dict_bytes);
    copy_span.end();
    write_fresh_meta(
        &meta,
        &mut wtxn,
        schema,
        generation,
        catalog.dict_next_id()?,
    )?;
    wtxn.commit()?;
    Ok(())
}

fn copy_map<C: OrderedRead>(
    catalog: &C,
    dest: Database<Bytes, Bytes>,
    wtxn: &mut heed::RwTxn<'_>,
    map: CatalogMap,
) -> Result<u64> {
    let mut bytes = 0u64;
    let mut range = catalog.range(map, Bounds::all())?;
    while let Some(entry) = range.next()? {
        bytes += (entry.key.len() + entry.value.len()) as u64;
        dest.put(wtxn, entry.key, entry.value)?;
    }
    Ok(bytes)
}

fn sync_staging_files(staging: &Path) -> Result<()> {
    for entry in std::fs::read_dir(staging)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            File::open(entry.path())?.sync_all()?;
        }
    }
    Ok(super::sync_dirent_chain(staging)?)
}

fn attach(path: &Path, lane: OpenLane, lock: File, schema: &Schema) -> Result<Environment> {
    let raw = open_env(path, lane)?;
    Environment::verify_and_open(raw, lock, schema)
}

#[cfg(test)]
pub(super) struct PublishPrefix {
    pub dest_exists: bool,
    pub staging: PathBuf,
    dest: PathBuf,
}

#[cfg(test)]
impl Drop for PublishPrefix {
    fn drop(&mut self) {
        if !self.dest_exists {
            let _ = std::fs::remove_dir_all(&self.staging);
        }
        let _ = std::fs::remove_dir_all(&self.dest);
        remove_staging_siblings(&self.dest);
    }
}

#[cfg(test)]
fn remove_staging_siblings(dest: &Path) {
    let Some(parent) = dest.parent() else {
        return;
    };
    let Some(name) = dest.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let prefix = format!("{name}.staging.");
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(&prefix))
        {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(test)]
impl Environment {
    pub(super) fn publish_until(
        path: &Path,
        catalog: &PublishCatalog<'_>,
        last: PublishStep,
    ) -> Result<PublishPrefix> {
        match publish_inner(path, catalog, OpenLane::Write, Some(last), false)? {
            PublishOutcome::Prefix {
                dest_exists,
                staging,
            } => Ok(PublishPrefix {
                dest_exists,
                staging,
                dest: path.to_path_buf(),
            }),
            PublishOutcome::Done(_) => unreachable!("stop_after never returns a handle"),
        }
    }

    pub(super) fn publish_failing_parent_sync(
        path: &Path,
        catalog: &PublishCatalog<'_>,
    ) -> Result<Self> {
        match publish_inner(path, catalog, OpenLane::Write, None, true)? {
            PublishOutcome::Done(env) => Ok(env),
            PublishOutcome::Prefix { .. } => unreachable!("full publish"),
        }
    }
}

#[cfg(test)]
mod tests;
