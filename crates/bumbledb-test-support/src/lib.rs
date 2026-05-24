#![allow(clippy::result_large_err)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::query_ir::{
    ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFieldBinding, TypedFindTerm,
    TypedInput, TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{
    EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType,
};
use bumbledb_lmdb::{
    Environment, Fact, InputBindings, QueryResultSet, Result, StorageSchema, Value,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "bumbledb-test-support-{name}-{}-{id}",
        std::process::id()
    ));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

pub fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "TestSupport",
        vec![
            pair_relation("R"),
            pair_relation("S"),
            pair_relation("T"),
            pair_relation("Edge"),
            RelationDescriptor::new(
                "Mixed",
                vec![
                    FieldDescriptor::generated_serial("id", "MixedId", "Mixed"),
                    FieldDescriptor::new("flag", ValueType::Bool),
                    FieldDescriptor::new("u", ValueType::U64),
                    FieldDescriptor::new("i", ValueType::I64),
                    FieldDescriptor::new(
                        "kind",
                        ValueType::Enum {
                            name: "Kind".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("name", ValueType::String),
                    FieldDescriptor::new("blob", ValueType::Bytes),
                ],
            ),
        ],
    )
    .with_enum(EnumDescriptor::codes("Kind", [1, 2, 3]))
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

pub fn insert(
    env: &Environment,
    schema: &StorageSchema,
    facts: impl IntoIterator<Item = Fact>,
) -> Result<()> {
    env.write(|txn| {
        for fact in facts {
            txn.insert(schema, fact)?;
        }
        Ok::<(), bumbledb_lmdb::Error>(())
    })
}

pub fn pair(relation: &str, left: u64, right: u64) -> Fact {
    Fact::new(
        relation,
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}

pub fn mixed(id: u64, flag: bool, u: u64, i: i64, kind: u8, name: &str, blob: &[u8]) -> Fact {
    Fact::new(
        "Mixed",
        [
            ("id", Value::Serial(id)),
            ("flag", Value::Bool(flag)),
            ("u", Value::U64(u)),
            ("i", Value::I64(i)),
            ("kind", Value::Enum(kind)),
            ("name", Value::String(name.to_owned())),
            ("blob", Value::Bytes(blob.to_vec())),
        ],
    )
}

pub fn execute(
    env: &Environment,
    schema: &StorageSchema,
    query: &TypedQuery,
) -> Result<QueryResultSet> {
    env.read(|txn| txn.execute_query(schema, query, &InputBindings::new()))
}

pub fn execute_inputs(
    env: &Environment,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
) -> Result<QueryResultSet> {
    env.read(|txn| txn.execute_query(schema, query, inputs))
}

pub fn binary_join_query(left_rel: &str, right_rel: &str, find: &[usize]) -> TypedQuery {
    typed_query(
        &["x", "a", "b"],
        find,
        vec![
            pair_atom(0, left_rel, [(0, "left", 0), (1, "right", 1)]),
            pair_atom(1, right_rel, [(0, "left", 0), (1, "right", 2)]),
        ],
        Vec::new(),
        Vec::new(),
    )
}

pub fn clover_query(find: &[usize]) -> TypedQuery {
    typed_query(
        &["x", "a", "b", "c"],
        find,
        vec![
            pair_atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            pair_atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
            pair_atom(2, "T", [(0, "left", 0), (1, "right", 3)]),
        ],
        Vec::new(),
        Vec::new(),
    )
}

pub fn triangle_query(find: &[usize]) -> TypedQuery {
    typed_query(
        &["x", "y", "z"],
        find,
        vec![
            pair_atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            pair_atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
            pair_atom(2, "T", [(0, "left", 2), (1, "right", 0)]),
        ],
        Vec::new(),
        Vec::new(),
    )
}

pub fn typed_query(
    variable_names: &[&str],
    find: &[usize],
    atoms: Vec<TypedRelationAtom>,
    comparisons: Vec<TypedComparison>,
    inputs: Vec<TypedInput>,
) -> TypedQuery {
    TypedQuery {
        variables: variable_names
            .iter()
            .enumerate()
            .map(|(id, name)| TypedVariable {
                id,
                name: (*name).to_owned(),
                value_type: ValueType::U64,
            })
            .collect(),
        inputs,
        find: find
            .iter()
            .copied()
            .map(|variable| TypedFindTerm::Variable { variable })
            .collect(),
        clauses: atoms
            .into_iter()
            .map(TypedClause::Relation)
            .chain(comparisons.into_iter().map(TypedClause::Comparison))
            .collect(),
    }
}

pub fn pair_atom<const N: usize>(
    relation_id: usize,
    relation: &str,
    fields: [(usize, &str, usize); N],
) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id,
        relation: relation.to_owned(),
        fields: fields
            .into_iter()
            .map(|(field_id, field, variable)| field_var(field_id, field, variable))
            .collect(),
    }
}

pub fn field_var(field_id: usize, field: &str, variable: usize) -> TypedFieldBinding {
    TypedFieldBinding {
        field_id,
        field: field.to_owned(),
        value_type: ValueType::U64,
        term: TypedTerm::Variable(variable),
    }
}

pub fn comparison(left: usize, op: ComparisonOperator, right: TypedOperand) -> TypedComparison {
    TypedComparison {
        left: TypedOperand::Variable(left),
        operator: op,
        right,
        value_type: ValueType::U64,
    }
}

pub fn int_lit(value: i128) -> TypedOperand {
    TypedOperand::Literal(TypedLiteral {
        literal: Literal::Integer(value),
        value_type: ValueType::U64,
    })
}

pub fn rows<const N: usize>(rows: impl IntoIterator<Item = [u64; N]>) -> Vec<Vec<Value>> {
    rows.into_iter()
        .map(|row| row.into_iter().map(Value::U64).collect())
        .collect()
}

pub fn distinct(mut rows: Vec<Vec<Value>>) -> Vec<Vec<Value>> {
    let set: BTreeSet<_> = rows.drain(..).collect();
    set.into_iter().collect()
}
