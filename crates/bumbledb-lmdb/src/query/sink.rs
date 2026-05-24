use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::query_ir::TypedFindTerm;

use crate::query::model::NormalizedQuery;
use crate::storage_v5;
use crate::tuple::EncodedTuple;
use crate::{Error, QueryResultSet, ReadTxn, Result, ResultColumn, ResultFact};

#[derive(Clone, Debug, Default)]
pub(super) struct Binding {
    pub(super) values: BTreeMap<usize, Vec<u8>>,
}

impl Binding {
    pub(super) fn extend_from_tuple(
        &self,
        vars: &[usize],
        tuple: &EncodedTuple,
        query: &NormalizedQuery,
    ) -> Result<Option<Self>> {
        let mut next = self.clone();
        let mut offset = 0;
        for variable in vars {
            let width = query.variables[*variable].value_type.encoded_width();
            let Some(bytes) = tuple.bytes().get(offset..offset + width) else {
                return Err(Error::corrupt("cover tuple width is too short"));
            };
            match next.values.get(variable) {
                Some(existing) if existing != bytes => return Ok(None),
                Some(_) => {}
                None => {
                    next.values.insert(*variable, bytes.to_vec());
                }
            }
            offset += width;
        }
        if offset != tuple.bytes().len() {
            return Err(Error::corrupt("cover tuple width has trailing bytes"));
        }
        Ok(Some(next))
    }
}

pub(super) trait BindingSink {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()>;
}

pub(super) struct ProjectionSink<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
    facts: BTreeSet<ResultFact>,
}

impl<'txn, 'env> ProjectionSink<'txn, 'env> {
    pub(super) fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self {
            txn,
            facts: BTreeSet::new(),
        }
    }

    pub(super) fn finish(self, query: &NormalizedQuery) -> Result<QueryResultSet> {
        Ok(QueryResultSet::new(
            result_columns(query)?,
            self.facts.into_iter().collect(),
        ))
    }
}

impl BindingSink for ProjectionSink<'_, '_> {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()> {
        let mut fact = Vec::with_capacity(query.find.len());
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { variable } => {
                    let value_type = &query.variables[*variable].value_type;
                    let bytes = binding.values.get(variable).ok_or_else(|| {
                        Error::corrupt(format!("projection variable {variable} is unbound"))
                    })?;
                    fact.push(storage_v5::decode_value(self.txn, value_type, bytes)?);
                }
            }
        }
        self.facts.insert(fact);
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct CountingSink {
    pub(super) count: usize,
}

#[cfg(test)]
impl BindingSink for CountingSink {
    fn consume(&mut self, _query: &NormalizedQuery, _binding: &Binding) -> Result<()> {
        self.count += 1;
        Ok(())
    }
}

fn result_columns(query: &NormalizedQuery) -> Result<Vec<ResultColumn>> {
    query
        .find
        .iter()
        .map(|term| match term {
            TypedFindTerm::Variable { variable } => query
                .variables
                .get(*variable)
                .map(|variable| ResultColumn::Variable(variable.name.clone()))
                .ok_or_else(|| Error::invalid_query(format!("unknown projection {variable}"))),
        })
        .collect()
}
