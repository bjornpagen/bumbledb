fn all_datasets(scale: u64) -> Vec<Dataset> {
    vec![
        ledger_dataset(scale),
        sailors_dataset(scale),
        join_stress_dataset(scale),
        tpch_dataset(scale),
    ]
}

fn ledger_dataset(scale: u64) -> Dataset {
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
                    SELECT p.instrument, SUM(p.amount) FROM posting p
                    JOIN account a ON a.id = p.account
                    WHERE a.holder = ?1
                    GROUP BY p.instrument
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

fn sailors_dataset(scale: u64) -> Dataset {
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

fn join_stress_dataset(scale: u64) -> Dataset {
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
                sqlite: "SELECT COUNT(DISTINCT eab.a) FROM edge_ab eab JOIN edge_ac eac ON eac.a = eab.a JOIN edge_bc ebc ON ebc.b = eab.b AND ebc.c = eac.c",
                sqlite_params: vec![],
            },
        ],
    }
}

fn tpch_dataset(scale: u64) -> Dataset {
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
                    SELECT c.id, SUM(l.extended_price) FROM customer c
                    JOIN orders o ON o.customer = c.id
                    JOIN lineitem l ON l.ord = o.id
                    WHERE c.nation = ?1 AND l.ship_date >= ?2 AND l.ship_date < ?3
                    GROUP BY c.id
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

fn build_ledger_postings_for_holder_range(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("amount", "amount")?
        .var("at", "t")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("posting")?
        .find_var("amount")?
        .finish()
}

fn build_ledger_balances_by_instrument(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("instrument", "instrument")?
        .var("amount", "amount")?
        .var("at", "t")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .find_var("instrument")?
        .find_sum_over("amount", ["posting"])?
        .finish()
}

fn build_ledger_tag_lookup_join(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("PostingTag")?
        .var("posting", "posting")?
        .input("tag", "tag")?
        .done()
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .done()
        .find_var("posting")?
        .find_var("account")?
        .finish()
}

fn build_sailors_red_boat_sailors(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Reserve")?
        .var("sailor", "sailor")?
        .var("boat", "boat")?
        .done()
        .rel("Boat")?
        .var("id", "boat")?
        .input("color", "color")?
        .done()
        .rel("Sailor")?
        .var("id", "sailor")?
        .var("rating", "rating")?
        .done()
        .find_var("sailor")?
        .find_var("rating")?
        .finish()
}

fn build_sailors_sailor_range_reserves(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Reserve")?
        .input("sailor", "sailor")?
        .var("boat", "boat")?
        .var("day", "day")?
        .done()
        .cmp(
            OperandRef::var("day"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("day"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("boat")?
        .find_var("day")?
        .finish()
}

fn build_sailors_high_rating_red_boats(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Sailor")?
        .var("id", "sailor")?
        .var("rating", "rating")?
        .done()
        .rel("Reserve")?
        .var("sailor", "sailor")?
        .var("boat", "boat")?
        .done()
        .rel("Boat")?
        .var("id", "boat")?
        .input("color", "color")?
        .done()
        .cmp(
            OperandRef::var("rating"),
            ComparisonOperator::Gte,
            OperandRef::input("min_rating"),
        )?
        .find_var("sailor")?
        .find_var("boat")?
        .finish()
}

fn build_joinstress_chain4_from_a(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("A")?
        .input("id", "a")?
        .done()
        .rel("B")?
        .var("id", "b")?
        .input("a", "a")?
        .done()
        .rel("C")?
        .var("id", "c")?
        .var("b", "b")?
        .done()
        .rel("D")?
        .var("id", "d")?
        .var("c", "c")?
        .done()
        .find_var("d")?
        .finish()
}

fn build_joinstress_triangle_count(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
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
        .find_count_domain(["a"])?
        .finish()
}

fn build_tpch_revenue_by_customer_range(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
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
        .var("ship_date", "ship")?
        .done()
        .cmp(
            OperandRef::var("ship"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("ship"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("customer")?
        .find_sum_over("price", ["line"])?
        .finish()
}

fn build_tpch_supplier_nation_orders(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Supplier")?
        .var("id", "supplier")?
        .input("nation", "nation")?
        .done()
        .rel("LineItem")?
        .var("id", "line")?
        .var("order", "order")?
        .var("supplier", "supplier")?
        .done()
        .rel("Orders")?
        .var("id", "order")?
        .var("customer", "customer")?
        .done()
        .find_var("line")?
        .find_var("order")?
        .finish()
}

fn sailors_facts(sailors: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    for sid in 1..=sailors {
        facts.push(Fact::new(
            "Sailor",
            [
                ("id", Value::Serial(sid)),
                ("name", Value::String(format!("sailor-{sid}"))),
                ("rating", Value::U64((sid % 10) + 1)),
                ("age", Value::I64(18 + (sid % 50) as i64)),
            ],
        ));
    }
    let boats = (sailors / 4).max(10);
    for bid in 1..=boats {
        facts.push(Fact::new(
            "Boat",
            [
                ("id", Value::Serial(bid)),
                ("name", Value::String(format!("boat-{bid}"))),
                ("color", Value::Enum(((bid % 3) + 1) as u8)),
            ],
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for sid in 1..=sailors {
        for offset in 0..5 {
            let bid = ((sid + offset * 7) % boats) + 1;
            let day = ((sid * 10 + offset) as i64) * 86_400;
            if seen.insert((sid, bid, day)) {
                facts.push(Fact::new(
                    "Reserve",
                    [
                        ("sailor", Value::Serial(sid)),
                        ("boat", Value::Serial(bid)),
                        ("day", Value::Timestamp(TimestampMicros(day))),
                    ],
                ));
            }
        }
    }
    facts
}

fn join_stress_facts(n: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    for id in 1..=n {
        facts.push(Fact::new(
            "A",
            [
                ("id", Value::Serial(id)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
        facts.push(Fact::new(
            "B",
            [
                ("id", Value::Serial(id)),
                ("a", Value::Serial(((id - 1) % n) + 1)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
        facts.push(Fact::new(
            "C",
            [
                ("id", Value::Serial(id)),
                ("b", Value::Serial(((id - 1) % n) + 1)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
        facts.push(Fact::new(
            "D",
            [
                ("id", Value::Serial(id)),
                ("c", Value::Serial(((id - 1) % n) + 1)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
    }
    let mut ab = std::collections::BTreeSet::new();
    let mut ac = std::collections::BTreeSet::new();
    let mut bc = std::collections::BTreeSet::new();
    for a in 1..=n {
        for offset in 0..3 {
            let b = ((a + offset) % n) + 1;
            let c = ((a + offset * 2) % n) + 1;
            if ab.insert((a, b)) {
                facts.push(Fact::new(
                    "EdgeAB",
                    [("a", Value::Serial(a)), ("b", Value::Serial(b))],
                ));
            }
            if ac.insert((a, c)) {
                facts.push(Fact::new(
                    "EdgeAC",
                    [("a", Value::Serial(a)), ("c", Value::Serial(c))],
                ));
            }
            if bc.insert((b, c)) {
                facts.push(Fact::new(
                    "EdgeBC",
                    [("b", Value::Serial(b)), ("c", Value::Serial(c))],
                ));
            }
        }
    }
    facts
}

fn tpch_facts(n: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    for id in 1..=n {
        facts.push(Fact::new(
            "Customer",
            [
                ("id", Value::Serial(id)),
                ("nation", Value::U64((id % 5) + 1)),
            ],
        ));
        facts.push(Fact::new(
            "Supplier",
            [
                ("id", Value::Serial(id)),
                ("nation", Value::U64((id % 7) + 1)),
            ],
        ));
        facts.push(Fact::new(
            "Part",
            [
                ("id", Value::Serial(id)),
                ("brand", Value::U64((id % 11) + 1)),
            ],
        ));
        facts.push(Fact::new(
            "Orders",
            [
                ("id", Value::Serial(id)),
                ("customer", Value::Serial(((id - 1) % n) + 1)),
                (
                    "order_date",
                    Value::Timestamp(TimestampMicros(id as i64 * 10)),
                ),
            ],
        ));
    }
    let mut line = 1;
    for order in 1..=n {
        for offset in 0..4 {
            facts.push(Fact::new(
                "LineItem",
                [
                    ("id", Value::Serial(line)),
                    ("order", Value::Serial(order)),
                    ("part", Value::Serial(((order + offset) % n) + 1)),
                    ("supplier", Value::Serial(((order + offset * 3) % n) + 1)),
                    ("quantity", Value::I64((offset + 1) as i64)),
                    (
                        "extended_price",
                        Value::Decimal(DecimalRaw(line as i128 * 100)),
                    ),
                    (
                        "ship_date",
                        Value::Timestamp(TimestampMicros(line as i64 * 10)),
                    ),
                ],
            ));
            line += 1;
        }
    }
    facts
}

fn insert_ledger_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Holder" => {
                tx.execute(
                    "INSERT INTO holder (id, name) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, text(fact, "name")?],
                )?;
            }
            "Account" => {
                tx.execute(
                    "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        rf(fact, "holder")?,
                        symbol(fact, "currency")?
                    ],
                )?;
            }
            "Instrument" => {
                tx.execute(
                    "INSERT INTO instrument (id, symbol) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, text(fact, "symbol")?],
                )?;
            }
            "JournalEntry" => {
                tx.execute(
                    "INSERT INTO journal_entry (id, source, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        rf(fact, "source")?,
                        ts(fact, "created_at")?
                    ],
                )?;
            }
            "Posting" => {
                tx.execute("INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(fact, "id")?, rf(fact, "entry")?, rf(fact, "account")?, rf(fact, "instrument")?, dec(fact, "amount")?, ts(fact, "at")?])?;
            }
            "PostingTag" => {
                tx.execute(
                    "INSERT INTO posting_tag (posting, tag) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "posting")?, symbol(fact, "tag")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_sailors_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Sailor" => {
                tx.execute(
                    "INSERT INTO sailor (id, name, rating, age) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        id(fact, "id")?,
                        text(fact, "name")?,
                        u64v(fact, "rating")?,
                        i64v(fact, "age")?
                    ],
                )?;
            }
            "Boat" => {
                tx.execute(
                    "INSERT INTO boat (id, name, color) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, text(fact, "name")?, symbol(fact, "color")?],
                )?;
            }
            "Reserve" => {
                tx.execute(
                    "INSERT INTO reserve (sailor, boat, day) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(fact, "sailor")?, rf(fact, "boat")?, ts(fact, "day")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_join_stress_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "A" => {
                tx.execute(
                    "INSERT INTO a (id, k) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "k")?],
                )?;
            }
            "B" => {
                tx.execute(
                    "INSERT INTO b (id, a, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "a")?, symbol(fact, "k")?],
                )?;
            }
            "C" => {
                tx.execute(
                    "INSERT INTO c (id, b, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "b")?, symbol(fact, "k")?],
                )?;
            }
            "D" => {
                tx.execute(
                    "INSERT INTO d (id, c, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "c")?, symbol(fact, "k")?],
                )?;
            }
            "EdgeAB" => {
                tx.execute(
                    "INSERT INTO edge_ab (a, b) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "a")?, rf(fact, "b")?],
                )?;
            }
            "EdgeAC" => {
                tx.execute(
                    "INSERT INTO edge_ac (a, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "a")?, rf(fact, "c")?],
                )?;
            }
            "EdgeBC" => {
                tx.execute(
                    "INSERT INTO edge_bc (b, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "b")?, rf(fact, "c")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_tpch_sqlite(conn: &Connection, facts: &[Fact]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Customer" => {
                tx.execute(
                    "INSERT INTO customer (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "nation")?],
                )?;
            }
            "Supplier" => {
                tx.execute(
                    "INSERT INTO supplier (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "nation")?],
                )?;
            }
            "Part" => {
                tx.execute(
                    "INSERT INTO part (id, brand) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "brand")?],
                )?;
            }
            "Orders" => {
                tx.execute(
                    "INSERT INTO orders (id, customer, order_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        rf(fact, "customer")?,
                        ts(fact, "order_date")?
                    ],
                )?;
            }
            "LineItem" => {
                tx.execute("INSERT INTO lineitem (id, ord, part, supplier, quantity, extended_price, ship_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(fact, "id")?, rf(fact, "order")?, rf(fact, "part")?, rf(fact, "supplier")?, i64v(fact, "quantity")?, dec(fact, "extended_price")?, ts(fact, "ship_date")?])?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn id(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Serial(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "id", other)),
    }
}

pub(crate) fn rf(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Serial(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "ref", other)),
    }
}

pub(crate) fn symbol(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Enum(v) => Ok(i64::from(*v)),
        Value::U64(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "symbol", other)),
    }
}

pub(crate) fn dec(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Decimal(DecimalRaw(v)) => Ok(*v as i64),
        other => Err(unexpected_value(field, "decimal", other)),
    }
}

pub(crate) fn ts(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::Timestamp(TimestampMicros(v)) => Ok(*v),
        other => Err(unexpected_value(field, "timestamp", other)),
    }
}

pub(crate) fn u64v(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::U64(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "u64", other)),
    }
}

pub(crate) fn i64v(fact: &Fact, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::I64(v) => Ok(*v),
        other => Err(unexpected_value(field, "i64", other)),
    }
}

pub(crate) fn text(fact: &Fact, field: &str) -> Result<String, Box<dyn std::error::Error>> {
    match required_value(fact, field)? {
        Value::String(v) => Ok(v.clone()),
        other => Err(unexpected_value(field, "string", other)),
    }
}

fn required_value<'a>(
    fact: &'a Fact,
    field: &str,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
    fact.value(field)
        .ok_or_else(|| bench_error(format!("missing field {field}")))
}

fn unexpected_value(field: &str, expected: &str, actual: &Value) -> Box<dyn std::error::Error> {
    bench_error(format!("expected {expected} {field}, got {actual:?}"))
}

pub(crate) fn serial_key_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: relation.to_owned(),
        },
    )
}

pub(crate) fn serial_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: target.to_owned(),
        },
    )
}

