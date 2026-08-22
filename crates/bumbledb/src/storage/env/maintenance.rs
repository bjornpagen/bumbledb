use crate::error::Result;

use super::Environment;

impl Environment {

    /// # Errors

    pub(crate) fn disk_size(&self) -> Result<u64> {
        Ok(self.env.real_disk_size()?)
    }
}
