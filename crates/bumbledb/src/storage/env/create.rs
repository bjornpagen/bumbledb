use std::path::Path;

use crate::error::Result;
use crate::schema::Schema;

use super::Environment;

impl Environment {
    /// Initializes a fresh environment through [`Self::publish`]:
    /// staging, one catalog txn, atomic rename. Format 8, four-key
    /// `_meta`, generation 0, dict next-id 0.
    ///
    /// # Errors
    ///
    /// `Io` on directory creation, `EnvironmentLocked` if another handle
    /// holds the environment, `DestinationExists` on a path that already
    /// exists (including as an empty directory), `Lmdb` on any LMDB
    /// failure.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn create(path: &Path, schema: &Schema) -> Result<Self> {
        let created = Self::publish_empty(path, schema)?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(created)
    }
}
