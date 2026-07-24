use super::{Fact, Key, Snapshot};
use crate::api::prepared::{Answers, BindValue, ParamArg, PreparedQuery};
use crate::error::{FactShapeError, Result};
use crate::ir::Value;
use crate::storage::{dict, read};
use bumbledb_theory::schema::RelationId;

impl<S> Snapshot<'_, S> {
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
        prepared: &mut PreparedQuery<'_, S>,
        params: &[BindValue<'_>],
        out: &mut Answers,
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
        prepared: &mut PreparedQuery<'_, S>,
        params: &[BindValue<'_>],
    ) -> Result<Answers> {
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
        prepared: &mut PreparedQuery<'_, S>,
        args: &[ParamArg<'_>],
        out: &mut Answers,
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
        prepared: &mut PreparedQuery<'_, S>,
        args: &[ParamArg<'_>],
    ) -> Result<Answers> {
        prepared.execute_collect_args(&self.txn, self.cache, args)
    }

    /// Plan introspection with ANALYZE semantics: executes with counting instrumentation
    /// and returns the answers alongside the rendered report.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute`].
    pub fn introspect(
        &self,
        prepared: &mut PreparedQuery<'_, S>,
        params: &[BindValue<'_>],
    ) -> Result<(Answers, String)> {
        prepared.introspect(&self.txn, self.cache, params)
    }

    /// ANALYZE with structured output: the answers alongside
    /// [`crate::api::stats::ExecutionStats`] — what `introspect` renders,
    /// as data.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::execute`].
    pub fn profile(
        &self,
        prepared: &mut PreparedQuery<'_, S>,
        params: &[BindValue<'_>],
    ) -> Result<(Answers, crate::api::stats::ExecutionStats)> {
        prepared.profile(&self.txn, self.cache, params)
    }

    /// The export surface (`70-api.md` ETL story): a full-relation scan
    /// yielding decoded dynamic facts (strings resolved; bytes<N> values
    /// are inline) in `row_id` order — a storage stream, not a query
    /// result set.
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
            crate::encoding::decode_values(bytes, layout, |id| {
                Ok(Box::from(dict::resolve(&self.txn, id)?))
            })
        }))
    }
}

impl<S> Snapshot<'_, S> {
    /// Committed-state membership of a typed fact — the snapshot sibling
    /// of [`super::WriteTx::contains`], completing the point-operation
    /// matrix (typed/dyn × write/snapshot, `docs/architecture/70-api.md`
    /// § point reads): the fact encodes through [`Fact::encode_read`] —
    /// the committed dictionary, never minting — so a string or bytes
    /// value the dictionary does not know proves the fact absent and the
    /// probe short-circuits to `false`. A **closed** relation answers
    /// from its sealed extension (virtual storage — no `M` rows exist).
    ///
    /// # Errors
    ///
    /// `Lmdb` on the membership probe or dictionary reads.
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.with_scratch(|scratch| {
            if !fact.encode_read(self, &mut scratch.bytes)? {
                return Ok(false);
            }
            if let Some(extension) = self.schema.relation(F::RELATION).extension() {
                return Ok(extension
                    .iter()
                    .any(|row| row.fact.as_ref() == scratch.bytes.as_slice()));
            }
            read::fact_row(&self.txn, F::RELATION, &scratch.bytes).map(|row| row.is_some())
        })
    }

    /// Committed-state membership of a dynamic fact — the snapshot
    /// sibling of [`super::WriteTx::contains_dyn`], completing the
    /// schema-generic read surface (`docs/architecture/70-api.md` § the
    /// dyn lane): one [`Value`] per field in declaration order, probed
    /// against this snapshot's one consistent state. Never interns: a
    /// string value the committed dictionary does not know proves the
    /// fact absent. A **closed** relation answers from its sealed
    /// extension (virtual storage — no `M` rows exist).
    ///
    /// # Errors
    ///
    /// `FactShape` on an unknown relation id or an arity/type/UTF-8
    /// mismatch (typed, never a panic — ids at this surface are data);
    /// `Lmdb` on the probe or dictionary reads.
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        let Some(relation) = self.schema.relation_checked(rel) else {
            return Err(FactShapeError::UnknownRelation { relation: rel }.into());
        };
        self.with_scratch(|scratch| {
            if !super::encode_dyn::dyn_value_refs(
                rel,
                values,
                relation.fields(),
                &mut scratch.refs,
                |text| dict::lookup_str(&self.txn, text),
            )? {
                return Ok(false);
            }
            crate::encoding::encode_fact(&scratch.refs, relation.layout(), &mut scratch.bytes);
            if let Some(extension) = relation.extension() {
                return Ok(extension
                    .iter()
                    .any(|row| row.fact.as_ref() == scratch.bytes.as_slice()));
            }
            read::fact_row(&self.txn, rel, &scratch.bytes).map(|row| row.is_some())
        })
    }

    /// Point lookup of the full fact through any key statement of
    /// `relation`, against committed state — the snapshot sibling of
    /// [`super::WriteTx::get_dyn`]: `key_values` are the key statement's
    /// projected fields in statement projection order, type-checked
    /// against the projection; the decoded fact comes back as owned
    /// [`Value`]s (strings resolved through the committed dictionary). A
    /// **closed** relation resolves against its sealed extension.
    ///
    /// # Errors
    ///
    /// `FactShape` when `relation` is unknown, `key` is not one of its
    /// `Functionality` statements, or `key_values` mismatch the
    /// projection in arity or type; `Lmdb`/`Corruption` from storage.
    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: bumbledb_theory::schema::StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        let (_, statement) = super::get::key_statement_of(self.schema, relation, key)?;
        self.with_scratch(|scratch| {
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if !super::get::encode_determinant_with(
                self.schema,
                relation,
                &statement.projection,
                key_values,
                key_bytes,
                |text| dict::lookup_str(&self.txn, text),
            )? {
                return Ok(None);
            }
            let rel = self.schema.relation(relation);
            let bytes = if rel.is_closed() {
                super::get::closed_fact_by_determinant(
                    rel,
                    statement,
                    &key_bytes[read::DETERMINANT_KEY_HEADER..],
                )
            } else if statement.fresh_row {
                // The fresh-row auto-key reads `F` directly: its
                // determinant IS the row id (R16).
                read::fact_at(
                    &self.txn,
                    self.schema,
                    relation,
                    super::get::fresh_row_id(&key_bytes[read::DETERMINANT_KEY_HEADER..]),
                )?
            } else {
                read::fact_for_key(&self.txn, self.schema, relation, key_bytes)?
            };
            bytes
                .map(|fact| {
                    crate::encoding::decode_values_keyed(
                        fact,
                        rel.layout(),
                        &statement.projection,
                        key_values,
                        |id| Ok(Box::from(dict::resolve(&self.txn, id)?)),
                    )
                })
                .transpose()
        })
    }

    /// Point lookup of the full fact through a typed key value ([`Key`]),
    /// against committed state — the committed-state sibling of
    /// [`super::WriteTx::get`]: the key value's TYPE carries the relation
    /// and the key statement (`K::STATEMENT`, computed at `schema!`
    /// expansion), so which key FD a read goes through is never a runtime
    /// question. A **closed** relation resolves against its sealed
    /// extension. No `Db`-level sugar fronts this — the Rust read scope IS
    /// `db.read(|snap| snap.get(key))` (recorded decision: the freeze
    /// keeps `Db` minimal; the TS surface carries the symmetry sugar).
    ///
    /// Variable-width fields of the returned fact borrow from the
    /// snapshot's dictionary at the snapshot lifetime — copy
    /// (`to_owned()`) what must outlive it.
    ///
    /// # Errors
    ///
    /// `FactShape` when a manual `Key` impl lies about its statement
    /// (typed, never a panic); `Lmdb`/`Corruption` from storage.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `snap.get(id)`: fresh \
                  newtypes are Copy and generated key structs are small — \
                  by-value keeps every call site free of `&` noise"
    )]
    pub fn get<'snap, K: Key<'snap, Schema = S>>(&'snap self, key: K) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'snap>>::RELATION;
        let (_, statement) = super::get::key_statement_of(self.schema, relation, K::STATEMENT)?;
        self.with_scratch(|scratch| {
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if !key.determinant_read(self, key_bytes)? {
                return Ok(None);
            }
            let rel = self.schema.relation(relation);
            let bytes = if rel.is_closed() {
                super::get::closed_fact_by_determinant(
                    rel,
                    statement,
                    &key_bytes[read::DETERMINANT_KEY_HEADER..],
                )
            } else if statement.fresh_row {
                // The fresh-row auto-key reads `F` directly: its
                // determinant IS the row id (R16).
                read::fact_at(
                    &self.txn,
                    self.schema,
                    relation,
                    super::get::fresh_row_id(&key_bytes[read::DETERMINANT_KEY_HEADER..]),
                )?
            } else {
                read::fact_for_key(&self.txn, self.schema, relation, key_bytes)?
            };
            bytes.map(|fact| K::Fact::decode(self, fact)).transpose()
        })
    }

    /// The typed sibling of [`Snapshot::scan`]: decodes each fact into its
    /// `schema!`-generated struct via [`Fact::decode`]. The dynamic form
    /// remains the ETL pairing for [`Db::bulk_load`]; this one is for
    /// hosts that want their own types back. Variable-width fields borrow
    /// from the snapshot's dictionary at the snapshot lifetime — copy
    /// (`to_owned()`) what must outlive it.
    ///
    /// # Errors
    ///
    /// As [`Snapshot::scan`].
    pub fn scan_facts<'snap, F: Fact<'snap, Schema = S>>(
        &'snap self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'snap> {
        let iter = read::scan(&self.txn, self.schema, F::RELATION)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            F::decode(self, bytes)
        }))
    }
}
