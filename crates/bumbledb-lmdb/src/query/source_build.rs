use std::collections::BTreeMap;

use crate::base_image::field_scope_for_plan;
use crate::colt::{ColtSource, tuple_schemas_for_atom};
use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::query::predicate::{self, PredicateMode};
use crate::query::trace::QueryTrace;
use crate::{Error, ReadTxn, Result, StorageSchema};

pub(super) fn build_sources(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    trace: &mut QueryTrace,
) -> Result<BTreeMap<AtomOccurrenceId, ColtSource>> {
    let mut scopes = field_scope_for_plan(plan);
    let mut sources = BTreeMap::new();
    for atom in &query.atoms {
        let filters = predicate::source_filters_for_atom_with_trace(
            txn,
            schema.descriptor(),
            query,
            atom,
            inputs,
            predicate_mode,
            trace,
        )?;
        scopes.entry(atom.id).or_default().extend(
            filters
                .iter()
                .filter_map(crate::colt::SourceFilter::field_id),
        );
        let field_ids = scopes.get(&atom.id).into_iter().flatten().copied();
        let image = txn.relation_base_image_with_trace(schema, &atom.relation, field_ids, trace)?;
        let tuple_schemas = tuple_schemas_for_atom(query, plan, atom.id);
        if tuple_schemas.is_empty() {
            return Err(Error::invalid_query(format!(
                "atom occurrence {:?} has no Free Join source schema",
                atom.id
            )));
        }
        sources.insert(
            atom.id,
            ColtSource::new_filtered_traced(atom.id, image, tuple_schemas, filters, trace),
        );
    }
    Ok(sources)
}
