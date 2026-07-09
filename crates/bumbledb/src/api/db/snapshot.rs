use super::{Fact, Snapshot};
use crate::api::prepared::{ParamArg, PreparedQuery, ResultBuffer};
use crate::encoding::{decode_field, ValueRef};
use crate::error::{FactShapeError, Result};
use crate::ir::Value;
use crate::schema::RelationId;
use crate::storage::{dict, read};

impl Snapshot<'_> {
    /// Executes a prepared query with positional parameters into the
    /// caller's reusable buffer (the zero-alloc path).
    ///
    /// # Errors
    ///
    /// `ParamCountMismatch`/`ParamTypeMismatch` at bind time; `Overflow`
    /// from aggregate finalization; `Lmdb`/`Corruption` from storage. A
    /// query error aborts the query; the snapshot remains usable.
    pub fn execute(
        &self,
        prepared: &mut PreparedQuery<'_>,
        params: &[Value],
        out: &mut ResultBuffer,
    ) -> Result<()> {
        prepared.execute(&self.txn, self.cache, params, out)
    }

    /// Convenience path: a fresh buffer per call.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute`].
    pub fn execute_collect(
        &self,
        prepared: &mut PreparedQuery<'_>,
        params: &[Value],
    ) -> Result<ResultBuffer> {
        prepared.execute_collect(&self.txn, self.cache, params)
    }

    /// Executes with mixed scalar/set parameter arguments
    /// (`docs/architecture/70-api.md` § facts and results): one
    /// [`ParamArg`] per `ParamId` position — scalars as values, param
    /// sets as slices (deduplicated at bind into the prepared query's
    /// pooled storage).
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute`], plus the precise per-position bind
    /// errors: `ParamSetExpected` (a scalar where the query binds a
    /// set), `ParamScalarExpected` (a set where it binds a scalar), and
    /// `ParamElementTypeMismatch` (a mistyped set element).
    pub fn execute_args(
        &self,
        prepared: &mut PreparedQuery<'_>,
        args: &[ParamArg<'_>],
        out: &mut ResultBuffer,
    ) -> Result<()> {
        prepared.execute_args(&self.txn, self.cache, args, out)
    }

    /// [`Snapshot::execute_args`]'s fresh-buffer convenience.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute_args`].
    pub fn execute_collect_args(
        &self,
        prepared: &mut PreparedQuery<'_>,
        args: &[ParamArg<'_>],
    ) -> Result<ResultBuffer> {
        prepared.execute_collect_args(&self.txn, self.cache, args)
    }

    /// EXPLAIN ANALYZE (docs/architecture/30-execution.md): executes with counting instrumentation
    /// and returns the rows alongside the rendered report.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute`].
    pub fn explain(
        &self,
        prepared: &mut PreparedQuery<'_>,
        params: &[Value],
    ) -> Result<(ResultBuffer, String)> {
        prepared.explain(&self.txn, self.cache, params)
    }

    /// ANALYZE with structured output: the rows alongside
    /// [`crate::api::stats::ExecutionStats`] — what `explain` renders,
    /// as data.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute`].
    pub fn profile(
        &self,
        prepared: &mut PreparedQuery<'_>,
        params: &[Value],
    ) -> Result<(ResultBuffer, crate::api::stats::ExecutionStats)> {
        prepared.profile(&self.txn, self.cache, params)
    }

    /// The export surface (`60-api.md` ETL story): a full-relation scan
    /// yielding decoded dynamic facts (strings and bytes resolved) in
    /// `row_id` order — a storage stream, not a query result set.
    ///
    /// # Errors
    ///
    /// `Lmdb` on cursor open; per-item `Corruption` is a hard error — stop
    /// at the first.
    pub fn scan(&self, rel: RelationId) -> Result<impl Iterator<Item = Result<Vec<Value>>> + '_> {
        let Some(relation) = self.schema.relation_checked(rel) else {
            return Err(FactShapeError::UnknownRelation { relation: rel }.into());
        };
        let layout = relation.layout();
        let iter = read::scan(&self.txn, self.schema, rel)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            (0..layout.field_count())
                .map(|idx| {
                    Ok(match decode_field(bytes, layout, idx)? {
                        ValueRef::Bool(v) => Value::Bool(v),
                        ValueRef::U64(v) => Value::U64(v),
                        ValueRef::I64(v) => Value::I64(v),
                        ValueRef::Enum(ordinal) => Value::Enum(ordinal),
                        ValueRef::String(id) => Value::String(Box::from(dict::resolve(
                            &self.txn,
                            id,
                            dict::TAG_STRING,
                        )?)),
                        ValueRef::Bytes(id) => {
                            Value::Bytes(Box::from(dict::resolve(&self.txn, id, dict::TAG_BYTES)?))
                        }
                        ValueRef::IntervalU64(start, end) => Value::IntervalU64(start, end),
                        ValueRef::IntervalI64(start, end) => Value::IntervalI64(start, end),
                    })
                })
                .collect()
        }))
    }
}

impl Snapshot<'_> {
    /// The typed sibling of [`Snapshot::scan`]: decodes each fact into its
    /// `schema!`-generated struct via [`Fact::decode`]. The dynamic form
    /// remains the ETL pairing for [`Db::bulk_load`]; this one is for
    /// hosts that want their own types back.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::scan`].
    pub fn scan_facts<F: Fact>(&self) -> Result<impl Iterator<Item = Result<F>> + '_> {
        let iter = read::scan(&self.txn, self.schema, F::RELATION)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            F::decode(self, bytes)
        }))
    }
}
