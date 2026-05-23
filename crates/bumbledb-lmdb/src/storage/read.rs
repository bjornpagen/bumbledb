use super::*;

impl<'env> ReadTxn<'env> {
    /// Scans a whole relation through the canonical fact-set access path.
    #[cfg(test)]
    pub(crate) fn scan_relation<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        let fact_set_access = schema
            .fact_set_index_name(relation_name)
            .ok_or_else(|| Error::unknown_index(relation_name, FACT_SET_ACCESS_NAME))?;
        self.scan_access_with_prefix(schema, relation_name, fact_set_access, &[], None)
    }

    /// Scans an access path by a leading-field prefix.
    #[cfg(test)]
    pub(super) fn scan_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        prefix: &FieldValues,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        if prefix.relation != relation_name {
            return Err(Error::internal(format!(
                "prefix relation {} does not match scan relation {relation_name}",
                prefix.relation
            )));
        }
        let (_, relation) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;

        let encoded_prefix = self.encode_index_prefix(relation, layout, &prefix.values)?;
        self.scan_access_with_prefix(schema, relation_name, index_name, &encoded_prefix, None)
    }

    /// Scans a range index. Bounds are inclusive start and exclusive end.
    #[cfg(test)]
    pub(super) fn scan_range<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        start: Option<Value>,
        end: Option<Value>,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        let (_, relation) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let Some(first_field) = layout.leading_fields.first() else {
            return Err(Error::internal(format!(
                "range index {relation_name}.{index_name} has no leading field"
            )));
        };
        let field = relation
            .field(first_field)
            .ok_or_else(|| Error::unknown_field(&relation.name, first_field))?;

        let start = start
            .as_ref()
            .map(|value| self.encode_read_value(relation, field, value))
            .transpose()?;
        let end = end
            .as_ref()
            .map(|value| self.encode_read_value(relation, field, value))
            .transpose()?;
        let range = EncodedRange {
            offset: access_prefix(layout.relation_id, layout.index_id).len(),
            width: field.value_type.encoded_width(),
            start,
            end,
        };

        self.scan_access_with_prefix(schema, relation_name, index_name, &[], Some(range))
    }

    /// Scans an access path by encoded key prefix without decoding logical facts.
    pub(crate) fn scan_encoded_access_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        encoded_prefix: &[u8],
    ) -> Result<EncodedFactCursor<'borrow, 'env, 'schema>> {
        let (relation_id, _) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let index_prefix = access_prefix(relation_id, layout.index_id);
        let mut scan_prefix = index_prefix.clone();
        scan_prefix.extend_from_slice(encoded_prefix);
        let iter = self
            .dbs
            .index
            .prefix_iter(&self.txn, scan_prefix.as_slice())?;
        Ok(EncodedFactCursor {
            iter,
            layout,
            index_prefix,
            _env: std::marker::PhantomData,
        })
    }

    /// Decodes one encoded query value by logical type.
    pub(crate) fn decode_query_value(&self, value_type: &ValueType, bytes: &[u8]) -> Result<Value> {
        decode_value(self.dbs.dict, &self.txn, value_type, bytes)
    }

    /// Encodes a query value by logical type using existing dictionary entries.
    pub(crate) fn encode_query_value(
        &self,
        value_type: &ValueType,
        value: &Value,
    ) -> Result<Vec<u8>> {
        encode_value_for_type(value_type, value, |kind, raw| {
            lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
        })
    }

    /// Returns the last committed storage transaction ID.
    pub fn last_committed_tx_id(&self) -> Result<u64> {
        Ok(read_u64(&self.dbs.meta, &self.txn, NEXT_TX_ID_KEY)?.unwrap_or(1) - 1)
    }

    /// Returns the stored fact count for a relation.
    pub fn relation_fact_count(&self, schema: &StorageSchema, relation_name: &str) -> Result<u64> {
        let (relation_id, _) = schema.relation(relation_name)?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &relation_fact_count_key(relation_id),
        )?
        .unwrap_or(0))
    }

    /// Returns the stored index-entry count for a current index.
    pub(crate) fn access_entry_count(
        &self,
        schema: &StorageSchema,
        relation_name: &str,
        index_name: &str,
    ) -> Result<u64> {
        let layout = schema.layout(relation_name, index_name).ok_or_else(|| {
            Error::internal(format!("missing index {relation_name}.{index_name}"))
        })?;
        Ok(read_u64(
            &self.dbs.meta,
            &self.txn,
            &access_entry_count_key(layout.relation_id, layout.index_id),
        )?
        .unwrap_or(0))
    }

    /// Counts canonical fact entries for a relation by scanning the canonical namespace.
    #[cfg(test)]
    pub(crate) fn canonical_fact_count(
        &self,
        schema: &StorageSchema,
        relation_name: &str,
    ) -> Result<usize> {
        let (relation_id, _) = schema.relation(relation_name)?;
        let prefix = canonical_fact_prefix(relation_id);
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        let mut count = 0usize;
        while iter.next().transpose()?.is_some() {
            count += 1;
        }
        Ok(count)
    }

    /// Checks whether a current access entry exists for a full fact.
    #[cfg(test)]
    pub(crate) fn access_entry_exists(
        &self,
        schema: &StorageSchema,
        fact: &Fact,
        index_name: &str,
    ) -> Result<bool> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        let layout = schema.layout(&fact.relation, index_name).ok_or_else(|| {
            Error::internal(format!("missing index {}.{index_name}", fact.relation))
        })?;
        let encoded = self.encode_fact_existing(relation_id, relation, fact)?;
        let key = access_key(layout, relation, &encoded)?;
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    /// Checks whether the exact fact exists in the canonical fact set.
    #[cfg(test)]
    pub(crate) fn exact_fact_exists(&self, schema: &StorageSchema, fact: &Fact) -> Result<bool> {
        let (relation_id, relation) = schema.relation(&fact.relation)?;
        let encoded = self.encode_fact_existing(relation_id, relation, fact)?;
        let key = canonical_fact_key(relation_id, &encoded);
        Ok(self.dbs.index.get(&self.txn, key.as_slice())?.is_some())
    }

    /// Looks up an interned string ID.
    #[cfg(test)]
    pub(crate) fn dictionary_string_id(&self, value: &str) -> Result<Option<u64>> {
        lookup_intern_value(&self.dbs.dict, &self.txn, DICT_STRING, value.as_bytes())
    }

    /// Counts reverse dictionary entries across all dictionary kinds.
    pub(crate) fn dictionary_entry_count(&self) -> Result<usize> {
        let prefix = [DICT_REV];
        let mut iter = self.dbs.dict.prefix_iter(&self.txn, &prefix[..])?;
        let mut count = 0;
        while iter.next().transpose()?.is_some() {
            count += 1;
        }
        Ok(count)
    }

    #[cfg(test)]
    pub(super) fn raw_index_value(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.dbs.index.get(&self.txn, key)?.map(ToOwned::to_owned))
    }

    #[cfg(test)]
    pub(super) fn raw_index_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let mut iter = self.dbs.index.prefix_iter(&self.txn, prefix)?;
        let mut keys = Vec::new();
        while let Some((key, _)) = iter.next().transpose()? {
            keys.push(key.to_vec());
        }
        Ok(keys)
    }

    #[cfg(test)]
    fn scan_access_with_prefix<'borrow, 'schema>(
        &'borrow self,
        schema: &'schema StorageSchema,
        relation_name: &str,
        index_name: &str,
        encoded_prefix: &[u8],
        range: Option<EncodedRange>,
    ) -> Result<FactCursor<'borrow, 'env, 'schema>> {
        let _span = tracing::trace_span!(
            "bumbledb.query.scan",
            relation = relation_name,
            index = index_name,
            prefix_bytes = encoded_prefix.len(),
            range = range.is_some()
        )
        .entered();
        let (relation_id, relation) = schema.relation(relation_name)?;
        let layout = schema
            .layout(relation_name, index_name)
            .ok_or_else(|| Error::unknown_index(relation_name, index_name))?;
        let mut prefix = access_prefix(relation_id, layout.index_id);
        prefix.extend_from_slice(encoded_prefix);
        let iter = self.dbs.index.prefix_iter(&self.txn, prefix.as_slice())?;
        Ok(FactCursor {
            iter,
            txn: &self.txn,
            index_db: self.dbs.index,
            dict: self.dbs.dict,
            relation,
            layout,
            range,
        })
    }

    #[cfg(test)]
    fn encode_index_prefix(
        &self,
        relation: &RelationDescriptor,
        layout: &AccessLayout,
        values: &BTreeMap<String, Value>,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut saw_missing = false;

        for field_name in &layout.leading_fields {
            match values.get(field_name) {
                Some(value) if !saw_missing => {
                    let field = relation
                        .field(field_name)
                        .ok_or_else(|| Error::unknown_field(&relation.name, field_name))?;
                    out.extend_from_slice(&self.encode_read_value(relation, field, value)?);
                }
                Some(_) => {
                    return Err(Error::internal(format!(
                        "index prefix for {}.{} is not contiguous",
                        relation.name, layout.index_name
                    )));
                }
                None => saw_missing = true,
            }
        }

        for field_name in values.keys() {
            if !layout
                .leading_fields
                .iter()
                .any(|leading| leading == field_name)
            {
                return Err(Error::unknown_field(&relation.name, field_name));
            }
        }

        Ok(out)
    }

    #[cfg(test)]
    fn encode_read_value(
        &self,
        relation: &RelationDescriptor,
        field: &FieldDescriptor,
        value: &Value,
    ) -> Result<Vec<u8>> {
        encode_value_with(relation, field, value, |kind, raw| {
            lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
        })
    }

    #[cfg(test)]
    pub(super) fn encode_fact_existing(
        &self,
        relation_id: u16,
        relation: &RelationDescriptor,
        fact: &Fact,
    ) -> Result<EncodedFact> {
        let mut bytes = Vec::with_capacity(fact_width(relation));
        for field in &relation.fields {
            let value = fact
                .values
                .get(&field.name)
                .ok_or_else(|| Error::missing_field(&relation.name, &field.name))?;
            bytes.extend_from_slice(&encode_value_with(relation, field, value, |kind, raw| {
                lookup_intern_value(&self.dbs.dict, &self.txn, kind, raw)?
                    .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
            })?);
        }
        Ok(EncodedFact {
            relation: RelationId(relation_id),
            bytes,
        })
    }
}
