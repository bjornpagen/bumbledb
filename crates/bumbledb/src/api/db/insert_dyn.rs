use super::{AcceptedCollection, MutationReport, WriteTx};
use crate::error::Result;
use crate::ir::Value;
use crate::storage::delta::Disposition;
use bumbledb_theory::schema::RelationId;

impl<S> WriteTx<'_, S> {
    /// Records a collection of dynamic inserts: one [`Value`] row per
    /// fact, in declaration order. Empty is lawful. Singleton is `[row]`.
    ///
    /// The whole collection is parsed before any row enters the delta.
    /// Then each row is interned and inserted. Relation lookup, closed
    /// refusal, and the field roster run once.
    ///
    /// # Errors
    ///
    /// `ClosedRelationWrite` on a closed relation; `FactShape` on an
    /// arity, type-kind, fixed-interval width/ray, `bytes<N>` length, or
    /// UTF-8 mismatch (`schema::value_matches` is the rule set — ETL
    /// input is data, so shape problems are typed); otherwise as
    /// [`WriteTx::insert`].
    pub fn insert_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        self.mutation.load_dyn(rel, facts)
    }

    /// Records one shape-proved collection of inserts — the bridge
    /// transport lane (`proposals/one-representation/20`): the collection
    /// was parsed at construction, so only the write-side walls remain
    /// (poison, closed, roster re-anchor). Semantics and reports exactly
    /// as [`WriteTx::insert_dyn`]. Bridge/harness surface (not embedding
    /// API).
    ///
    /// # Errors
    ///
    /// As [`WriteTx::insert_dyn`], minus the per-row shape family the
    /// constructor already refused.
    #[doc(hidden)]
    pub fn insert_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.mutation
            .apply_accepted(collection, Disposition::Insert)
    }
}
