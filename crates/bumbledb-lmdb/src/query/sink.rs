use bumbledb_core::query_ir::TypedFindTerm;

use crate::query::model::NormalizedQuery;
use crate::query::projection_dedup::{ProjectionDedup, ProjectionScratch};
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
    pub(crate) encoded_facts_inserted: usize,
    pub(crate) materialized_facts: usize,
    pub(crate) duplicate_witnesses_suppressed: usize,
    pub(crate) expansions_avoided: usize,
    pub(crate) decoded_values: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Binding {
    slots: Vec<Option<EncodedValueSlot>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EncodedValueSlot {
    len: u8,
    bytes: [u8; 16],
}

impl EncodedValueSlot {
    fn new(value: &[u8]) -> Result<Self> {
        let len = value.len();
        if len > 16 {
            return Err(Error::corrupt("encoded binding value is too wide"));
        }
        let mut bytes = [0; 16];
        bytes[..len].copy_from_slice(value);
        Ok(Self {
            len: len as u8,
            bytes,
        })
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

impl Binding {
    pub(super) fn new(variable_count: usize) -> Self {
        Self {
            slots: vec![None; variable_count],
        }
    }

    pub(super) fn value(&self, variable: usize) -> Option<&[u8]> {
        self.slots
            .get(variable)?
            .as_ref()
            .map(EncodedValueSlot::as_slice)
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
            match slot.as_ref().map(EncodedValueSlot::as_slice) {
                Some(existing) if existing != bytes => {
                    return Ok(BindingExtend {
                        accepted: false,
                        writes,
                        conflicts: 1,
                    });
                }
                Some(_) => {}
                None => {
                    *slot = Some(EncodedValueSlot::new(bytes)?);
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
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<SinkConsumeStats>;

    fn skip_seen_projection(
        &mut self,
        _query: &NormalizedQuery,
        _binding: &Binding,
    ) -> Result<bool> {
        Ok(false)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SinkConsumeStats {
    pub(super) inserted: bool,
}

pub(super) struct ProjectionSink<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
    encoded_facts: ProjectionDedup,
    scratch: ProjectionScratch,
    stats: OutputStats,
}

impl<'txn, 'env> ProjectionSink<'txn, 'env> {
    pub(super) fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self {
            txn,
            encoded_facts: ProjectionDedup::default(),
            scratch: ProjectionScratch::default(),
            stats: OutputStats::default(),
        }
    }

    pub(super) fn finish(self, query: &NormalizedQuery) -> Result<QueryResultSet> {
        self.finish_with_stats(query).map(|(result, _stats)| result)
    }

    #[allow(dead_code)]
    pub(super) fn finish_with_stats(
        mut self,
        query: &NormalizedQuery,
    ) -> Result<(QueryResultSet, OutputStats)> {
        self.stats.materialized_facts = self.encoded_facts.len();
        let columns = result_columns(query)?;
        let mut facts = self
            .encoded_facts
            .iter()
            .map(|fact| decode_encoded_projection(self.txn, query, fact))
            .collect::<Result<Vec<_>>>()?;
        facts.sort();
        self.stats.decoded_values = facts.len() * query.find.len();
        Ok((QueryResultSet::from_canonical(columns, facts), self.stats))
    }
}

impl BindingSink for ProjectionSink<'_, '_> {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<SinkConsumeStats> {
        let encoded = self
            .scratch
            .encoded_projection(query, binding)?
            .ok_or_else(|| Error::corrupt("projection variable is unbound"))?;
        self.stats.logical_facts_represented += 1;
        let inserted = self.encoded_facts.insert(encoded);
        if inserted {
            self.stats.encoded_facts_inserted += 1;
        } else {
            self.stats.duplicate_witnesses_suppressed += 1;
        }
        Ok(SinkConsumeStats { inserted })
    }

    fn skip_seen_projection(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<bool> {
        let Some(encoded) = self.scratch.encoded_projection(query, binding)? else {
            return Ok(false);
        };
        if self.encoded_facts.contains(encoded) {
            self.stats.expansions_avoided += 1;
            return Ok(true);
        }
        Ok(false)
    }
}

#[allow(dead_code)]
pub(super) struct FactorizedProjectionSink<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
    encoded_facts: ProjectionDedup,
    scratch: ProjectionScratch,
    stats: OutputStats,
}

#[allow(dead_code)]
impl<'txn, 'env> FactorizedProjectionSink<'txn, 'env> {
    pub(super) fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self {
            txn,
            encoded_facts: ProjectionDedup::default(),
            scratch: ProjectionScratch::default(),
            stats: OutputStats::default(),
        }
    }

    pub(super) fn finish(
        mut self,
        query: &NormalizedQuery,
    ) -> Result<(QueryResultSet, OutputStats)> {
        self.stats.materialized_facts = self.encoded_facts.len();
        let mut facts = self
            .encoded_facts
            .iter()
            .map(|fact| decode_encoded_projection(self.txn, query, fact))
            .collect::<Result<Vec<_>>>()?;
        facts.sort();
        self.stats.decoded_values = facts.len() * query.find.len();
        let stats = self.stats;
        Ok((
            QueryResultSet::from_canonical(result_columns(query)?, facts),
            stats,
        ))
    }
}

impl BindingSink for FactorizedProjectionSink<'_, '_> {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<SinkConsumeStats> {
        let encoded = self
            .scratch
            .encoded_projection(query, binding)?
            .ok_or_else(|| Error::corrupt("projection variable is unbound"))?;
        self.stats.logical_facts_represented += 1;
        let inserted = self.encoded_facts.insert(encoded);
        if inserted {
            self.stats.encoded_facts_inserted += 1;
        } else {
            self.stats.duplicate_witnesses_suppressed += 1;
            self.stats.expansions_avoided += 1;
        }
        Ok(SinkConsumeStats { inserted })
    }

    fn skip_seen_projection(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<bool> {
        let Some(encoded) = self.scratch.encoded_projection(query, binding)? else {
            return Ok(false);
        };
        if self.encoded_facts.contains(encoded) {
            self.stats.expansions_avoided += 1;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct CountingSink {
    pub(super) count: usize,
}

#[cfg(test)]
impl BindingSink for CountingSink {
    fn consume(
        &mut self,
        _query: &NormalizedQuery,
        _binding: &Binding,
    ) -> Result<SinkConsumeStats> {
        self.count += 1;
        Ok(SinkConsumeStats { inserted: true })
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
