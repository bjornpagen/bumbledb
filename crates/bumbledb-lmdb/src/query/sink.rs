use std::collections::BTreeSet;

use bumbledb_core::query_ir::TypedFindTerm;

use crate::query::model::NormalizedQuery;
use crate::storage_v5;
use crate::tuple::EncodedTupleRef;
use crate::{Error, QueryResultSet, ReadTxn, Result, ResultColumn, ResultFact};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum OutputMode {
    Materialized,
    Factorized,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OutputStats {
    pub(crate) logical_facts_represented: usize,
    pub(crate) materialized_facts: usize,
    pub(crate) duplicate_witnesses_suppressed: usize,
    pub(crate) expansions_avoided: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Binding {
    slots: Vec<Option<Vec<u8>>>,
}

impl Binding {
    pub(super) fn new(variable_count: usize) -> Self {
        Self {
            slots: vec![None; variable_count],
        }
    }

    pub(super) fn value(&self, variable: usize) -> Option<&[u8]> {
        self.slots.get(variable)?.as_deref()
    }

    pub(super) fn undo_mark(undo: &[BindingUndo]) -> usize {
        undo.len()
    }

    pub(super) fn undo_to(&mut self, undo: &mut Vec<BindingUndo>, mark: usize) {
        while undo.len() > mark {
            let Some(entry) = undo.pop() else { break };
            if let Some(slot) = self.slots.get_mut(entry.variable) {
                *slot = None;
            }
        }
    }

    pub(super) fn extend_from_tuple(
        &mut self,
        vars: &[usize],
        tuple: EncodedTupleRef<'_>,
        query: &NormalizedQuery,
        undo: &mut Vec<BindingUndo>,
    ) -> Result<BindingExtend> {
        let mut offset = 0;
        let mut writes = 0;
        let tuple = tuple.bytes();
        for variable in vars {
            let width = query.variables[*variable].value_type.encoded_width();
            let Some(bytes) = tuple.get(offset..offset + width) else {
                return Err(Error::corrupt("cover tuple width is too short"));
            };
            let slot = self
                .slots
                .get_mut(*variable)
                .ok_or_else(|| Error::corrupt(format!("missing binding slot {variable}")))?;
            match slot.as_deref() {
                Some(existing) if existing != bytes => {
                    return Ok(BindingExtend {
                        accepted: false,
                        writes,
                        conflicts: 1,
                    });
                }
                Some(_) => {}
                None => {
                    *slot = Some(bytes.to_vec());
                    undo.push(BindingUndo {
                        variable: *variable,
                    });
                    writes += 1;
                }
            }
            offset += width;
        }
        if offset != tuple.len() {
            return Err(Error::corrupt("cover tuple width has trailing bytes"));
        }
        Ok(BindingExtend {
            accepted: true,
            writes,
            conflicts: 0,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BindingUndo {
    variable: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct BindingExtend {
    pub(super) accepted: bool,
    pub(super) writes: u64,
    pub(super) conflicts: u64,
}

pub(super) trait BindingSink {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()>;
}

pub(super) struct ProjectionSink<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
    facts: BTreeSet<ResultFact>,
    stats: OutputStats,
}

impl<'txn, 'env> ProjectionSink<'txn, 'env> {
    pub(super) fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self {
            txn,
            facts: BTreeSet::new(),
            stats: OutputStats::default(),
        }
    }

    pub(super) fn finish(self, query: &NormalizedQuery) -> Result<QueryResultSet> {
        Ok(QueryResultSet::new(
            result_columns(query)?,
            self.facts.into_iter().collect(),
        ))
    }

    #[allow(dead_code)]
    pub(super) fn finish_with_stats(
        mut self,
        query: &NormalizedQuery,
    ) -> Result<(QueryResultSet, OutputStats)> {
        self.stats.materialized_facts = self.facts.len();
        let columns = result_columns(query)?;
        let facts = self.facts.into_iter().collect();
        Ok((QueryResultSet::new(columns, facts), self.stats))
    }
}

impl BindingSink for ProjectionSink<'_, '_> {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()> {
        let mut fact = Vec::with_capacity(query.find.len());
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { variable } => {
                    let value_type = &query.variables[*variable].value_type;
                    let bytes = binding.value(*variable).ok_or_else(|| {
                        Error::corrupt(format!("projection variable {variable} is unbound"))
                    })?;
                    fact.push(storage_v5::decode_value(self.txn, value_type, bytes)?);
                }
            }
        }
        self.stats.logical_facts_represented += 1;
        if !self.facts.insert(fact) {
            self.stats.duplicate_witnesses_suppressed += 1;
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub(super) struct FactorizedProjectionSink<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
    encoded_facts: BTreeSet<Vec<u8>>,
    stats: OutputStats,
}

#[allow(dead_code)]
impl<'txn, 'env> FactorizedProjectionSink<'txn, 'env> {
    pub(super) fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self {
            txn,
            encoded_facts: BTreeSet::new(),
            stats: OutputStats::default(),
        }
    }

    pub(super) fn finish(
        mut self,
        query: &NormalizedQuery,
    ) -> Result<(QueryResultSet, OutputStats)> {
        self.stats.materialized_facts = self.encoded_facts.len();
        let facts = self
            .encoded_facts
            .iter()
            .map(|fact| decode_encoded_projection(self.txn, query, fact))
            .collect::<Result<Vec<_>>>()?;
        let stats = self.stats;
        Ok((QueryResultSet::new(result_columns(query)?, facts), stats))
    }
}

impl BindingSink for FactorizedProjectionSink<'_, '_> {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()> {
        let encoded = encoded_projection(query, binding)?;
        self.stats.logical_facts_represented += 1;
        if !self.encoded_facts.insert(encoded) {
            self.stats.duplicate_witnesses_suppressed += 1;
            self.stats.expansions_avoided += 1;
        }
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

#[allow(dead_code)]
fn encoded_projection(query: &NormalizedQuery, binding: &Binding) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for term in &query.find {
        match term {
            TypedFindTerm::Variable { variable } => {
                let bytes = binding.value(*variable).ok_or_else(|| {
                    Error::corrupt(format!("projection variable {variable} is unbound"))
                })?;
                out.extend_from_slice(bytes);
            }
        }
    }
    Ok(out)
}

#[allow(dead_code)]
fn decode_encoded_projection(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    encoded: &[u8],
) -> Result<ResultFact> {
    let mut offset = 0;
    let mut fact = Vec::with_capacity(query.find.len());
    for term in &query.find {
        match term {
            TypedFindTerm::Variable { variable } => {
                let value_type = &query.variables[*variable].value_type;
                let width = value_type.encoded_width();
                let Some(bytes) = encoded.get(offset..offset + width) else {
                    return Err(Error::corrupt("encoded projection is too short"));
                };
                fact.push(storage_v5::decode_value(txn, value_type, bytes)?);
                offset += width;
            }
        }
    }
    if offset != encoded.len() {
        return Err(Error::corrupt("encoded projection has trailing bytes"));
    }
    Ok(fact)
}
