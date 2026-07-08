use crate::error::Result;

use super::Environment;

impl Environment {
    /// The environment file's real on-disk size.
    ///
    /// # Errors
    ///
    /// `Io` via heed on a failed stat.
    /// Writes a compacted copy of the live pages into `file` — the
    /// freelist is dropped and the B-trees lay out sequentially (LMDB's
    /// `mdb_env_copy2(MDB_CP_COMPACT)`). The source stays open and
    /// untouched.
    pub(crate) fn copy_compacted(&self, file: &mut std::fs::File) -> Result<()> {
        Ok(self
            .env
            .copy_to_file(file, heed::CompactionOption::Enabled)?)
    }

    pub(crate) fn disk_size(&self) -> Result<u64> {
        Ok(self.env.real_disk_size()?)
    }
}
