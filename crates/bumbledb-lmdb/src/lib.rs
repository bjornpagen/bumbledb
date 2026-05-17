//! Internal LMDB storage boundary for Bumbledb.
//!
//! This crate intentionally keeps all LMDB details behind opaque environment and
//! transaction types. Higher layers should not depend on raw LMDB handles.

use std::fs;
use std::path::{Path, PathBuf};

use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

/// Current on-disk storage format version.
pub const STORAGE_FORMAT_VERSION: u32 = 1;

const DEFAULT_MAP_SIZE: usize = 64 * 1024 * 1024 * 1024;
const DEFAULT_MAX_READERS: u32 = 1024;
const FIXED_DATABASE_COUNT: u32 = 3;

const META_DB: &str = "_meta";
const INDEX_DB: &str = "_index";
const DICT_DB: &str = "_dict";

const DATA_FILE: &str = "data.mdb";
const STORAGE_FORMAT_VERSION_KEY: &[u8] = b"storage_format_version";

type RawDatabase = Database<Bytes, Bytes>;

/// Result type for storage operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Storage-layer errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// LMDB or heed failure.
    #[error(transparent)]
    Heed(#[from] heed::Error),

    /// Filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Existing database has an incompatible storage format version.
    #[error("storage format version mismatch: expected {expected}, found {found}")]
    StorageFormatMismatch { expected: u32, found: u32 },

    /// Existing database is missing required storage metadata.
    #[error("storage format version metadata is missing")]
    MissingStorageFormatVersion,

    /// Storage metadata is malformed.
    #[error("storage metadata is corrupt: {0}")]
    CorruptMetadata(&'static str),

    /// Internal storage invariant failure.
    #[error("internal storage error: {0}")]
    Internal(String),
}

/// Fixed LMDB databases opened by every environment.
#[derive(Clone, Copy)]
struct Databases {
    meta: RawDatabase,
    #[allow(dead_code)]
    index: RawDatabase,
    #[allow(dead_code)]
    dict: RawDatabase,
}

/// Embedded LMDB environment wrapper.
pub struct Environment {
    env: Env<WithoutTls>,
    dbs: Databases,
    #[allow(dead_code)]
    path: PathBuf,
}

impl Environment {
    /// Opens or creates an LMDB environment at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let had_data_file = path.join(DATA_FILE).exists();
        fs::create_dir_all(path)?;

        let mut options = EnvOpenOptions::new().read_txn_without_tls();
        options
            .map_size(DEFAULT_MAP_SIZE)
            .max_dbs(FIXED_DATABASE_COUNT)
            .max_readers(DEFAULT_MAX_READERS);

        let env = unsafe { options.open(path)? };
        let dbs = Self::open_fixed_databases(&env, had_data_file)?;

        Ok(Self {
            env,
            dbs,
            path: path.to_path_buf(),
        })
    }

    /// Returns the configured maximum LMDB key size.
    pub fn max_key_size(&self) -> usize {
        self.env.max_key_size()
    }

    /// Returns the maximum reader slots configured for the environment.
    pub fn max_readers(&self) -> u32 {
        self.env.max_readers()
    }

    /// Clears stale LMDB reader slots.
    pub fn clear_stale_readers(&self) -> Result<usize> {
        Ok(self.env.clear_stale_readers()?)
    }

    /// Reads the storage format version from metadata.
    pub fn storage_format_version(&self) -> Result<u32> {
        self.read(|txn| txn.storage_format_version())
    }

    /// Runs a closure inside a read-only transaction.
    pub fn read<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&ReadTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let txn = self.env.read_txn().map_err(Error::from).map_err(E::from)?;
        let read = ReadTxn { txn, dbs: self.dbs };
        f(&read)
    }

    /// Runs a closure inside a read-write transaction.
    pub fn write<T, E>(
        &self,
        f: impl for<'txn> FnOnce(&mut WriteTxn<'txn>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        let txn = self.env.write_txn().map_err(Error::from).map_err(E::from)?;
        let mut write = WriteTxn { txn, dbs: self.dbs };

        match f(&mut write) {
            Ok(value) => {
                let WriteTxn { txn, .. } = write;
                txn.commit().map_err(Error::from).map_err(E::from)?;
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    fn open_fixed_databases(env: &Env<WithoutTls>, had_data_file: bool) -> Result<Databases> {
        let mut txn = env.write_txn()?;
        let dbs = Databases {
            meta: env.create_database(&mut txn, Some(META_DB))?,
            index: env.create_database(&mut txn, Some(INDEX_DB))?,
            dict: env.create_database(&mut txn, Some(DICT_DB))?,
        };

        match read_u32(&dbs.meta, &txn, STORAGE_FORMAT_VERSION_KEY)? {
            Some(STORAGE_FORMAT_VERSION) => {}
            Some(found) => {
                return Err(Error::StorageFormatMismatch {
                    expected: STORAGE_FORMAT_VERSION,
                    found,
                });
            }
            None if had_data_file => return Err(Error::MissingStorageFormatVersion),
            None => write_u32(
                &dbs.meta,
                &mut txn,
                STORAGE_FORMAT_VERSION_KEY,
                STORAGE_FORMAT_VERSION,
            )?,
        }

        txn.commit()?;
        Ok(dbs)
    }
}

/// Opaque read transaction wrapper.
pub struct ReadTxn<'env> {
    txn: RoTxn<'env, WithoutTls>,
    dbs: Databases,
}

impl ReadTxn<'_> {
    /// Reads the storage format version visible to this snapshot.
    pub fn storage_format_version(&self) -> Result<u32> {
        read_u32(&self.dbs.meta, &self.txn, STORAGE_FORMAT_VERSION_KEY)?
            .ok_or(Error::MissingStorageFormatVersion)
    }

    #[cfg(test)]
    fn get_meta_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.dbs.meta.get(&self.txn, key)?.map(ToOwned::to_owned))
    }
}

/// Opaque write transaction wrapper.
pub struct WriteTxn<'env> {
    txn: RwTxn<'env>,
    #[allow(dead_code)]
    dbs: Databases,
}

impl WriteTxn<'_> {
    #[cfg(test)]
    fn put_meta_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        Ok(self.dbs.meta.put(&mut self.txn, key, value)?)
    }
}

fn read_u32(db: &RawDatabase, txn: &RoTxn, key: &[u8]) -> Result<Option<u32>> {
    let Some(bytes) = db.get(txn, key)? else {
        return Ok(None);
    };

    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| Error::CorruptMetadata("u32 metadata must be four bytes"))?;
    Ok(Some(u32::from_be_bytes(bytes)))
}

fn write_u32(db: &RawDatabase, txn: &mut RwTxn, key: &[u8], value: u32) -> Result<()> {
    let bytes = value.to_be_bytes();
    Ok(db.put(txn, key, &bytes[..])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKER_KEY: &[u8] = b"test_marker";

    #[test]
    fn opens_initializes_and_reopens_metadata() {
        let dir = tempfile::tempdir().unwrap();

        let env = Environment::open(dir.path()).unwrap();
        assert_eq!(
            env.storage_format_version().unwrap(),
            STORAGE_FORMAT_VERSION
        );
        assert_eq!(env.max_readers(), DEFAULT_MAX_READERS);
        assert!(env.max_key_size() > 0);
        drop(env);

        let env = Environment::open(dir.path()).unwrap();
        assert_eq!(
            env.storage_format_version().unwrap(),
            STORAGE_FORMAT_VERSION
        );
    }

    #[test]
    fn write_commits_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();

        env.write(|txn| {
            txn.put_meta_bytes(MARKER_KEY, b"committed")?;
            Ok::<(), Error>(())
        })
        .unwrap();

        let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY)).unwrap();
        assert_eq!(marker.as_deref(), Some(&b"committed"[..]));
    }

    #[test]
    fn write_aborts_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();

        let result: Result<()> = env.write(|txn| {
            txn.put_meta_bytes(MARKER_KEY, b"aborted")?;
            Err(Error::Internal("intentional abort".to_owned()))
        });

        assert!(result.is_err());
        let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY)).unwrap();
        assert_eq!(marker, None);
    }

    #[test]
    fn read_snapshot_is_stable_across_later_commit() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();

        env.write(|txn| {
            txn.put_meta_bytes(MARKER_KEY, b"before")?;
            Ok::<(), Error>(())
        })
        .unwrap();

        env.read(|read| {
            assert_eq!(
                read.get_meta_bytes(MARKER_KEY)?.as_deref(),
                Some(&b"before"[..])
            );

            env.write(|write| {
                write.put_meta_bytes(MARKER_KEY, b"after")?;
                Ok::<(), Error>(())
            })?;

            assert_eq!(
                read.get_meta_bytes(MARKER_KEY)?.as_deref(),
                Some(&b"before"[..])
            );
            Ok::<(), Error>(())
        })
        .unwrap();

        let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY)).unwrap();
        assert_eq!(marker.as_deref(), Some(&b"after"[..]));
    }
}
