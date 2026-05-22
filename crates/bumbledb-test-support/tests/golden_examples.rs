#![allow(clippy::result_large_err)]

use std::collections::BTreeSet;

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuilder};
use bumbledb_core::query_ir::ComparisonOperator;
use bumbledb_core::schema::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};
use bumbledb_lmdb::{DeleteOutcome, Environment, InputBindings, Row, StorageSchema, Value};
use bumbledb_test_support::assertions::assert_same_rows;
use bumbledb_test_support::golden::{GOLDEN_FAMILIES, GoldenFamily};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn golden_manifest_lists_all_required_families() {
    let names = GOLDEN_FAMILIES
        .iter()
        .map(|family| family.name())
        .collect::<BTreeSet<_>>();
    assert_eq!(GOLDEN_FAMILIES.len(), 7);
    assert!(names.contains(GoldenFamily::Ledger.name()));
    assert!(names.contains(GoldenFamily::Sailors.name()));
    assert!(names.contains(GoldenFamily::Joinstress.name()));
    assert!(names.contains(GoldenFamily::TpchSubset.name()));
    assert!(names.contains(GoldenFamily::ImdbJobSubset.name()));
    assert!(names.contains(GoldenFamily::LahmanSubset.name()));
    assert!(names.contains(GoldenFamily::LdbcSubset.name()));
}

#[test]
fn ledger_golden_preserves_set_projection_aggregate_and_restrict() -> TestResult {
    let schema = bumbledb_lmdb::benchmark::benchmark_schema();
    let rows = bumbledb_lmdb::benchmark::benchmark_rows(2);
    let (env, storage) = load(schema, rows)?;

    let duplicate = env.write(|txn| {
        txn.insert(
            &storage,
            Row::new(
                "Holder",
                [
                    ("id", Value::Serial(1)),
                    ("name", Value::String("holder-1".to_owned())),
                ],
            ),
        )
    })?;
    assert_eq!(duplicate, bumbledb_lmdb::InsertOutcome::AlreadyPresent);

    let absent = env.write(|txn| {
        txn.delete(
            &storage,
            Row::new(
                "PostingTag",
                [("posting", Value::Serial(99)), ("tag", Value::Enum(1))],
            ),
        )
    })?;
    assert_eq!(absent, DeleteOutcome::Absent);

    let restricted = env.write(|txn| {
        txn.delete(
            &storage,
            Row::new(
                "Holder",
                [
                    ("id", Value::Serial(1)),
                    ("name", Value::String("holder-1".to_owned())),
                ],
            ),
        )
    });
    assert!(restricted.is_err());

    let account_projection = QueryBuilder::new(storage.descriptor())
        .rel("Posting")?
        .var("account", "account")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .find_var("account")?
        .finish()?;
    assert_same_rows(
        execute(
            &env,
            &storage,
            &account_projection,
            inputs([("holder", Value::Serial(1))]),
        )?,
        vec![vec![Value::Serial(1)]],
    );

    let balances = QueryBuilder::new(storage.descriptor())
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("instrument", "instrument")?
        .var("amount", "amount")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .find_var("instrument")?
        .find_sum_over("amount", ["posting"])?
        .finish()?;
    assert_same_rows(
        execute(
            &env,
            &storage,
            &balances,
            inputs([("holder", Value::Serial(1))]),
        )?,
        vec![
            vec![Value::Serial(1), Value::Decimal(DecimalRaw(100))],
            vec![Value::Serial(2), Value::Decimal(DecimalRaw(200))],
            vec![Value::Serial(3), Value::Decimal(DecimalRaw(300))],
        ],
    );
    Ok(())
}

#[test]
fn sailors_golden_preserves_duplicate_witness_projection_and_deletes() -> TestResult {
    let (env, schema) = load(
        sailors_schema(),
        vec![
            sailor(1, 9),
            sailor(2, 5),
            boat(1, 1),
            boat(2, 1),
            reserve(1, 1, 10),
            reserve(1, 2, 20),
            reserve(2, 1, 30),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Reserve")?
        .var("sailor", "sailor")?
        .var("boat", "boat")?
        .done()
        .rel("Boat")?
        .var("id", "boat")?
        .input("color", "color")?
        .done()
        .find_var("sailor")?
        .finish()?;
    assert_same_rows(
        execute(&env, &schema, &query, inputs([("color", Value::Enum(1))]))?,
        vec![vec![Value::Serial(1)], vec![Value::Serial(2)]],
    );
    assert_eq!(
        env.write(|txn| txn.delete(&schema, reserve(1, 2, 20)))?,
        DeleteOutcome::Deleted
    );
    assert_eq!(
        env.write(|txn| txn.delete(&schema, reserve(1, 2, 20)))?,
        DeleteOutcome::Absent
    );
    Ok(())
}

#[test]
fn joinstress_golden_preserves_triangle_domains() -> TestResult {
    let (env, schema) = load(
        triangle_schema(),
        vec![
            edge_ab(1, 10),
            edge_ab(1, 11),
            edge_ac(1, 20),
            edge_bc(10, 20),
            edge_bc(11, 20),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("EdgeAB")?
        .var("a", "a")?
        .var("b", "b")?
        .done()
        .rel("EdgeAC")?
        .var("a", "a")?
        .var("c", "c")?
        .done()
        .rel("EdgeBC")?
        .var("b", "b")?
        .var("c", "c")?
        .done()
        .find_count_distinct("a")?
        .find_count_domain(["a", "b", "c"])?
        .finish()?;
    assert_same_rows(
        execute(&env, &schema, &query, InputBindings::new())?,
        vec![vec![Value::U64(1), Value::U64(2)]],
    );
    Ok(())
}

#[test]
fn tpch_golden_preserves_lineitem_revenue_domain() -> TestResult {
    let (env, schema) = load(
        tpch_schema(),
        vec![
            customer(1, 1),
            orders(1, 1),
            lineitem(1, 1, 100),
            lineitem(2, 1, 100),
            supplier(1, 2),
            supplier(2, 3),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Customer")?
        .var("id", "customer")?
        .input("nation", "nation")?
        .done()
        .rel("Orders")?
        .var("id", "order")?
        .var("customer", "customer")?
        .done()
        .rel("LineItem")?
        .var("id", "line")?
        .var("order", "order")?
        .var("extended_price", "price")?
        .done()
        .find_var("customer")?
        .find_sum_over("price", ["line"])?
        .finish()?;
    assert_same_rows(
        execute(&env, &schema, &query, inputs([("nation", Value::U64(1))]))?,
        vec![vec![Value::Serial(1), Value::Decimal(DecimalRaw(200))]],
    );
    Ok(())
}

#[test]
fn imdb_job_golden_preserves_title_count_and_static_empty() -> TestResult {
    let (env, schema) = load(
        imdb_schema(),
        vec![
            title(1, 2020),
            name(1),
            name(2),
            principal(1, 1, 1, 1),
            principal(1, 2, 1, 2),
        ],
    )?;
    let count = QueryBuilder::new(schema.descriptor())
        .rel("Title")?
        .var("id", "title")?
        .done()
        .rel("Principal")?
        .var("title", "title")?
        .var("name", "name")?
        .input("category", "category")?
        .done()
        .find_count_domain(["title"])?
        .finish()?;
    assert_same_rows(
        execute(
            &env,
            &schema,
            &count,
            inputs([("category", Value::Enum(1))]),
        )?,
        vec![vec![Value::U64(1)]],
    );

    let empty = QueryBuilder::new(schema.descriptor())
        .rel("Title")?
        .var("id", "title")?
        .var("year", "year")?
        .done()
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gt,
            OperandRef::integer(3000),
        )?
        .find_count_domain(["title"])?
        .finish()?;
    assert_same_rows(
        execute(&env, &schema, &empty, InputBindings::new())?,
        vec![vec![Value::U64(0)]],
    );
    Ok(())
}

#[test]
fn lahman_golden_preserves_compound_year_join() -> TestResult {
    let (env, schema) = load(
        lahman_schema(),
        vec![
            player(1),
            team(1, 2000),
            batting(1, 1, 2000, 7),
            salary(1, 1, 2000, 50),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Salary")?
        .var("player", "player")?
        .var("team", "team")?
        .input("year", "year")?
        .var("salary", "salary")?
        .done()
        .rel("Batting")?
        .var("player", "player")?
        .var("team", "team")?
        .input("year", "year")?
        .var("hits", "hits")?
        .done()
        .find_var("player")?
        .find_var("salary")?
        .find_var("hits")?
        .finish()?;
    assert_same_rows(
        execute(&env, &schema, &query, inputs([("year", Value::I64(2000))]))?,
        vec![vec![Value::Serial(1), Value::I64(50), Value::I64(7)]],
    );
    Ok(())
}

#[test]
fn ldbc_golden_preserves_two_hop_projection_set() -> TestResult {
    let (env, schema) = load(
        ldbc_schema(),
        vec![
            person(1),
            person(2),
            person(3),
            person(4),
            knows(1, 2),
            knows(1, 3),
            knows(2, 4),
            knows(3, 4),
        ],
    )?;
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Knows")?
        .input("person1", "person")?
        .var("person2", "friend1")?
        .done()
        .rel("Knows")?
        .var("person1", "friend1")?
        .var("person2", "friend2")?
        .done()
        .find_var("friend2")?
        .finish()?;
    assert_same_rows(
        execute(
            &env,
            &schema,
            &query,
            inputs([("person", Value::Serial(1))]),
        )?,
        vec![vec![Value::Serial(4)]],
    );
    Ok(())
}

fn load(
    schema: SchemaDescriptor,
    rows: Vec<Row>,
) -> Result<(Environment, StorageSchema), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let storage = StorageSchema::new(schema, env.max_key_size())?;
    env.bulk_load(&storage, rows)?;
    Ok((env, storage))
}

fn execute(
    env: &Environment,
    schema: &StorageSchema,
    query: &bumbledb_core::query_ir::TypedQuery,
    inputs: InputBindings,
) -> bumbledb_lmdb::Result<Vec<Vec<Value>>> {
    Ok(env
        .read(|txn| txn.execute_query(schema, query, &inputs))?
        .rows)
}

fn inputs(values: impl IntoIterator<Item = (&'static str, Value)>) -> InputBindings {
    InputBindings::from_values(values)
}

fn serial_type(name: &str, relation: &str) -> ValueType {
    ValueType::Serial {
        type_name: name.to_owned(),
        owning_relation: relation.to_owned(),
    }
}

fn serial_field(type_name: &str, name: &str, owner: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, serial_type(type_name, owner))
}

fn serial_id(type_name: &str, relation: &str) -> FieldDescriptor {
    serial_field(type_name, "id", relation)
}

fn sailors_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenSailorsDb",
        vec![
            RelationDescriptor::new(
                "Sailor",
                vec![
                    serial_id("SailorId", "Sailor"),
                    FieldDescriptor::new("rating", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Boat",
                vec![
                    serial_id("BoatId", "Boat"),
                    FieldDescriptor::new(
                        "color",
                        ValueType::Enum {
                            name: "Color".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Reserve",
                vec![
                    serial_field("SailorId", "sailor", "Sailor"),
                    serial_field("BoatId", "boat", "Boat"),
                    FieldDescriptor::new("day", ValueType::TimestampMicros),
                ],
            )
            .with_unique("sailor_boat_day", ["sailor", "boat", "day"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "sailor",
                ["sailor"],
                "Sailor",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "boat",
                ["boat"],
                "Boat",
                "id",
            )),
        ],
    )
    .with_enum(EnumDescriptor::codes("Color", [1, 2]))
}

fn triangle_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenTriangleDb",
        vec![
            RelationDescriptor::new(
                "EdgeAB",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                ],
            )
            .with_unique("ab", ["a", "b"]),
            RelationDescriptor::new(
                "EdgeAC",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("ac", ["a", "c"]),
            RelationDescriptor::new(
                "EdgeBC",
                vec![
                    FieldDescriptor::new("b", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("bc", ["b", "c"]),
        ],
    )
}

fn tpch_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenTpchDb",
        vec![
            RelationDescriptor::new(
                "Customer",
                vec![
                    serial_id("CustomerId", "Customer"),
                    FieldDescriptor::new("nation", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Supplier",
                vec![
                    serial_id("SupplierId", "Supplier"),
                    FieldDescriptor::new("nation", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Orders",
                vec![
                    serial_id("OrderId", "Orders"),
                    serial_field("CustomerId", "customer", "Customer"),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "customer",
                ["customer"],
                "Customer",
                "id",
            )),
            RelationDescriptor::new(
                "LineItem",
                vec![
                    serial_id("LineItemId", "LineItem"),
                    serial_field("OrderId", "order", "Orders"),
                    FieldDescriptor::new("extended_price", ValueType::Decimal { scale: 2 }),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "order",
                ["order"],
                "Orders",
                "id",
            )),
        ],
    )
}

fn imdb_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenImdbDb",
        vec![
            RelationDescriptor::new(
                "Title",
                vec![
                    serial_id("TitleId", "Title"),
                    FieldDescriptor::new("year", ValueType::I64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new("Name", vec![serial_id("NameId", "Name")])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Principal",
                vec![
                    serial_field("TitleId", "title", "Title"),
                    serial_field("NameId", "name", "Name"),
                    FieldDescriptor::new(
                        "category",
                        ValueType::Enum {
                            name: "Category".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("ordering", ValueType::U64),
                ],
            )
            .with_unique(
                "title_name_category_order",
                ["title", "name", "category", "ordering"],
            )
            .with_constraint(ConstraintDescriptor::foreign_key(
                "title",
                ["title"],
                "Title",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "name",
                ["name"],
                "Name",
                "id",
            )),
        ],
    )
    .with_enum(EnumDescriptor::codes("Category", [1, 2]))
}

fn lahman_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenLahmanDb",
        vec![
            RelationDescriptor::new("Player", vec![serial_id("PlayerId", "Player")])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Team",
                vec![
                    serial_id("TeamId", "Team"),
                    FieldDescriptor::new("year", ValueType::I64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Batting",
                vec![
                    serial_field("PlayerId", "player", "Player"),
                    serial_field("TeamId", "team", "Team"),
                    FieldDescriptor::new("year", ValueType::I64),
                    FieldDescriptor::new("hits", ValueType::I64),
                ],
            )
            .with_unique("player_team_year", ["player", "team", "year"]),
            RelationDescriptor::new(
                "Salary",
                vec![
                    serial_field("PlayerId", "player", "Player"),
                    serial_field("TeamId", "team", "Team"),
                    FieldDescriptor::new("year", ValueType::I64),
                    FieldDescriptor::new("salary", ValueType::I64),
                ],
            )
            .with_unique("player_team_year", ["player", "team", "year"]),
        ],
    )
}

fn ldbc_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenLdbcDb",
        vec![
            RelationDescriptor::new("Person", vec![serial_id("PersonId", "Person")])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Knows",
                vec![
                    serial_field("PersonId", "person1", "Person"),
                    serial_field("PersonId", "person2", "Person"),
                ],
            )
            .with_unique("person1_person2", ["person1", "person2"]),
        ],
    )
}

fn sailor(id: u64, rating: u64) -> Row {
    Row::new(
        "Sailor",
        [("id", Value::Serial(id)), ("rating", Value::U64(rating))],
    )
}
fn boat(id: u64, color: u8) -> Row {
    Row::new(
        "Boat",
        [("id", Value::Serial(id)), ("color", Value::Enum(color))],
    )
}
fn reserve(sailor: u64, boat: u64, day: i64) -> Row {
    Row::new(
        "Reserve",
        [
            ("sailor", Value::Serial(sailor)),
            ("boat", Value::Serial(boat)),
            ("day", Value::Timestamp(TimestampMicros(day))),
        ],
    )
}
fn edge_ab(a: u64, b: u64) -> Row {
    Row::new("EdgeAB", [("a", Value::U64(a)), ("b", Value::U64(b))])
}
fn edge_ac(a: u64, c: u64) -> Row {
    Row::new("EdgeAC", [("a", Value::U64(a)), ("c", Value::U64(c))])
}
fn edge_bc(b: u64, c: u64) -> Row {
    Row::new("EdgeBC", [("b", Value::U64(b)), ("c", Value::U64(c))])
}
fn customer(id: u64, nation: u64) -> Row {
    Row::new(
        "Customer",
        [("id", Value::Serial(id)), ("nation", Value::U64(nation))],
    )
}
fn supplier(id: u64, nation: u64) -> Row {
    Row::new(
        "Supplier",
        [("id", Value::Serial(id)), ("nation", Value::U64(nation))],
    )
}
fn orders(id: u64, customer: u64) -> Row {
    Row::new(
        "Orders",
        [
            ("id", Value::Serial(id)),
            ("customer", Value::Serial(customer)),
        ],
    )
}
fn lineitem(id: u64, order: u64, price: i128) -> Row {
    Row::new(
        "LineItem",
        [
            ("id", Value::Serial(id)),
            ("order", Value::Serial(order)),
            ("extended_price", Value::Decimal(DecimalRaw(price))),
        ],
    )
}
fn title(id: u64, year: i64) -> Row {
    Row::new(
        "Title",
        [("id", Value::Serial(id)), ("year", Value::I64(year))],
    )
}
fn name(id: u64) -> Row {
    Row::new("Name", [("id", Value::Serial(id))])
}
fn principal(title: u64, name: u64, category: u8, ordering: u64) -> Row {
    Row::new(
        "Principal",
        [
            ("title", Value::Serial(title)),
            ("name", Value::Serial(name)),
            ("category", Value::Enum(category)),
            ("ordering", Value::U64(ordering)),
        ],
    )
}
fn player(id: u64) -> Row {
    Row::new("Player", [("id", Value::Serial(id))])
}
fn team(id: u64, year: i64) -> Row {
    Row::new(
        "Team",
        [("id", Value::Serial(id)), ("year", Value::I64(year))],
    )
}
fn batting(player: u64, team: u64, year: i64, hits: i64) -> Row {
    Row::new(
        "Batting",
        [
            ("player", Value::Serial(player)),
            ("team", Value::Serial(team)),
            ("year", Value::I64(year)),
            ("hits", Value::I64(hits)),
        ],
    )
}
fn salary(player: u64, team: u64, year: i64, salary: i64) -> Row {
    Row::new(
        "Salary",
        [
            ("player", Value::Serial(player)),
            ("team", Value::Serial(team)),
            ("year", Value::I64(year)),
            ("salary", Value::I64(salary)),
        ],
    )
}
fn person(id: u64) -> Row {
    Row::new("Person", [("id", Value::Serial(id))])
}
fn knows(left: u64, right: u64) -> Row {
    Row::new(
        "Knows",
        [
            ("person1", Value::Serial(left)),
            ("person2", Value::Serial(right)),
        ],
    )
}
