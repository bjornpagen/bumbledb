use std::path::Path;

use heed::WithoutTls;

use crate::error::{Error, Result};
use crate::schema::Schema;

use super::read_meta::{MetaBlock, classify_meta_block, write_fresh_meta};
use super::{Environment, GenerationId, StoreKind};

impl Environment {
    /// Initializes a fresh DURABLE environment through [`Self::publish`]:
    /// staging, one catalog txn, atomic rename. Format 8, six-key `_meta`,
    /// generation 0, dict next-id 0.
    ///
    /// # Errors
    ///
    /// `Io` on directory creation, `EnvironmentLocked` if another handle
    /// holds the environment, `DestinationExists` on a path that already
    /// exists (including as an empty directory), `Lmdb` on any LMDB
    /// failure.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn create(path: &Path, schema: &Schema) -> Result<Self> {
        let created = Self::publish_empty(path, StoreKind::Durable, schema)?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(created)
    }

    /// In-place empty initialization for ephemeral crash recovery: the
    /// destination already exists (wiped). Fresh-path constructors use
    /// [`Self::publish`] instead.
    pub(super) fn initialize(
        env: heed::Env<WithoutTls>,
        lock: std::fs::File,
        schema: &Schema,
        kind: StoreKind,
        dirty_marker: Option<std::path::PathBuf>,
    ) -> Result<Self> {
        let mut wtxn = env.write_txn()?;
        match classify_meta_block(&env, &wtxn)? {
            MetaBlock::Present(_) => return Err(Error::AlreadyInitialized),
            MetaBlock::HalfCreated => {}
        }
        let meta = env.create_database(&mut wtxn, Some("_meta"))?;
        let data = env.create_database(&mut wtxn, Some("_data"))?;
        let dict = env.create_database(&mut wtxn, Some("_dict"))?;
        write_fresh_meta(
            &meta,
            &mut wtxn,
            schema,
            kind,
            GenerationId::initial(),
            crate::encoding::InternId::from_raw(0),
        )?;
        wtxn.commit()?;
        Ok(Self::assemble(
            env,
            meta,
            data,
            dict,
            Self::mode_for(kind, lock, dirty_marker),
        ))
    }
}
