use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::tuple::{TupleField, TupleSchema};

pub(crate) fn tuple_schemas_for_atom(
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    atom: AtomOccurrenceId,
) -> Vec<TupleSchema> {
    let occurrence = &query.atoms[atom.0];
    let mut schemas = Vec::new();
    for node in &plan.nodes {
        for subatom in &node.subatoms {
            if subatom.atom == atom {
                let fields = subatom
                    .vars
                    .iter()
                    .zip(&subatom.field_ids)
                    .filter_map(|(variable, field_id)| {
                        let Ok(field) = TupleField::new(
                            *variable,
                            Some(*field_id),
                            occurrence.fields[*field_id].value_type.encoded_width(),
                        ) else {
                            return None;
                        };
                        Some(field)
                    })
                    .collect();
                schemas.push(TupleSchema::new(fields));
            }
        }
    }
    schemas
}
