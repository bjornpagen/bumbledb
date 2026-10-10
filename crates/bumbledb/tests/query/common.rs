//! Store and render helpers shared by the query tests.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::ir::render::render;
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Db, Query, Schema, Theory, WorkContext};

pub fn work() -> WorkContext {
    WorkContext::new()
}

/// A store path no other test shares, removed on drop.
pub struct TempDir {
    root: PathBuf,
    path: PathBuf,
}

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("bumbledb-query-{tag}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create the test directory");
        Self {
            path: root.join("store.bdb"),
            root,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Prepares `query` against a fresh store of `theory` (which validates it)
/// and returns its rendered text.
pub fn pin<S: Theory + Copy>(tag: &str, theory: S, query: &Query) -> String {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), theory, work())
        .expect("create the theory's store")
        .expect("accepted");
    db.prepare(query, work())
        .unwrap_or_else(|error| panic!("{tag}: the query validates: {error:?}"));
    let schema: Schema = theory.descriptor().validate().expect("a valid theory");
    render(&schema, query)
}
