//! Assertion helpers and invariant scanner.

use std::collections::BTreeSet;

use bumbledb_lmdb::{
    Environment, Fact, InputBindings, InternalError, Result, StorageSchema, Value,
};

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
            let primary_facts = txn
                .scan_relation(schema, &relation.name)?
                .map(|item| item.map(|item| item.fact))
                .collect::<Result<Vec<_>>>()?;
            let primary_set = primary_facts.iter().cloned().collect::<BTreeSet<Fact>>();
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
            assert_eq!(
                txn.canonical_fact_count(schema, &relation.name)?,
                primary_facts.len(),
                "canonical fact count drift for {}",
                relation.name
            );

            for path in schema.access_paths(&relation.name)? {
                let facts = txn
                    .scan_prefix(
                        schema,
                        &relation.name,
                        &path.index_name,
                        &bumbledb_lmdb::FieldValues::new(
                            &relation.name,
                            std::iter::empty::<(&str, Value)>(),
                        ),
                    )?
                    .map(|item| item.map(|item| item.fact))
                    .collect::<Result<Vec<_>>>()?;
                let fact_set = facts.iter().cloned().collect::<BTreeSet<Fact>>();
                assert_eq!(
                    primary_set, fact_set,
                    "index {}.{} does not decode to primary facts",
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
                    facts.len(),
                    "index stat drift for {}.{}",
                    relation.name,
                    path.index_name
                );
            }
        }
        Ok(())
    })
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
