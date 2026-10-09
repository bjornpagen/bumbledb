//! Staged publication: a store is built in a private sibling directory and
//! renamed into place, so a crash leaves no destination or a complete one.
//! An unpublished staging directory is removed on drop.

use std::path::{Path, PathBuf};

use super::error::{StoreError, StoreResult};
use super::format::DatabaseId;
use super::store_env::{DATA_FILE, Options, Store, init_directory};
use crate::schema::Schema;

pub(crate) struct Staging {
    path: PathBuf,
    dest: PathBuf,
    published: bool,
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn sync_dirent_chain(dir: &Path) -> std::io::Result<()> {
    let parent = match dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    for d in [dir, parent] {
        std::fs::File::open(d)?.sync_all()?;
    }
    Ok(())
}

impl Staging {
    /// Reserve a fresh sibling of `dest`, which must not exist.
    pub(crate) fn begin(dest: &Path) -> StoreResult<Self> {
        if dest.exists() {
            return Err(StoreError::DestinationExists {
                path: dest.to_path_buf(),
            });
        }
        if let Some(parent) = dest.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let name = dest.file_name().unwrap_or(dest.as_os_str());
        let mut nonce = DatabaseId::mint().0;
        for _ in 0..16 {
            let suffix = u64::from_be_bytes(nonce[..8].try_into().expect("8 bytes"));
            let path =
                dest.with_file_name(format!("{}.staging.{suffix:016x}", name.to_string_lossy()));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        dest: dest.to_path_buf(),
                        published: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    nonce = DatabaseId::mint().0;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists).into())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The staged data file a downloaded image is moved into.
    pub(crate) fn data_path(&self) -> PathBuf {
        self.path.join(DATA_FILE)
    }

    /// Sync the staged files, rename into the destination, sync the parent,
    /// and open the published store.
    pub(crate) fn publish(mut self, schema: &Schema, options: Options) -> StoreResult<Store> {
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                std::fs::File::open(entry.path())?.sync_all()?;
            }
        }
        sync_dirent_chain(&self.path)?;
        if self.dest.exists() {
            return Err(StoreError::DestinationExists {
                path: self.dest.clone(),
            });
        }
        std::fs::rename(&self.path, &self.dest)?;
        self.published = true;
        sync_dirent_chain(&self.dest)?;
        Store::open(&self.dest, schema, options)
    }
}

impl Store {
    /// Create a new store at `dest` and populate it unjudged before it is
    /// published; nothing is visible at `dest` until population succeeds.
    pub(crate) fn install_populated(
        dest: &Path,
        schema: &Schema,
        database: DatabaseId,
        options: Options,
        populate: impl FnOnce(&Store) -> StoreResult<()>,
    ) -> StoreResult<Self> {
        let staging = Staging::begin(dest)?;
        init_directory(staging.path(), schema, database, options)?;
        {
            let store = Store::open(staging.path(), schema, options)?;
            populate(&store)?;
        }
        staging.publish(schema, options)
    }

    /// Install the image file at `image` as a new store at `dest`: it is moved
    /// into a staging directory, opened (format and schema checked), and
    /// published.
    pub(crate) fn install_image(
        image: &Path,
        dest: &Path,
        schema: &Schema,
        options: Options,
    ) -> StoreResult<Self> {
        let staging = Staging::begin(dest)?;
        let data = staging.data_path();
        if std::fs::rename(image, &data).is_err() {
            std::fs::copy(image, &data)?;
            std::fs::remove_file(image)?;
        }
        drop(Store::open(staging.path(), schema, options)?);
        staging.publish(schema, options)
    }
}
