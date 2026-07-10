use crate::error::Result;

use super::Environment;

impl Environment {
    /// Writes a compacted copy of the live pages into `file` — the
    /// freelist is dropped and the B-trees lay out sequentially (LMDB's
    /// `mdb_env_copy2(MDB_CP_COMPACT)`). The source stays open and
    /// untouched.
    ///
    /// # Errors
    ///
    /// `Lmdb` via heed on a failed copy.
    pub(crate) fn copy_compacted(&self, file: &mut std::fs::File) -> Result<()> {
        Ok(self
            .env
            .copy_to_file(file, heed::CompactionOption::Enabled)?)
    }

    /// The environment file's real on-disk size.
    ///
    /// # Errors
    ///
    /// `Lmdb` via heed on a failed stat.
    pub(crate) fn disk_size(&self) -> Result<u64> {
        Ok(self.env.real_disk_size()?)
    }
}
