#![no_main]

use bumbledb_core::query_ir::{TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm, TypedVariable};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb_lmdb::{Environment, Fact, InputBindings, StorageSchema, Value};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let path = std::env::temp_dir().join(format!("bumbledb-fuzz-query-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    let schema = match StorageSchema::new(schema(), 511) {
        Ok(schema) => schema,
        Err(_) => return,
    };
    let env = match Environment::open_with_schema(&path, &schema) {
        Ok(env) => env,
        Err(_) => return,
    };
    let _ = env.write(|txn| {
        txn.insert(&schema, pair("R", data[0] as u64, data[1] as u64))?;
        txn.insert(&schema, pair("S", data[0] as u64, data[2] as u64))?;
        Ok::<(), bumbledb_lmdb::Error>(())
    });
    let query = query();
    let result = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()));
    if let Ok(result) = result
        && result.facts != vec![vec![Value::U64(data[0] as u64)]]
    {
        std::process::abort();
    }
    let _ = std::fs::remove_dir_all(path);
});

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "FuzzQuery",
        vec![pair_relation("R"), pair_relation("S")],
    )
}

fn pair_relation(name: &str) -> RelationDescriptor {
    RelationDescriptor::new(
        name,
        vec![
            FieldDescriptor::new("left", ValueType::U64),
            FieldDescriptor::new("right", ValueType::U64),
        ],
    )
}

fn pair(relation: &str, left: u64, right: u64) -> Fact {
    Fact::new(
        relation,
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}

fn query() -> TypedQuery {
    TypedQuery {
        variables: (0..3)
            .map(|id| TypedVariable { id, name: format!("v{id}"), value_type: ValueType::U64 })
            .collect(),
        inputs: Vec::new(),
        find: vec![TypedFindTerm::Variable { variable: 0 }],
        clauses: vec![
            TypedClause::Relation(atom(0, "R", [(0, "left", 0), (1, "right", 1)])),
            TypedClause::Relation(atom(1, "S", [(0, "left", 0), (1, "right", 2)])),
        ],
    }
}

fn atom<const N: usize>(relation_id: usize, relation: &str, fields: [(usize, &str, usize); N]) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id,
        relation: relation.to_owned(),
        fields: fields
            .into_iter()
            .map(|(field_id, field, variable)| TypedFieldBinding { field_id, field: field.to_owned(), value_type: ValueType::U64, term: TypedTerm::Variable(variable) })
            .collect(),
    }
}
