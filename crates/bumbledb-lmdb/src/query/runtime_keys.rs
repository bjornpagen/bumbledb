use crate::query::free_join::FjSubatom;
use crate::query::model::NormalizedQuery;
use crate::query::sink::Binding;
use crate::tuple::{EncodedTuple, TupleError, TupleField, TupleSchema};
use crate::{Error, Result};

pub(super) fn key_from_binding(
    query: &NormalizedQuery,
    binding: &Binding,
    subatom: &FjSubatom,
) -> Result<EncodedTuple> {
    tuple_schema_for_vars(query, &subatom.vars)?
        .tuple_from_bindings(&binding.values)
        .map_err(tuple_error)
}

pub(super) fn key_from_binding_by_bound_widths(
    binding: &Binding,
    subatom: &FjSubatom,
) -> Result<EncodedTuple> {
    let fields = subatom
        .vars
        .iter()
        .map(|variable| {
            let width = binding
                .values
                .get(variable)
                .map(Vec::len)
                .ok_or_else(|| Error::corrupt(format!("missing variable {variable}")))?;
            TupleField::new(*variable, None, width).map_err(tuple_error)
        })
        .collect::<Result<Vec<_>>>()?;
    TupleSchema::new(fields)
        .tuple_from_bindings(&binding.values)
        .map_err(tuple_error)
}

fn tuple_schema_for_vars(query: &NormalizedQuery, vars: &[usize]) -> Result<TupleSchema> {
    let fields = vars
        .iter()
        .map(|variable| {
            let value_type = &query
                .variables
                .get(*variable)
                .ok_or_else(|| Error::invalid_query(format!("unknown variable {variable}")))?
                .value_type;
            TupleField::new(*variable, None, value_type.encoded_width()).map_err(tuple_error)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TupleSchema::new(fields))
}

fn tuple_error(error: TupleError) -> Error {
    Error::corrupt(error.to_string())
}
