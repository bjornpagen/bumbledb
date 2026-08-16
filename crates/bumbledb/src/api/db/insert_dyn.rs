use super::{MutationReport, WriteTx};
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
        let rows: Vec<_> = facts.into_iter().collect();
        let parsed = self.parse_dyn_collection(rel, &rows)?;
        self.apply_collection(rel, Disposition::Insert, parsed, |tx, row, bytes| {
            let layout = tx.schema.relation(rel).layout();
            tx.encode_parsed_mint(&row, layout, bytes)
        })
    }
}
