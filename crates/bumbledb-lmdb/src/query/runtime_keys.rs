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
    let schema = tuple_schema_for_vars(query, &subatom.vars)?;
    let bytes = bytes_from_binding(binding, &subatom.vars, &schema)?;
    EncodedTuple::new(&schema, bytes).map_err(tuple_error)
}

pub(super) fn key_from_binding_by_bound_widths(
    binding: &Binding,
    subatom: &FjSubatom,
) -> Result<EncodedTuple> {
    let mut bytes = Vec::new();
    let mut fields = Vec::new();
    for variable in &subatom.vars {
        let value = binding
            .value(*variable)
            .ok_or_else(|| Error::corrupt(format!("missing variable {variable}")))?;
        fields.push(TupleField::new(*variable, None, value.len()).map_err(tuple_error)?);
        bytes.extend_from_slice(value);
    }
    let schema = TupleSchema::new(fields);
    EncodedTuple::new(&schema, bytes).map_err(tuple_error)
}

fn bytes_from_binding(binding: &Binding, vars: &[usize], schema: &TupleSchema) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(schema.encoded_width());
    for field in &schema.fields {
        let value = binding
            .value(field.variable)
            .ok_or_else(|| Error::corrupt(format!("missing variable {}", field.variable)))?;
        if value.len() != field.width {
            return Err(Error::corrupt(format!(
                "binding width mismatch for variable {}",
                field.variable
            )));
        }
        bytes.extend_from_slice(value);
    }
    if schema.fields.len() != vars.len() {
        return Err(Error::corrupt("binding schema arity mismatch"));
    }
    Ok(bytes)
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
