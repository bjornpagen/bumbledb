use std::path::Path;

use heed::Database;
use heed::types::Bytes;

use crate::error::{CorruptionError, Error, Result};

use super::open_env::{OpenLane, open_env};
use super::read_meta::{SelfDescription, parse_meta, parse_meta_head};
use super::{EnvMode, Environment, StoreKind};

/// What [`Environment::exhume`] hands the API layer: the opened
/// environment plus the hash-verified self-description [`parse_meta`]
/// mints.
pub(crate) struct ExhumedEnvironment {
    pub(crate) env: Environment,
    pub(crate) kind: StoreKind,
    pub(crate) description: SelfDescription,
}

impl Environment {
    /// Opens an existing environment FROM ITS OWN DESCRIPTION — no
    /// caller-supplied theory anywhere (`docs/architecture/70-api.md`
    /// § exhume). Format 8 only.
    ///
    /// # Errors
    ///
    /// `Io` on a nonexistent path, `FormatMismatch` on any other
    /// version (format 8 only; no format-7 decode arm),
    /// `Corruption(MetaMissing)`/`Corruption(StoreKindInvalid)`
    /// on a store missing its databases or meta keys (descriptor
    /// included), `Lmdb` otherwise.
    pub(crate) fn exhume(path: &Path) -> Result<ExhumedEnvironment> {
        let env = open_env(path, OpenLane::ReadOnly)?;
        let rtxn = env.read_txn()?;
        let meta: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_meta"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let (_, kind) = parse_meta_head(&meta, &rtxn)?;
        if kind == StoreKind::Ephemeral && super::dirty_marker_path(path).try_exists()? {
            return Err(Error::Corruption(CorruptionError::EphemeralDirtyArmed));
        }
        let data: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let store = parse_meta(&meta, &rtxn)?;
        store.require_preimage()?;
        rtxn.commit()?;
        Ok(ExhumedEnvironment {
            env: Self::assemble(env, meta, data, dict, EnvMode::Exhume),
            kind: store.kind,
            description: store.self_description(),
        })
    }
}
