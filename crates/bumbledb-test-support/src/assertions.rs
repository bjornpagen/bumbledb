//! Assertion helpers and invariant scanner.

use std::collections::BTreeSet;

use bumbledb_core::query_builder::QueryBuilder;
use bumbledb_lmdb::{Environment, InputBindings, InternalError, Result, StorageSchema, Value};

/// Sorts query result facts deterministically.
pub fn sorted_facts(mut facts: Vec<Vec<Value>>) -> Vec<Vec<Value>> {
    facts.sort();
    facts
}

/// Asserts two query outputs are equal under set semantics.
pub fn assert_same_facts(actual: Vec<Vec<Value>>, expected: Vec<Vec<Value>>) {
    assert_eq!(fact_set("actual", actual), fact_set("expected", expected));
}

fn fact_set(label: &str, facts: Vec<Vec<Value>>) -> BTreeSet<Vec<Value>> {
    let fact_count = facts.len();
    let set = facts.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(
        set.len(),
        fact_count,
        "{label} contains duplicate facts under set semantics"
    );
    set
}

/// Scans public current access paths and verifies visible invariants.
pub fn assert_invariants(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.read(|txn| {
        let diagnostics = env.storage_diagnostics(schema)?;
        for relation in &schema.descriptor().relations {
            let query = relation_projection_query(schema, relation)?;
            let primary_facts = txn
                .execute_query(schema, &query, &InputBindings::new())?
                .result
                .facts;
            let relation_diag = diagnostics
                .relations
                .iter()
                .find(|diag| diag.relation == relation.name)
                .ok_or_else(|| {
                    bumbledb_lmdb::Error::Internal(InternalError::Invariant {
                        message: format!("missing diagnostics for relation {}", relation.name),
                    })
                })?;
            assert_eq!(
                relation_diag.fact_count as usize,
                primary_facts.len(),
                "fact count drift for {}",
                relation.name
            );

            for index_diag in &relation_diag.indexes {
                assert_eq!(
                    index_diag.entry_count as usize,
                    primary_facts.len(),
                    "index stat drift for {}.{}",
                    relation.name,
                    index_diag.index
                );
            }
        }
        Ok(())
    })
}

fn relation_projection_query(
    schema: &StorageSchema,
    relation: &bumbledb_core::schema::RelationDescriptor,
) -> Result<bumbledb_core::query_ir::TypedQuery> {
    let mut builder = QueryBuilder::new(schema.descriptor());
    let mut atom = builder
        .rel(&relation.name)
        .map_err(|error| internal_error(format!("query build error: {error}")))?;
    for field in &relation.fields {
        atom = atom
            .var(&field.name, &field.name)
            .map_err(|error| internal_error(format!("query build error: {error}")))?;
    }
    atom.done();
    for field in &relation.fields {
        builder
            .find_var(&field.name)
            .map_err(|error| internal_error(format!("query build error: {error}")))?;
    }
    builder
        .finish()
        .map_err(|error| internal_error(format!("query build error: {error}")))
}

fn internal_error(message: String) -> bumbledb_lmdb::Error {
    bumbledb_lmdb::Error::Internal(InternalError::Invariant { message })
}

/// Executes query against LMDB and returns sorted facts.
pub fn execute_sorted_facts(
    env: &Environment,
    schema: &StorageSchema,
    query: &bumbledb_core::query_ir::TypedQuery,
    inputs: &InputBindings,
) -> Result<Vec<Vec<Value>>> {
    Ok(sorted_facts(
        env.read(|txn| txn.execute_query(schema, query, inputs))?
            .result
            .facts,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_same_facts_ignores_order() {
        assert_same_facts(
            vec![vec![Value::U64(2)], vec![Value::U64(1)]],
            vec![vec![Value::U64(1)], vec![Value::U64(2)]],
        );
    }

    #[test]
    #[should_panic(expected = "actual contains duplicate facts under set semantics")]
    fn assert_same_facts_rejects_duplicate_actual_facts() {
        assert_same_facts(
            vec![vec![Value::U64(1)], vec![Value::U64(1)]],
            vec![vec![Value::U64(1)]],
        );
    }
}
