use super::*;

#[path = "datasets/facts.rs"]
mod dataset_facts;
#[path = "datasets/helpers.rs"]
mod dataset_helpers;
#[path = "datasets/queries.rs"]
mod dataset_queries;
#[path = "datasets/sqlite.rs"]
mod dataset_sqlite;

use dataset_facts::{join_stress_facts, sailors_facts, tpch_facts};
pub(crate) use dataset_helpers::{
    dec, i64v, id, rf, serial_field, serial_key_field, symbol, text, ts, u64v,
};
use dataset_queries::*;
use dataset_sqlite::*;

pub(crate) fn all_datasets(scale: u64) -> Vec<Dataset> {
    vec![
        ledger_dataset(scale),
        sailors_dataset(scale),
        join_stress_dataset(scale),
        tpch_dataset(scale),
    ]
}

pub(super) fn ledger_dataset(scale: u64) -> Dataset {
    Dataset {
        name: "ledger",
        schema: bumbledb_lmdb::benchmark::benchmark_schema(),
        facts: bumbledb_lmdb::benchmark::benchmark_facts(scale),
        fact_source: None,
        sqlite_schema: r#"
            CREATE TABLE holder (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE account (id INTEGER PRIMARY KEY, holder INTEGER NOT NULL, currency INTEGER NOT NULL);
            CREATE TABLE instrument (id INTEGER PRIMARY KEY, symbol TEXT NOT NULL);
            CREATE TABLE journal_entry (id INTEGER PRIMARY KEY, source INTEGER NOT NULL, created_at INTEGER NOT NULL);
            CREATE TABLE posting (id INTEGER PRIMARY KEY, entry INTEGER NOT NULL, account INTEGER NOT NULL, instrument INTEGER NOT NULL, amount INTEGER NOT NULL, at INTEGER NOT NULL);
            CREATE TABLE posting_tag (posting INTEGER NOT NULL, tag INTEGER NOT NULL, PRIMARY KEY (posting, tag));
            CREATE INDEX account_holder ON account(holder, id);
            CREATE INDEX posting_account ON posting(account, id);
            CREATE INDEX posting_at ON posting(at, id);
            CREATE INDEX posting_instrument ON posting(instrument, id);
            CREATE INDEX posting_tag_tag ON posting_tag(tag, posting);
        "#,
        sqlite_insert: insert_ledger_sqlite,
        queries: vec![
            BenchQuery {
                name: "postings_for_holder_range",
                build: build_ledger_postings_for_holder_range,
                inputs: vec![
                    ("holder", Value::Serial(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    (
                        "end",
                        Value::Timestamp(TimestampMicros((scale as i64 * 3 + 1) * 10)),
                    ),
                ],
                sqlite: r#"
                    SELECT DISTINCT p.id, p.amount FROM posting p
                    JOIN account a ON a.id = p.account
                    WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
                "#,
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64((scale as i64 * 3 + 1) * 10),
                ],
            },
            BenchQuery {
                name: "balances_by_instrument",
                build: build_ledger_balances_by_instrument,
                inputs: vec![("holder", Value::Serial(1))],
                sqlite: r#"
                    SELECT DISTINCT p.instrument, p.amount FROM posting p
                    JOIN account a ON a.id = p.account
                    WHERE a.holder = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "tag_lookup_join",
                build: build_ledger_tag_lookup_join,
                inputs: vec![("tag", Value::Enum(1))],
                sqlite: r#"
                    SELECT DISTINCT p.id, p.account FROM posting_tag t
                    JOIN posting p ON p.id = t.posting
                    WHERE t.tag = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(1)],
            },
        ],
    }
}

pub(super) fn sailors_dataset(scale: u64) -> Dataset {
    let sailors = scale.max(10);
    Dataset {
        name: "sailors",
        schema: SchemaDescriptor::new(
            "SailorsDb",
            vec![
                RelationDescriptor::new(
                    "Sailor",
                    vec![
                        serial_key_field("SailorId", "Sailor"),
                        FieldDescriptor::new("name", ValueType::String),
                        FieldDescriptor::new("rating", ValueType::U64).range_indexed(),
                        FieldDescriptor::new("age", ValueType::I64),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Boat",
                    vec![
                        serial_key_field("BoatId", "Boat"),
                        FieldDescriptor::new("name", ValueType::String),
                        FieldDescriptor::new(
                            "color",
                            ValueType::Enum {
                                name: "Color".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_index(IndexDescriptor::equality("by_color", ["color", "id"])),
                RelationDescriptor::new(
                    "Reserve",
                    vec![
                        serial_field("SailorId", "sailor", "Sailor"),
                        serial_field("BoatId", "boat", "Boat"),
                        FieldDescriptor::new("day", ValueType::TimestampMicros).range_indexed(),
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
        .with_enum(EnumDescriptor::codes("Color", [1, 2, 3])),
        facts: sailors_facts(sailors),
        fact_source: None,
        sqlite_schema: r#"
            CREATE TABLE sailor (id INTEGER PRIMARY KEY, name TEXT NOT NULL, rating INTEGER NOT NULL, age INTEGER NOT NULL);
            CREATE TABLE boat (id INTEGER PRIMARY KEY, name TEXT NOT NULL, color INTEGER NOT NULL);
            CREATE TABLE reserve (sailor INTEGER NOT NULL, boat INTEGER NOT NULL, day INTEGER NOT NULL, PRIMARY KEY (sailor, boat, day));
            CREATE INDEX sailor_rating ON sailor(rating, id);
            CREATE INDEX boat_color ON boat(color, id);
            CREATE INDEX reserve_sailor ON reserve(sailor, boat, day);
            CREATE INDEX reserve_boat ON reserve(boat, sailor, day);
            CREATE INDEX reserve_day ON reserve(day, sailor, boat);
        "#,
        sqlite_insert: insert_sailors_sqlite,
        queries: vec![
            BenchQuery {
                name: "red_boat_sailors",
                build: build_sailors_red_boat_sailors,
                inputs: vec![("color", Value::Enum(1))],
                sqlite: r#"
                    SELECT DISTINCT s.id, s.rating FROM reserve r
                    JOIN boat b ON b.id = r.boat
                    JOIN sailor s ON s.id = r.sailor
                    WHERE b.color = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "sailor_range_reserves",
                build: build_sailors_sailor_range_reserves,
                inputs: vec![
                    ("sailor", Value::Serial(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(10_000_000))),
                ],
                sqlite: "SELECT DISTINCT boat, day FROM reserve WHERE sailor = ?1 AND day >= ?2 AND day < ?3",
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64(10_000_000),
                ],
            },
            BenchQuery {
                name: "high_rating_red_boats",
                build: build_sailors_high_rating_red_boats,
                inputs: vec![("color", Value::Enum(1)), ("min_rating", Value::U64(7))],
                sqlite: r#"
                    SELECT DISTINCT s.id, b.id FROM sailor s
                    JOIN reserve r ON r.sailor = s.id
                    JOIN boat b ON b.id = r.boat
                    WHERE b.color = ?1 AND s.rating >= ?2
                "#,
                sqlite_params: vec![SqlParam::I64(1), SqlParam::I64(7)],
            },
        ],
    }
}

pub(super) fn join_stress_dataset(scale: u64) -> Dataset {
    let n = scale.max(20);
    Dataset {
        name: "joinstress",
        schema: SchemaDescriptor::new(
            "JoinStressDb",
            vec![
                RelationDescriptor::new(
                    "A",
                    vec![
                        serial_key_field("AId", "A"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Enum {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "B",
                    vec![
                        serial_key_field("BId", "B"),
                        serial_field("AId", "a", "A"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Enum {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key("a", ["a"], "A", "id")),
                RelationDescriptor::new(
                    "C",
                    vec![
                        serial_key_field("CId", "C"),
                        serial_field("BId", "b", "B"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Enum {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key("b", ["b"], "B", "id")),
                RelationDescriptor::new(
                    "D",
                    vec![
                        serial_key_field("DId", "D"),
                        serial_field("CId", "c", "C"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Enum {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key("c", ["c"], "C", "id")),
                RelationDescriptor::new(
                    "EdgeAB",
                    vec![serial_field("AId", "a", "A"), serial_field("BId", "b", "B")],
                )
                .with_unique("a_b", ["a", "b"])
                .with_constraint(ConstraintDescriptor::foreign_key("a", ["a"], "A", "id"))
                .with_constraint(ConstraintDescriptor::foreign_key("b", ["b"], "B", "id")),
                RelationDescriptor::new(
                    "EdgeAC",
                    vec![serial_field("AId", "a", "A"), serial_field("CId", "c", "C")],
                )
                .with_unique("a_c", ["a", "c"])
                .with_constraint(ConstraintDescriptor::foreign_key("a", ["a"], "A", "id"))
                .with_constraint(ConstraintDescriptor::foreign_key("c", ["c"], "C", "id")),
                RelationDescriptor::new(
                    "EdgeBC",
                    vec![serial_field("BId", "b", "B"), serial_field("CId", "c", "C")],
                )
                .with_unique("b_c", ["b", "c"])
                .with_constraint(ConstraintDescriptor::foreign_key("b", ["b"], "B", "id"))
                .with_constraint(ConstraintDescriptor::foreign_key("c", ["c"], "C", "id")),
            ],
        )
        .with_enum(EnumDescriptor::codes("K", 0..10)),
        facts: join_stress_facts(n),
        fact_source: None,
        sqlite_schema: r#"
            CREATE TABLE a (id INTEGER PRIMARY KEY, k INTEGER NOT NULL);
            CREATE TABLE b (id INTEGER PRIMARY KEY, a INTEGER NOT NULL, k INTEGER NOT NULL);
            CREATE TABLE c (id INTEGER PRIMARY KEY, b INTEGER NOT NULL, k INTEGER NOT NULL);
            CREATE TABLE d (id INTEGER PRIMARY KEY, c INTEGER NOT NULL, k INTEGER NOT NULL);
            CREATE TABLE edge_ab (a INTEGER NOT NULL, b INTEGER NOT NULL, PRIMARY KEY (a, b));
            CREATE TABLE edge_ac (a INTEGER NOT NULL, c INTEGER NOT NULL, PRIMARY KEY (a, c));
            CREATE TABLE edge_bc (b INTEGER NOT NULL, c INTEGER NOT NULL, PRIMARY KEY (b, c));
            CREATE INDEX b_a ON b(a, id);
            CREATE INDEX c_b ON c(b, id);
            CREATE INDEX d_c ON d(c, id);
            CREATE INDEX edge_ab_b ON edge_ab(b, a);
            CREATE INDEX edge_ac_c ON edge_ac(c, a);
            CREATE INDEX edge_bc_c ON edge_bc(c, b);
        "#,
        sqlite_insert: insert_join_stress_sqlite,
        queries: vec![
            BenchQuery {
                name: "chain4_from_a",
                build: build_joinstress_chain4_from_a,
                inputs: vec![("a", Value::Serial(1))],
                sqlite: "SELECT DISTINCT d.id FROM a JOIN b ON b.a = a.id JOIN c ON c.b = b.id JOIN d ON d.c = c.id WHERE a.id = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "triangle_count",
                build: build_joinstress_triangle_count,
                inputs: vec![],
                sqlite: "SELECT DISTINCT eab.a FROM edge_ab eab JOIN edge_ac eac ON eac.a = eab.a JOIN edge_bc ebc ON ebc.b = eab.b AND ebc.c = eac.c",
                sqlite_params: vec![],
            },
        ],
    }
}

pub(super) fn tpch_dataset(scale: u64) -> Dataset {
    let n = scale.max(20);
    Dataset {
        name: "tpch",
        schema: SchemaDescriptor::new(
            "TpchSubsetDb",
            vec![
                RelationDescriptor::new(
                    "Customer",
                    vec![
                        serial_key_field("CustomerId", "Customer"),
                        FieldDescriptor::new("nation", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"])
                .with_index(IndexDescriptor::equality("by_nation", ["nation", "id"])),
                RelationDescriptor::new(
                    "Supplier",
                    vec![
                        serial_key_field("SupplierId", "Supplier"),
                        FieldDescriptor::new("nation", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"])
                .with_index(IndexDescriptor::equality("by_nation", ["nation", "id"])),
                RelationDescriptor::new(
                    "Part",
                    vec![
                        serial_key_field("PartId", "Part"),
                        FieldDescriptor::new("brand", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Orders",
                    vec![
                        serial_key_field("OrderId", "Orders"),
                        serial_field("CustomerId", "customer", "Customer"),
                        FieldDescriptor::new("order_date", ValueType::TimestampMicros)
                            .range_indexed(),
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
                        serial_key_field("LineItemId", "LineItem"),
                        serial_field("OrderId", "order", "Orders"),
                        serial_field("PartId", "part", "Part"),
                        serial_field("SupplierId", "supplier", "Supplier"),
                        FieldDescriptor::new("quantity", ValueType::I64),
                        FieldDescriptor::new("extended_price", ValueType::Decimal { scale: 2 }),
                        FieldDescriptor::new("ship_date", ValueType::TimestampMicros)
                            .range_indexed(),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "order",
                    ["order"],
                    "Orders",
                    "id",
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "part",
                    ["part"],
                    "Part",
                    "id",
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "supplier",
                    ["supplier"],
                    "Supplier",
                    "id",
                )),
            ],
        ),
        facts: tpch_facts(n),
        fact_source: None,
        sqlite_schema: r#"
            CREATE TABLE customer (id INTEGER PRIMARY KEY, nation INTEGER NOT NULL);
            CREATE TABLE supplier (id INTEGER PRIMARY KEY, nation INTEGER NOT NULL);
            CREATE TABLE part (id INTEGER PRIMARY KEY, brand INTEGER NOT NULL);
            CREATE TABLE orders (id INTEGER PRIMARY KEY, customer INTEGER NOT NULL, order_date INTEGER NOT NULL);
            CREATE TABLE lineitem (id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, part INTEGER NOT NULL, supplier INTEGER NOT NULL, quantity INTEGER NOT NULL, extended_price INTEGER NOT NULL, ship_date INTEGER NOT NULL);
            CREATE INDEX orders_customer ON orders(customer, id);
            CREATE INDEX lineitem_order ON lineitem(ord, id);
            CREATE INDEX lineitem_supplier ON lineitem(supplier, id);
            CREATE INDEX lineitem_ship_date ON lineitem(ship_date, id);
            CREATE INDEX supplier_nation ON supplier(nation, id);
        "#,
        sqlite_insert: insert_tpch_sqlite,
        queries: vec![
            BenchQuery {
                name: "revenue_by_customer_range",
                build: build_tpch_revenue_by_customer_range,
                inputs: vec![
                    ("nation", Value::U64(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(1_000_000_000))),
                ],
                sqlite: r#"
                    SELECT DISTINCT c.id, l.extended_price FROM customer c
                    JOIN orders o ON o.customer = c.id
                    JOIN lineitem l ON l.ord = o.id
                    WHERE c.nation = ?1 AND l.ship_date >= ?2 AND l.ship_date < ?3
                "#,
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64(1_000_000_000),
                ],
            },
            BenchQuery {
                name: "supplier_nation_orders",
                build: build_tpch_supplier_nation_orders,
                inputs: vec![("nation", Value::U64(2))],
                sqlite: r#"
                    SELECT DISTINCT l.id, o.id FROM supplier s
                    JOIN lineitem l ON l.supplier = s.id
                    JOIN orders o ON o.id = l.ord
                    WHERE s.nation = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(2)],
            },
        ],
    }
}
