//! Test substrate for the execution lane: canonical rows in RAM, served
//! through the one C05 source seam — no LMDB, no directories. Suites that
//! need images build them exactly the way production does
//! (`build_from_source` through an [`ImageCache`]), so the walker,
//! interner and column conventions under test are the real ones.
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::api::prepared::source::{HeapRows, QuerySource};
use crate::image::cache::ImageCache;
use crate::image::{RelationImage, ViewEpoch};
use crate::ir::Value;
use crate::schema::Schema;
use bumbledb_theory::schema::RelationId;

pub(crate) struct TestSource {
    schema: Arc<Schema>,
    rows: BTreeMap<RelationId, Vec<Box<[u8]>>>,
    tick: std::cell::Cell<u64>,
}

impl HeapRows for TestSource {
    fn rows(&self, relation: RelationId) -> &[Box<[u8]>] {
        self.rows.get(&relation).map_or(&[], |rows| rows.as_slice())
    }
}

impl TestSource {
    pub(crate) fn new(schema: &Schema, rows: &[(RelationId, Vec<Vec<Value>>)]) -> Self {
        let schema = Arc::new(schema.clone());
        let work = crate::api::db::test_operation().expect("test ledger");
        let mut stored: BTreeMap<RelationId, Vec<Box<[u8]>>> = BTreeMap::new();
        for (relation, facts) in rows {
            let fields = schema.relation(*relation).fields();
            let entry = stored.entry(*relation).or_default();
            for fact in facts {
                let row = crate::canonical::CanonicalRow::encode(fields, fact, &work)
                    .expect("fixture rows are canonical");
                entry.push(Box::from(row.as_bytes()));
            }
            entry.sort();
            entry.dedup();
        }
        Self {
            schema,
            rows: stored,
            tick: std::cell::Cell::new(0),
        }
    }

    pub(crate) fn schema(&self) -> &Schema {
        &self.schema
    }

    /// A fresh heap source (each call is a new tick — the production heap
    /// discipline: no memo can cross instances).
    pub(crate) fn source(&self) -> QuerySource<'_> {
        self.tick.set(self.tick.get() + 1);
        QuerySource::heap(self, self.tick.get(), crate::api::db::test_operation().expect("test ledger"))
    }

    /// Build one relation's image through the production path.
    pub(crate) fn image(&self, cache: &ImageCache, relation: RelationId) -> Arc<RelationImage> {
        let source = self.source();
        let epoch = match cache.slot(relation) {
            crate::image::cache::RelationSlot::Closed(_) => ViewEpoch::Closed,
            crate::image::cache::RelationSlot::Ordinary(_) => source
                .relation_epoch(relation)
                .expect("heap epochs never fail"),
        };
        cache
            .get_or_build_at(&source, &self.schema, relation, epoch)
            .expect("fixture image builds")
            .expect_ready("fixture stays resident")
    }

    /// One-call convenience: a fresh cache plus the relation's image.
    pub(crate) fn image_with_cache(
        &self,
        relation: RelationId,
    ) -> (ImageCache, Arc<RelationImage>) {
        let cache = ImageCache::new(&self.schema);
        let image = self.image(&cache, relation);
        (cache, image)
    }
}
