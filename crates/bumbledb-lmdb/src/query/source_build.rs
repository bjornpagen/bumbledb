use std::collections::BTreeMap;
use std::rc::Rc;

use crate::base_image::{RelationBaseImage, RelationStats, field_scope_for_plan};
use crate::colt::tuple_schemas_for_atom;
use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::{AtomOccurrence, NormalizedQuery, SourcePredicate};
use crate::query::predicate::{self, PredicateMode};
use crate::query::runtime_frame::SourceStore;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::{Error, ReadTxn, Result, StorageSchema};

pub(super) fn build_sources(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    trace: &mut QueryTrace,
) -> Result<SourceStore> {
    let mut scopes = field_scope_for_plan(plan);
    let mut sources = SourceStore::with_atom_count(query.atoms.len());
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
        let impossible = filters
            .iter()
            .any(|filter| matches!(filter, crate::colt::SourceFilter::False));
        let filter_label = if trace.is_enabled() {
            source_filter_label(atom)
        } else {
            String::new()
        };
        scopes.entry(atom.id).or_default().extend(
            filters
                .iter()
                .filter_map(crate::colt::SourceFilter::field_id),
        );
        let tuple_schemas = tuple_schemas_for_atom(query, plan, atom.id);
        if tuple_schemas.is_empty() {
            return Err(Error::invalid_query(format!(
                "atom occurrence {:?} has no Free Join source schema",
                atom.id
            )));
        }
        if impossible {
            let span = crate::query_trace_span!(
                trace,
                TracePhase::EmptySourceShortCircuit,
                "relation={} atom={:?} filters={}",
                atom.relation,
                atom.id,
                filter_label
            );
            if let Some(span) = span {
                trace.finish_span(
                    span,
                    TraceCounters {
                        empty_source_short_circuits: 1,
                        source_filter_survivors: 0,
                        ..TraceCounters::default()
                    },
                );
            }
            sources.insert_filtered_traced_labeled(
                atom.id,
                Rc::new(empty_image(atom)),
                tuple_schemas,
                filters,
                filter_label,
                trace,
            );
            continue;
        }
        let field_ids = scopes
            .get(&atom.id)
            .into_iter()
            .flat_map(|scope| scope.iter());
        let image = txn.relation_base_image_with_trace(schema, &atom.relation, field_ids, trace)?;
        sources.insert_filtered_traced_labeled(
            atom.id,
            image,
            tuple_schemas,
            filters,
            filter_label,
            trace,
        );
    }
    Ok(sources)
}

fn empty_image(atom: &AtomOccurrence) -> RelationBaseImage {
    RelationBaseImage {
        relation_id: atom.relation_id as u32,
        name: atom.relation.clone(),
        row_handles: Vec::new(),
        columns: BTreeMap::new(),
        stats: RelationStats { row_count: 0 },
    }
}

fn source_filter_label(atom: &AtomOccurrence) -> String {
    atom.source_predicates
        .iter()
        .map(|predicate| match predicate {
            SourcePredicate::InputEq { field_id, .. } => {
                format!("{} = <input>", atom.fields[*field_id].field)
            }
            SourcePredicate::LiteralEq { field_id, literal } => {
                format!(
                    "{} = {}",
                    atom.fields[*field_id].field,
                    literal_label(literal)
                )
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn literal_label(literal: &bumbledb_core::query_ir::TypedLiteral) -> String {
    match &literal.literal {
        bumbledb_core::query_ir::Literal::Bool(value) => value.to_string(),
        bumbledb_core::query_ir::Literal::Integer(value) => value.to_string(),
        bumbledb_core::query_ir::Literal::String(value) => format!("'{value}'"),
    }
}
