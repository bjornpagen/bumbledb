use crate::error::Result;

use super::Environment;

impl Environment {
    /// The environment file's real on-disk size.
    ///
    /// # Errors
    ///
    /// `Lmdb` via heed on a failed stat.
    pub(crate) fn disk_size(&self) -> Result<u64> {
        Ok(self.env.real_disk_size()?)
    }
}
