//! Assertion helpers and invariant scanner.

use std::collections::BTreeSet;

use bumbledb_lmdb::{Environment, InputBindings, InternalError, Result, Row, StorageSchema, Value};

/// Sorts query result rows deterministically.
pub fn sorted_rows(mut rows: Vec<Vec<Value>>) -> Vec<Vec<Value>> {
    rows.sort();
    rows
}

/// Asserts two query outputs are equal under set semantics.
pub fn assert_same_rows(actual: Vec<Vec<Value>>, expected: Vec<Vec<Value>>) {
    assert_eq!(sorted_rows(actual), sorted_rows(expected));
}

/// Scans public current access paths and verifies visible invariants.
pub fn assert_invariants(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.read(|txn| {
        let diagnostics = env.storage_diagnostics(schema)?;
        for relation in &schema.descriptor().relations {
            let primary_rows = txn
                .scan_relation(schema, &relation.name)?
                .map(|item| item.map(|item| item.row))
                .collect::<Result<Vec<_>>>()?;
            let primary_set = primary_rows.iter().cloned().collect::<BTreeSet<Row>>();
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
                relation_diag.row_count as usize,
                primary_rows.len(),
                "row count drift for {}",
                relation.name
            );

            for path in schema.access_paths(&relation.name)? {
                let rows = txn
                    .scan_prefix(
                        schema,
                        &relation.name,
                        &path.index_name,
                        &bumbledb_lmdb::FieldValues::new(
                            &relation.name,
                            std::iter::empty::<(&str, Value)>(),
                        ),
                    )?
                    .map(|item| item.map(|item| item.row))
                    .collect::<Result<Vec<_>>>()?;
                let row_set = rows.iter().cloned().collect::<BTreeSet<Row>>();
                assert_eq!(
                    primary_set, row_set,
                    "index {}.{} does not decode to primary rows",
                    relation.name, path.index_name
                );
                let index_diag = relation_diag
                    .indexes
                    .iter()
                    .find(|diag| diag.index == path.index_name)
                    .ok_or_else(|| {
                        bumbledb_lmdb::Error::Internal(InternalError::Invariant {
                            message: format!(
                                "missing diagnostics for index {}.{}",
                                relation.name, path.index_name
                            ),
                        })
                    })?;
                assert_eq!(
                    index_diag.entry_count as usize,
                    rows.len(),
                    "index stat drift for {}.{}",
                    relation.name,
                    path.index_name
                );
            }
        }
        Ok(())
    })
}

/// Executes query against LMDB and returns sorted rows.
pub fn execute_sorted(
    env: &Environment,
    schema: &StorageSchema,
    query: &bumbledb_core::query_ir::TypedQuery,
    inputs: &InputBindings,
) -> Result<Vec<Vec<Value>>> {
    Ok(sorted_rows(
        env.read(|txn| txn.execute_query(schema, query, inputs))?
            .rows,
    ))
}
