//! The timed lanes' store constructor switch. `bench --nosync`
//! (`--ephemeral` is the same flag) runs the roster against a
//! durable-shaped store attached with the hidden NOSYNC open
//! (`Db::create_nosync` / `Db::open_nosync`). Not a store kind: the
//! stamped corpus is the same bytes either lane opens; only the
//! environment flags differ. The crate-private [`StoreMode::Nosync`]
//! arm is the NosyncLane flag issue 33 re-anchors on.

use std::path::Path;

use bumbledb::schema::Theory;
use bumbledb::{Admission, Db};

/// Which constructor the timed lanes build their stores with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreMode {
    #[default]
    Durable,
    /// Hidden NOSYNC attach over a durable-shaped store — the
    /// characterization lane, never a product kind.
    Nosync,
}

impl StoreMode {
    /// A fresh scratch store under the mode's constructor.
    ///
    /// # Errors
    ///
    /// The engine's error, stringified with the mode named.
    pub fn create<S: Theory>(self, path: &Path, schema: S) -> Result<Db<S>, String> {
        match match self {
            Self::Durable => Db::create(path, schema),
            Self::Nosync => Db::create_nosync(path, schema),
        } {
            Err(error) => Err(format!("create ({}): {error:?}", self.label())),
            Ok(Admission::Accepted(db)) => Ok(db),
            Ok(Admission::Rejected(violations)) => Err(format!(
                "create ({}): empty rejected: {violations}",
                self.label()
            )),
        }
    }

    /// Re-open a published store under the mode's attach flags.
    ///
    /// # Errors
    ///
    /// The engine's error, stringified with the mode named.
    pub fn open<S: Theory>(self, path: &Path, schema: S) -> Result<Db<S>, String> {
        match self {
            Self::Durable => Db::open(path, schema),
            Self::Nosync => Db::open_nosync(path, schema),
        }
        .map_err(|error| format!("open ({}): {error:?}", self.label()))
    }

    /// The mode's name, as reports print it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Nosync => "nosync",
        }
    }
}
