mod access;
mod builder;
mod cache;
mod columns;
mod scope;
mod types;

pub(crate) use access::{RelationAccessComponent, RelationIndexImage};
pub(crate) use builder::QueryImageBuilder;
pub(crate) use cache::QueryImageCache;
pub use cache::QueryImageCacheDiagnostics;
#[cfg(test)]
pub(crate) use columns::{ColumnImage, EncodedColumnBuilder};
pub(crate) use scope::{QueryImageKey, QueryImageScope};
pub(crate) use types::{
    EncodedRef, FactId, FieldId, FieldImage, QueryImage, RelationId, RelationImage, RelationStats,
};

#[cfg(test)]
use crate::{EncodedOwned, Error, ReadTxn, Result, StorageSchema};

#[cfg(test)]
#[path = "query_image_tests.rs"]
mod tests;
