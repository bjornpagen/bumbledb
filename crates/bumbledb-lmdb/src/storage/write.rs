use super::*;

impl WriteTxn<'_> {
    /// Bulk-loads facts in deterministic schema relation order.
    ///
    /// This is one write transaction: any constraint failure aborts all current
    /// facts, indexes, stats, counters, and dictionary inserts made by
    /// the attempted load.
    pub fn bulk_load(
        &mut self,
        schema: &StorageSchema,
        facts: impl IntoIterator<Item = Fact>,
    ) -> Result<usize> {
        let _span = tracing::debug_span!("bumbledb.storage.bulk_load").entered();
        let mut facts = facts.into_iter().collect::<Vec<_>>();
        tracing::debug!(
            facts = facts.len(),
            "bulk load facts sorted by relation order"
        );
        facts.sort_by_key(|fact| relation_sort_key(schema, fact.relation()));

        let mut inserted = 0;
        for fact in facts {
            if self.insert(schema, fact)? == InsertOutcome::Inserted {
                inserted += 1;
            }
        }
        Ok(inserted)
    }

    /// Inserts a relation fact using set semantics.
    #[tracing::instrument(name = "bumbledb.insert", skip_all, fields(relation = fact.relation()))]
    pub fn insert(&mut self, schema: &StorageSchema, fact: Fact) -> Result<InsertOutcome> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        validate_fact_values(schema.descriptor(), relation, &fact)?;
        let encoded = self.encode_fact(relation_id, relation, &fact, InternMode::Create)?;

        if self.exact_current_fact_exists(relation_id, &encoded)? {
            return Ok(InsertOutcome::AlreadyPresent);
        }

        self.check_foreign_keys(schema, relation, &encoded)?;
        self.check_unique_constraints(schema, relation, &encoded)?;

        self.insert_canonical_fact(relation_id, &encoded)?;
        self.insert_unique_entries(schema, relation_id, relation, &encoded)?;
        self.insert_reverse_fk_entries(schema, relation_id, relation, &encoded)?;
        self.insert_access_entries(schema, relation_id, relation, &encoded)?;
        adjust_relation_fact_count(self, relation_id, 1)?;
        self.ensure_tx_id()?;
        Ok(InsertOutcome::Inserted)
    }

    /// Deletes an exact relation fact using set semantics.
    #[tracing::instrument(name = "bumbledb.delete", skip_all)]
    pub fn delete(&mut self, schema: &StorageSchema, fact: Fact) -> Result<DeleteOutcome> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        validate_fact_values(schema.descriptor(), relation, &fact)?;
        let old_encoded = match self.encode_fact(relation_id, relation, &fact, InternMode::Existing)
        {
            Ok(encoded) => encoded,
            Err(Error::Storage(crate::StorageError::DictionaryValueNotFound { .. })) => {
                return Ok(DeleteOutcome::Absent);
            }
            Err(error) => return Err(error),
        };
        if !self.exact_current_fact_exists(relation_id, &old_encoded)? {
            return Ok(DeleteOutcome::Absent);
        };

        self.check_delete_restrictions(schema, relation, &old_encoded)?;
        self.delete_access_entries(schema, relation_id, relation, &old_encoded)?;
        self.delete_reverse_fk_entries(schema, relation_id, relation, &old_encoded)?;
        self.delete_unique_entries(schema, relation_id, relation, &old_encoded)?;
        self.delete_canonical_fact(relation_id, &old_encoded)?;
        adjust_relation_fact_count(self, relation_id, -1)?;
        self.ensure_tx_id()?;
        Ok(DeleteOutcome::Deleted)
    }

    fn exact_current_fact_exists(&self, relation_id: u16, fact: &EncodedFact) -> Result<bool> {
        let key = canonical_fact_key(relation_id, fact);
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    fn insert_canonical_fact(&mut self, relation_id: u16, fact: &EncodedFact) -> Result<()> {
        let key = canonical_fact_key(relation_id, fact);
        self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
        let id_key = fact_id_key(relation_id, fact);
        if let Some(existing) = self.dbs.index.get(&self.txn, id_key.as_slice())?
            && existing != fact.bytes()
        {
            return Err(Error::hash_collision("fact id"));
        }
        self.dbs
            .index
            .put(&mut self.txn, id_key.as_slice(), fact.bytes())?;
        crate::failpoints::check(crate::failpoints::Failpoint::AfterCanonicalFactPut)?;
        Ok(())
    }

    fn delete_canonical_fact(&mut self, relation_id: u16, fact: &EncodedFact) -> Result<()> {
        let key = canonical_fact_key(relation_id, fact);
        self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        self.dbs
            .index
            .delete(&mut self.txn, fact_id_key(relation_id, fact).as_slice())?;
        Ok(())
    }

    fn encode_fact(
        &mut self,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &Fact,
        mode: InternMode,
    ) -> Result<EncodedFact> {
        let known_fields = relation
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<BTreeSet<_>>();
        for field in fact.values.keys() {
            if !known_fields.contains(field.as_str()) {
                return Err(Error::unknown_field(&relation.name, field));
            }
        }

        let mut bytes = Vec::with_capacity(fact_width(relation));
        for field in &relation.fields {
            let value = fact
                .values
                .get(&field.name)
                .ok_or_else(|| Error::missing_field(&relation.name, &field.name))?;
            bytes.extend_from_slice(&self.encode_value(relation, field, value, &mode)?);
        }
        Ok(EncodedFact {
            relation: RelationId(relation_id),
            bytes,
        })
    }

    fn encode_value(
        &mut self,
        relation: &RelationDescriptor,
        field: &FieldDescriptor,
        value: &Value,
        mode: &InternMode,
    ) -> Result<Vec<u8>> {
        encode_value_with(relation, field, value, |kind, raw| match mode {
            InternMode::Create => self.intern_value(kind, raw),
            InternMode::Existing => self
                .lookup_intern_value(kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind))),
        })
    }

    fn check_foreign_keys(
        &self,
        schema: &StorageSchema,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for constraint in &relation.constraints {
            let ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                ..
            } = constraint
            else {
                continue;
            };
            let (target_relation_id, target) = schema.relation(target_relation)?;
            let key = unique_entry_key_from_source(
                target_relation_id,
                target_constraint,
                relation,
                fact,
                fields,
            )?;
            if self.dbs.index.get(&self.txn, key.as_slice())?.is_none() {
                return Err(Error::foreign_key_violation(
                    &relation.name,
                    name,
                    &target.name,
                ));
            }
        }
        Ok(())
    }

    fn check_unique_constraints(
        &self,
        _schema: &StorageSchema,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for constraint in &relation.constraints {
            let ConstraintDescriptor::Unique { name, fields } = constraint else {
                continue;
            };
            let key = unique_entry_key_from_fact(fact.relation.0, name, relation, fact, fields)?;
            if let Some(existing) = self.dbs.index.get(&self.txn, key.as_slice())? {
                let id = fact_id(fact);
                if existing != id.as_slice() {
                    return Err(Error::unique_violation(&relation.name, name));
                }
            }
        }
        Ok(())
    }

    fn check_delete_restrictions(
        &self,
        schema: &StorageSchema,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let target_relation_id = fact.relation.0;
        for source_relation in schema.descriptor.relations.iter() {
            for constraint in &source_relation.constraints {
                let ConstraintDescriptor::ForeignKey {
                    name,
                    target_relation,
                    target_constraint,
                    ..
                } = constraint
                else {
                    continue;
                };
                if target_relation != &relation.name {
                    continue;
                }
                let Ok((_, target_fields)) = target_unique_constraint(relation, target_constraint)
                else {
                    continue;
                };
                let target_key = target_fields
                    .iter()
                    .map(|field| fact.field(relation, field))
                    .collect::<Result<Vec<_>>>()?
                    .concat();
                let prefix = reverse_fk_prefix(target_relation_id, target_constraint, &target_key);
                let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
                if iter.next().transpose()?.is_some() {
                    return Err(Error::restrict_violation(
                        &relation.name,
                        &source_relation.name,
                        name,
                    ));
                }
            }
        }
        Ok(())
    }

    fn insert_unique_entries(
        &mut self,
        _schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let id = fact_id(fact);
        for constraint in &relation.constraints {
            let ConstraintDescriptor::Unique { name, fields } = constraint else {
                continue;
            };
            let key = unique_entry_key_from_fact(relation_id, name, relation, fact, fields)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), &id)?;
        }
        Ok(())
    }

    fn delete_unique_entries(
        &mut self,
        _schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for constraint in &relation.constraints {
            let ConstraintDescriptor::Unique { name, fields } = constraint else {
                continue;
            };
            let key = unique_entry_key_from_fact(relation_id, name, relation, fact, fields)?;
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        }
        Ok(())
    }

    fn insert_reverse_fk_entries(
        &mut self,
        schema: &StorageSchema,
        source_relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let source_id = fact_id(fact);
        for constraint in &relation.constraints {
            let ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                ..
            } = constraint
            else {
                continue;
            };
            let (target_relation_id, _) = schema.relation(target_relation)?;
            let target_key =
                encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
            let key = reverse_fk_entry_key(
                target_relation_id,
                target_constraint,
                &target_key,
                source_relation_id,
                name,
                &source_id,
            );
            self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
        }
        Ok(())
    }

    fn delete_reverse_fk_entries(
        &mut self,
        schema: &StorageSchema,
        source_relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        let source_id = fact_id(fact);
        for constraint in &relation.constraints {
            let ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                ..
            } = constraint
            else {
                continue;
            };
            let (target_relation_id, _) = schema.relation(target_relation)?;
            let target_key =
                encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
            let key = reverse_fk_entry_key(
                target_relation_id,
                target_constraint,
                &target_key,
                source_relation_id,
                name,
                &source_id,
            );
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
        }
        Ok(())
    }

    fn insert_access_entries(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for layout in schema.layouts_for_relation(relation_id) {
            tracing::trace!(relation = %relation.name, index = %layout.index_name, "put current index entry");
            let key = access_key(layout, relation, fact)?;
            self.dbs.index.put(&mut self.txn, key.as_slice(), &[])?;
            crate::failpoints::check(crate::failpoints::Failpoint::AfterCurrentIndexPut)?;
            adjust_access_entry_count(self, relation_id, layout.index_id, 1)?;
        }
        Ok(())
    }

    fn delete_access_entries(
        &mut self,
        schema: &StorageSchema,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &EncodedFact,
    ) -> Result<()> {
        for layout in schema.layouts_for_relation(relation_id) {
            tracing::trace!(relation = %relation.name, index = %layout.index_name, "delete current index entry");
            let key = access_key(layout, relation, fact)?;
            self.dbs.index.delete(&mut self.txn, key.as_slice())?;
            adjust_access_entry_count(self, relation_id, layout.index_id, -1)?;
        }
        Ok(())
    }

    fn ensure_tx_id(&mut self) -> Result<u64> {
        if let Some(tx_id) = self.active_tx_id {
            return Ok(tx_id);
        }

        let next = read_u64_meta(self, NEXT_TX_ID_KEY)?.unwrap_or(1);
        write_u64_meta(self, NEXT_TX_ID_KEY, next + 1)?;
        self.active_tx_id = Some(next);
        Ok(next)
    }

    fn intern_value(&mut self, kind: u8, raw: &[u8]) -> Result<u64> {
        let _span = tracing::trace_span!(
            "bumbledb.dict_intern",
            kind = dict_kind_name(kind),
            bytes = raw.len()
        )
        .entered();
        if let Some(id) = self.lookup_intern_value(kind, raw)? {
            tracing::trace!(id, existing = true, "dictionary value already interned");
            return Ok(id);
        }

        let id_key = next_dict_id_key(kind);
        let id = read_u64_meta(self, &id_key)?.unwrap_or(1);
        write_u64_meta(self, &id_key, id + 1)?;

        let fwd_key = dict_fwd_key(kind, raw);
        crate::failpoints::check(crate::failpoints::Failpoint::BeforeDictionaryPut)?;
        let mut fwd_value = Vec::with_capacity(8 + raw.len());
        push_u64(&mut fwd_value, id);
        fwd_value.extend_from_slice(raw);
        self.dbs
            .dict
            .put(&mut self.txn, fwd_key.as_slice(), fwd_value.as_slice())?;
        self.dbs
            .dict
            .put(&mut self.txn, dict_rev_key(kind, id).as_slice(), raw)?;
        crate::failpoints::check(crate::failpoints::Failpoint::AfterDictionaryPut)?;
        tracing::trace!(id, existing = false, "dictionary value interned");

        Ok(id)
    }

    fn lookup_intern_value(&self, kind: u8, raw: &[u8]) -> Result<Option<u64>> {
        lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)
    }
}

fn relation_sort_key(schema: &StorageSchema, relation_name: &str) -> usize {
    schema
        .descriptor
        .relations
        .iter()
        .position(|relation| relation.name == relation_name)
        .unwrap_or(usize::MAX)
}
