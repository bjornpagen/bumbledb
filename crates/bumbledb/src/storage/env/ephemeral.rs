use std::path::Path;

use heed::types::Bytes;

use crate::error::{Admission, CorruptionError, Error, Mismatch, Result};
use crate::schema::Schema;

use super::acquire_lock::acquire_lock;
use super::open_env::{OpenLane, open_env};
use super::read_meta::{
    MetaBlock, classify_meta_block, parse_meta, parse_meta_head, read_store_kind,
};
use super::{Environment, StoreKind};

/// Filesystem classification for the one ephemeral verb.
#[derive(Debug)]
pub(crate) enum EphemeralClass {
    /// Path does not exist — publish a fresh store.
    Fresh,
    /// Verified ephemeral format-8 store. Reopen; do not re-admit empty.
    ExistingVerified,
    /// Dirty marker or a half-created root — wipe when the marker says so,
    /// then in-place initialize.
    CrashVictim,
}

impl Environment {
    /// Opens or initializes an EPHEMERAL environment at `path`.
    ///
    /// A missing path publishes a fresh empty catalog. An existing
    /// ephemeral store opens with the same version/kind/roster/
    /// fingerprint/descriptor checks as [`Environment::open`]. A durable
    /// store refuses typed (`StoreKindMismatch`).
    ///
    /// # Errors
    ///
    /// `Io` on directory creation, `EnvironmentLocked` if another handle
    /// holds the environment, `AlreadyInitialized` on a directory
    /// holding a foreign LMDB environment, `FormatMismatch`/
    /// `StoreKindMismatch`/`SchemaMismatch` on an existing store that
    /// fails verification, `Corruption` on a missing or undecodable
    /// meta key, `Lmdb` otherwise.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn ephemeral(path: &Path, schema: &Schema) -> Result<Self> {
        match Self::ephemeral_gated(path, schema, |_| Ok(Admission::Accepted(())))? {
            Admission::Accepted(env) => Ok(env),
            Admission::Rejected(_) => {
                unreachable!("Environment::ephemeral gate always accepts")
            }
        }
    }

    /// [`Self::ephemeral`] with a gate that runs after classification
    /// and before any mutation. [`crate::Db::ephemeral`] complete-admits
    /// empty only on [`EphemeralClass::Fresh`] and
    /// [`EphemeralClass::CrashVictim`]. A rejected gate returns
    /// [`Admission::Rejected`] and mutates nothing.
    pub(crate) fn ephemeral_gated(
        path: &Path,
        schema: &Schema,
        before_mutate: impl FnOnce(&EphemeralClass) -> Result<Admission<()>>,
    ) -> Result<Admission<Self>> {
        let class = classify_ephemeral(path, schema)?;
        match before_mutate(&class)? {
            Admission::Rejected(violations) => return Ok(Admission::Rejected(violations)),
            Admission::Accepted(()) => {}
        }
        match class {
            EphemeralClass::Fresh => Self::publish_empty(path, StoreKind::Ephemeral, schema),
            EphemeralClass::ExistingVerified | EphemeralClass::CrashVictim => {
                Self::ephemeral_existing(path, schema)
            }
        }
        .map(Admission::Accepted)
    }

    fn ephemeral_existing(path: &Path, schema: &Schema) -> Result<Self> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }
        let lock = acquire_lock(path)?;
        let marker = super::dirty_marker_path(path);
        let crashed = marker.try_exists()?;
        if crashed {
            let has_data = path.join("data.mdb").try_exists()?;
            if has_data && Self::marker_shields_durable(path) {
                return Err(Error::StoreKindMismatch {
                    mismatch: Mismatch {
                        witnessed: StoreKind::Durable,
                        required: StoreKind::Ephemeral,
                    },
                });
            }
            crate::obs::event(
                crate::obs::names::EPHEMERAL_WIPE,
                crate::obs::TraceArgs::Flag(has_data),
            );
            for file in ["data.mdb", "lock.mdb"] {
                match std::fs::remove_file(path.join(file)) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(Error::from(e)),
                }
            }
        }
        let has_meta = if !crashed && path.join("data.mdb").try_exists()? {
            Self::probe_ephemeral_kind(path, schema)?
        } else {
            false
        };
        std::fs::File::create(&marker)?.sync_all()?;
        super::sync_dirent_chain(path)?;
        let opened = open_env(path, OpenLane::Write(StoreKind::Ephemeral)).and_then(|env| {
            if has_meta {
                Self::verify_and_open(
                    env,
                    lock,
                    schema,
                    StoreKind::Ephemeral,
                    Some(marker.clone()),
                )
            } else {
                Self::initialize(
                    env,
                    lock,
                    schema,
                    StoreKind::Ephemeral,
                    Some(marker.clone()),
                )
            }
        });
        match opened {
            Ok(opened) => Ok(opened),
            Err(e) => {
                if has_meta {
                    let _ = std::fs::remove_file(&marker);
                    let _ = super::sync_dirent_chain(path);
                }
                Err(e)
            }
        }
    }

    /// Whether the store under a stale dirty marker cleanly reads as
    /// DURABLE — the one outcome that shields it from the R18 wipe.
    fn marker_shields_durable(path: &Path) -> bool {
        let kind = || -> Result<StoreKind> {
            let env = open_env(path, OpenLane::ReadOnly)?;
            let rtxn = env.read_txn()?;
            let MetaBlock::Present(meta) = classify_meta_block(&env, &rtxn)? else {
                return Err(Error::AlreadyInitialized);
            };
            read_store_kind(&meta, &rtxn)
        };
        matches!(kind(), Ok(StoreKind::Durable))
    }

    /// Probe over an existing data file: version, kind, roster,
    /// fingerprint, descriptor. Returns `Ok(true)` on a verified
    /// ephemeral store.
    fn probe_ephemeral_kind(path: &Path, schema: &Schema) -> Result<bool> {
        match probe_ephemeral_target(path, schema)? {
            EphemeralTarget::Fresh => Ok(false),
            EphemeralTarget::Existing => Ok(true),
        }
    }
}

enum EphemeralTarget {
    Fresh,
    Existing,
}

fn classify_ephemeral(path: &Path, schema: &Schema) -> Result<EphemeralClass> {
    if !path.exists() {
        return Ok(EphemeralClass::Fresh);
    }
    let has_data = path.join("data.mdb").try_exists()?;
    let has_marker = super::dirty_marker_path(path).try_exists()?;
    if has_marker {
        if has_data && Environment::marker_shields_durable(path) {
            return Err(Error::StoreKindMismatch {
                mismatch: Mismatch {
                    witnessed: StoreKind::Durable,
                    required: StoreKind::Ephemeral,
                },
            });
        }
        return Ok(EphemeralClass::CrashVictim);
    }
    if !has_data {
        return Err(Error::DestinationExists {
            path: path.to_path_buf(),
        });
    }
    match probe_ephemeral_target(path, schema)? {
        EphemeralTarget::Fresh => Ok(EphemeralClass::CrashVictim),
        EphemeralTarget::Existing => Ok(EphemeralClass::ExistingVerified),
    }
}

fn probe_ephemeral_target(path: &Path, schema: &Schema) -> Result<EphemeralTarget> {
    let env = open_env(path, OpenLane::Write(StoreKind::Durable))?;
    let rtxn = env.read_txn()?;
    let MetaBlock::Present(meta) = classify_meta_block(&env, &rtxn)? else {
        return Ok(EphemeralTarget::Fresh);
    };
    let (_, kind) = parse_meta_head(&meta, &rtxn)?;
    if kind != StoreKind::Ephemeral {
        return Err(Error::StoreKindMismatch {
            mismatch: Mismatch {
                witnessed: kind,
                required: StoreKind::Ephemeral,
            },
        });
    }
    if env
        .open_database::<Bytes, Bytes>(&rtxn, Some("_data"))?
        .is_none()
        || env
            .open_database::<Bytes, Bytes>(&rtxn, Some("_dict"))?
            .is_none()
    {
        return Err(Error::Corruption(CorruptionError::MetaMissing));
    }
    let store = parse_meta(&meta, &rtxn)?;
    store.matches_schema(schema)?;
    Ok(EphemeralTarget::Existing)
}
