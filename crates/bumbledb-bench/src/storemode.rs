//! The timed lanes' store constructor switch. `bench --ephemeral` runs
//! the roster against [`Db::ephemeral`] stores (`MDB_WRITEMAP|MDB_NOSYNC`
//! — the in-memory characterization lane, `docs/architecture/70-api.md`
//! § environment lifecycle); the default is the durable constructor.
//! A mode over the bench's scratch stores, never a flag on the engine:
//! the store kind is on-disk identity there, so an ephemeral run loads
//! ephemeral twins — `Db::ephemeral` on the stamped durable corpus is
//! the typed `StoreKindMismatch` refusal, by design.

use std::path::Path;

use bumbledb::Db;
use bumbledb::schema::Theory;

/// Which constructor the timed lanes build their stores with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreMode {
    #[default]
    Durable,
    Ephemeral,
}

impl StoreMode {
    /// A fresh scratch store under the mode's constructor.
    ///
    /// # Errors
    ///
    /// The engine's error, stringified with the mode named.
    pub fn create<S: Theory>(self, path: &Path, schema: S) -> Result<Db<S>, String> {
        match self {
            Self::Durable => Db::create(path, schema),
            Self::Ephemeral => Db::ephemeral(path, schema),
        }
        .map_err(|e| format!("create ({}): {e:?}", self.label()))
    }

    /// The mode's name, as reports print it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Ephemeral => "ephemeral",
        }
    }
}
