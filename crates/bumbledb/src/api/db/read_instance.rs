use super::{Fact, Key, Probe, ReadInstance};
use crate::api::prepared::{Answers, ParamArg, PreparedQuery};
use crate::error::{DynIdError, Result};
use crate::ir::{Query, Value};
use crate::storage::catalog::CatalogRead;
use crate::storage::read;
use bumbledb_theory::schema::RelationId;

impl<S> ReadInstance<'_, S> {
    /// Prepares a query against this lease's catalog and statistics.
    ///
    /// # Errors
    ///
    /// As [`crate::OwnedInstance::prepare`].
    pub fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>> {
        let catalog = self.core.source.catalog();
        crate::api::prepared::prepare_on(
            &self.core.identity,
            &catalog,
            &self.core.source,
            std::sync::Arc::clone(&self.core.schema),
            query,
        )
    }

    /// Exact cardinality of `relation` at this lease's committed view —
    /// THE public spelling for stored cardinality (one-representation PRD
    /// 40): a structural read of the maintained counter
    /// (`StatKind::RowCount`, folded transactionally at every commit
    /// since format 8, O(1) to read, pinned equal to the scan count —
    /// `row_count_equals_scan_count_after_mixed_commits`) — never a scan,
    /// never an estimate, no allocation. The read runs inside the lease's
    /// one `ReadTxn`, so `count` and [`ReadInstance::scan`] observe the
    /// same committed view by construction. A **closed** relation answers
    /// its sealed extension length (virtual storage — the stored counter
    /// never exists for it), the same arm as
    /// [`crate::OwnedInstance::count`].
    ///
    /// # Errors
    ///
    /// `UnknownRelation`; `Corruption` on a malformed counter.
    ///
    /// # Panics
    ///
    /// Never: a sealed extension is schema data admitted at declaration —
    /// its length always fits `u64`.
    pub fn count(&self, relation: RelationId) -> Result<u64> {
        let Some(rel) = self.core.schema.relation_checked(relation) else {
            return Err(DynIdError::UnknownRelation { relation }.into());
        };
        match rel.body().closed_rows() {
            Some(rows) => Ok(u64::try_from(rows.len()).expect("bounded extension")),
            None => self.core.source.catalog().row_count(relation),
        }
    }

    /// Executes a prepared query with positional parameters into the
    /// caller's reusable buffer (the zero-alloc path).
    ///
    /// # Errors
    ///
    /// `ParamCountMismatch`/`ParamTypeMismatch` at bind time; `Overflow`
    /// from aggregate finalization; `Lmdb`/`Corruption` from storage. A
    /// query error aborts the query; the instance remains usable.
    pub fn execute<'p, P: crate::api::prepared::BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        prepared.execute(self.txn(), self.cache(), params, out)
    }

    /// Convenience path: a fresh buffer per call.
    ///
    /// # Errors
    ///
    /// As [`ReadInstance::execute`].
    pub fn execute_collect<'p, P: crate::api::prepared::BindArgs<'p>>(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: P,
    ) -> Result<Answers> {
        prepared.execute_collect(self.txn(), self.cache(), params)
    }

    /// Plan introspection with ANALYZE semantics: executes with counting instrumentation
    /// and returns the answers alongside the rendered report. Takes the
    /// mixed [`ParamArg`] entry — execute-symmetry (R13): whatever
    /// [`ReadInstance::execute`] binds, introspection binds.
    ///
    /// Harness-only (not embedding API).
    ///
    /// # Errors
    ///
    /// As [`ReadInstance::execute`].
    #[doc(hidden)]
    pub fn introspect(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, String)> {
        prepared.introspect(self.txn(), self.cache(), params)
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
        let Some(_relation) = self.core.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let iter = read::scan(self.txn(), self.core.schema.as_ref(), rel)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            crate::encoding::decode_values(bytes, |id| {
                Ok(Box::from(super::plumbing::resolve_string(
                    self,
                    crate::encoding::InternId::from_raw(id),
                )?))
            })
        }))
    }
}

impl<S> ReadInstance<'_, S> {
    /// Committed-state membership of a typed fact — the store sibling
    /// of [`super::WriteTx::contains`], completing the point-operation
    /// matrix (typed/dyn × write/read, `docs/architecture/70-api.md`
    /// § point reads): the fact encodes through [`Fact::encode_probe`] —
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
            if matches!(
                fact.encode_probe(self, &mut scratch.bytes)?,
                Probe::ProvablyAbsent
            ) {
                return Ok(false);
            }
            if let Some(extension) = self.core.schema.relation(F::RELATION).body().closed_rows() {
                return Ok(extension
                    .iter()
                    .any(|row| row.fact.as_ref() == scratch.bytes.as_slice()));
            }
            let hash = crate::encoding::fact_hash(&scratch.bytes);
            self.core
                .source
                .catalog()
                .membership_row(F::RELATION, &hash)
                .map(|row| row.is_some())
        })
    }

    /// Committed-state membership of a dynamic fact — the store
    /// sibling of [`super::WriteTx::contains_dyn`], completing the
    /// schema-generic read surface (`docs/architecture/70-api.md` § the
    /// dyn lane): one [`Value`] per field in declaration order, probed
    /// against this lease's one consistent state. Never interns: a
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
        let Some(relation) = self.core.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        self.with_scratch(|scratch| {
            if !super::collection::intern_value_row(
                rel,
                relation.fields(),
                values,
                &mut scratch.refs,
                |text| self.core.source.catalog().dict_lookup(text.as_bytes()),
            )? {
                return Ok(false);
            }
            crate::encoding::encode_fact(&scratch.refs, relation.layout(), &mut scratch.bytes);
            if let Some(extension) = relation.body().closed_rows() {
                return Ok(extension
                    .iter()
                    .any(|row| row.fact.as_ref() == scratch.bytes.as_slice()));
            }
            let hash = crate::encoding::fact_hash(&scratch.bytes);
            self.core
                .source
                .catalog()
                .membership_row(rel, &hash)
                .map(|row| row.is_some())
        })
    }

    /// Point lookup of the full fact through any key statement of
    /// `relation`, against committed state — the committed-state sibling of
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
        let mut out = Vec::new();
        Ok(self
            .get_dyn_into(relation, key, key_values, &mut out)?
            .then_some(out))
    }

    /// [`ReadInstance::get_dyn`] into a caller-provided buffer — the pooled
    /// point-read lane (docs/architecture/70-api.md § point reads): the
    /// values `Vec` is the caller's, its capacity retained across gets,
    /// so a warm keyed get's allocator traffic shrinks to the
    /// variable-width payload boxes alone (the key-encode scratch was
    /// already pooled, R15). `Ok(true)` = hit, `out` holds the fact's
    /// fields in declaration order; `Ok(false)` = no fact, `out` empty.
    ///
    /// # Errors
    ///
    /// As [`ReadInstance::get_dyn`].
    pub fn get_dyn_into(
        &self,
        relation: RelationId,
        key: bumbledb_theory::schema::StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        out.clear();
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let (_, statement) =
            super::get::key_statement_of(self.core.schema.as_ref(), relation, key)?;
        let hit = self.with_scratch(|scratch| {
            let catalog = self.core.source.catalog();
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if !super::get::encode_determinant_with(
                self.core.schema.as_ref(),
                relation,
                &statement.projection,
                key_values,
                key_bytes,
                |text| catalog.dict_lookup(text.as_bytes()),
            )? {
                return Ok(false);
            }
            let rel = self.core.schema.relation(relation);
            let determinant = &key_bytes[read::DETERMINANT_KEY_HEADER..];
            let bytes = match super::get::point_read(rel, statement, determinant) {
                super::get::PointRead::Closed => {
                    super::get::closed_fact_by_determinant(rel, statement, determinant)
                }
                super::get::PointRead::FreshRow { row_id } => {
                    match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            read::check_width(&self.core.schema, relation, row_id, fact)?;
                            Some(fact)
                        }
                        None => None,
                    }
                }
                super::get::PointRead::Determinant => match catalog.determinant_row(key_bytes)? {
                    Some(row_id) => match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            read::check_width(&self.core.schema, relation, row_id, fact)?;
                            Some(fact)
                        }
                        None => None,
                    },
                    None => None,
                },
            };
            let Some(fact) = bytes else {
                return Ok(false);
            };
            crate::encoding::decode_values_keyed_into(
                rel.layout().encoded(fact),
                &statement.projection,
                key_values,
                |id| {
                    Ok(Box::from(super::plumbing::resolve_string(
                        self,
                        crate::encoding::InternId::from_raw(id),
                    )?))
                },
                out,
            )?;
            Ok(true)
        })?;
        span.set_flag(hit);
        span.end();
        Ok(hit)
    }

    /// Point lookup of the full fact through a typed key value ([`Key`]),
    /// against committed state — the committed-state sibling of
    /// [`super::WriteTx::get`]: the key value's TYPE carries the relation
    /// and the key statement (`K::STATEMENT`, computed at `schema!`
    /// expansion), so which key FD a read goes through is never a runtime
    /// question. A **closed** relation resolves against its sealed
    /// extension. No `Db`-level sugar fronts this — the Rust read scope IS
    /// `db.read(|instance| instance.get(key))` (recorded decision: the freeze
    /// keeps `Db` minimal; the TS surface carries the symmetry sugar).
    ///
    /// Variable-width fields of the returned fact borrow from the
    /// lease's dictionary at the lease lifetime — copy
    /// (`to_owned()`) what must outlive it.
    ///
    /// # Errors
    ///
    /// `FactShape` when a manual `Key` impl lies about its statement
    /// (typed, never a panic); `Lmdb`/`Corruption` from storage.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `instance.get(id)`: fresh \
                  newtypes are Copy and generated key structs are small — \
                  by-value keeps every call site free of `&` noise"
    )]
    pub fn get<'lease, K: Key<'lease, Schema = S>>(
        &'lease self,
        key: K,
    ) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'lease>>::RELATION;
        let mut span = crate::obs::span(crate::obs::names::POINT_READ);
        let (_, statement) =
            super::get::key_statement_of(self.core.schema.as_ref(), relation, K::STATEMENT)?;
        let result = self.with_scratch(|scratch| {
            let catalog = self.core.source.catalog();
            let key_bytes = &mut scratch.bytes;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if matches!(
                key.encode_determinant(self, key_bytes)?,
                Probe::ProvablyAbsent
            ) {
                return Ok(None);
            }
            let rel = self.core.schema.relation(relation);
            let determinant = &key_bytes[read::DETERMINANT_KEY_HEADER..];
            match super::get::point_read(rel, statement, determinant) {
                super::get::PointRead::Closed => {
                    match super::get::closed_fact_by_determinant(rel, statement, determinant) {
                        Some(fact) => K::Fact::decode(self, fact).map(Some),
                        None => Ok(None),
                    }
                }
                super::get::PointRead::FreshRow { row_id } => {
                    match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            let view =
                                read::check_width(&self.core.schema, relation, row_id, fact)?;
                            K::Fact::decode(self, view.bytes()).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                super::get::PointRead::Determinant => match catalog.determinant_row(key_bytes)? {
                    Some(row_id) => match catalog.fetch_fact(relation, row_id)? {
                        Some(fact) => {
                            let view =
                                read::check_width(&self.core.schema, relation, row_id, fact)?;
                            K::Fact::decode(self, view.bytes()).map(Some)
                        }
                        None => Ok(None),
                    },
                    None => Ok(None),
                },
            }
        });
        if let Ok(found) = &result {
            span.set_flag(found.is_some());
        }
        span.end();
        result
    }

    /// The typed sibling of [`ReadInstance::scan`]: decodes each fact into its
    /// `schema!`-generated struct via [`Fact::decode`]. The dynamic form
    /// is the ETL pairing for [`crate::WriteTx::insert_dyn`] under
    /// [`crate::Db::write`]; this one is for hosts that want their own
    /// types back. Variable-width fields borrow from the lease's
    /// dictionary at the lease lifetime — copy (`to_owned()`) what
    /// must outlive it.
    ///
    /// # Errors
    ///
    /// As [`ReadInstance::scan`].
    pub fn scan_facts<'lease, F: Fact<'lease, Schema = S>>(
        &'lease self,
    ) -> Result<impl Iterator<Item = Result<F>> + 'lease> {
        let iter = read::scan(self.txn(), self.core.schema.as_ref(), F::RELATION)?;
        Ok(iter.map(move |entry| {
            let (_, bytes) = entry?;
            F::decode(self, bytes.bytes())
        }))
    }
}
