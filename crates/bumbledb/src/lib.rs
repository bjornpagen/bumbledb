//! bumbledb: an embedded, typed, set-semantic relational database over LMDB.
//!
//! The normative design lives in `docs/architecture/`; the build plan in
//! `docs/implementation/`.

// 64-bit only (docs/architecture/00-product.md): `usize` is 8 bytes everywhere
// and no design decision accommodates narrower platforms. Building for a
// 32-bit target (e.g. `--target i686-unknown-linux-gnu`) fails with this
// explicit error instead of miscompiling pointer-width assumptions.
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only (docs/architecture/00-product.md)");

#[cfg(feature = "alloc-counter")]
pub mod alloc_counter;
pub mod api;
pub mod arena;
pub mod encoding;
pub mod error;
pub mod exec;
pub mod image;
pub mod ir;
pub mod plan;
pub mod schema;
pub mod storage;

#[cfg(test)]
pub(crate) mod testutil {
    //! Shared test scaffolding: a self-cleaning temp directory (no external
    //! dev-dependency — deps stay exactly heed + blake3).

    use std::path::{Path, PathBuf};

    pub struct TempDir(PathBuf);

    impl TempDir {
        /// Creates (or wipes and recreates) a per-test directory. `tag` must
        /// be unique per test function so parallel tests never collide.
        pub fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!("bumbledb-test-{tag}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create test dir");
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
