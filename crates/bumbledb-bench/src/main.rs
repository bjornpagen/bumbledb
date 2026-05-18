use std::hint::black_box;
use std::time::{Duration, Instant};

use bumbledb_core::datalog::parse_and_typecheck;
use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{
    FieldDescriptor, IndexDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind,
    SchemaDescriptor, ValueType,
};
use bumbledb_lmdb::{Environment, InputBindings, Row, StorageSchema, Value};
use rusqlite::{Connection, params_from_iter};

mod open;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    if config.trace {
        let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "bumbledb_lmdb=debug".to_owned());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .try_init()
            .ok();
    }
    println!("BumbleDB benchmark suite");
    println!(
        "scale={} repeats={} datasets={:?} open_datasets={}",
        config.scale,
        config.repeats,
        config.datasets,
        config.has_open_datasets()
    );
    println!();

    let mut datasets = all_datasets(config.scale);
    datasets.extend(open::open_datasets(&config)?);

    let datasets = datasets
        .into_iter()
        .filter(|dataset| {
            config.datasets.is_empty() || config.datasets.iter().any(|name| name == dataset.name)
        })
        .collect::<Vec<_>>();

    if datasets.is_empty() {
        return Err("no matching datasets".into());
    }

    for dataset in datasets {
        run_dataset(dataset, config.repeats)?;
        println!();
    }

    Ok(())
}

#[derive(Debug)]
struct Config {
    scale: u64,
    repeats: u64,
    datasets: Vec<String>,
    imdb_dir: Option<String>,
    tpch_dir: Option<String>,
    lahman_dir: Option<String>,
    ldbc_dir: Option<String>,
    trace: bool,
}

impl Config {
    fn from_env() -> Self {
        let mut scale = 200;
        let mut repeats = 10;
        let mut datasets = Vec::new();
        let mut imdb_dir = None;
        let mut tpch_dir = None;
        let mut lahman_dir = None;
        let mut ldbc_dir = None;
        let mut trace = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scale" => {
                    scale = args
                        .next()
                        .expect("--scale value")
                        .parse()
                        .expect("numeric scale")
                }
                "--repeats" => {
                    repeats = args
                        .next()
                        .expect("--repeats value")
                        .parse()
                        .expect("numeric repeats")
                }
                "--dataset" => datasets.push(args.next().expect("--dataset value")),
                "--imdb-dir" => imdb_dir = Some(args.next().expect("--imdb-dir value")),
                "--tpch-dir" => tpch_dir = Some(args.next().expect("--tpch-dir value")),
                "--lahman-dir" => lahman_dir = Some(args.next().expect("--lahman-dir value")),
                "--ldbc-dir" => ldbc_dir = Some(args.next().expect("--ldbc-dir value")),
                "--trace" => trace = true,
                "--help" | "-h" => {
                    println!(
                        "usage: cargo run -p bumbledb-bench --release -- [--scale N] [--repeats N] [--trace] [--dataset ledger|sailors|joinstress|tpch|imdb|tpch-open|lahman|ldbc] [--imdb-dir DIR] [--tpch-dir DIR] [--lahman-dir DIR] [--ldbc-dir DIR]"
                    );
                    std::process::exit(0);
                }
                other => panic!("unknown arg {other}"),
            }
        }
        Self {
            scale,
            repeats,
            datasets,
            imdb_dir,
            tpch_dir,
            lahman_dir,
            ldbc_dir,
            trace,
        }
    }

    fn has_open_datasets(&self) -> bool {
        self.imdb_dir.is_some()
            || self.tpch_dir.is_some()
            || self.lahman_dir.is_some()
            || self.ldbc_dir.is_some()
    }
}

pub(crate) struct Dataset {
    name: &'static str,
    schema: SchemaDescriptor,
    rows: Vec<Row>,
    sqlite_schema: &'static str,
    sqlite_insert: SqliteInsert,
    queries: Vec<BenchQuery>,
}

pub(crate) type SqliteInsert = fn(&Connection, &[Row]) -> Result<(), Box<dyn std::error::Error>>;

pub(crate) struct BenchQuery {
    name: &'static str,
    datalog: &'static str,
    inputs: Vec<(&'static str, Value)>,
    sqlite: &'static str,
    sqlite_params: Vec<SqlParam>,
}

#[derive(Clone)]
pub(crate) enum SqlParam {
    I64(i64),
}

impl rusqlite::ToSql for SqlParam {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SqlParam::I64(value) => Ok((*value).into()),
        }
    }
}

fn run_dataset(dataset: Dataset, repeats: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("== {} ==", dataset.name);
    println!("rows={}", dataset.rows.len());

    let bumble_dir = tempfile::tempdir()?;
    let bumble_env = Environment::open(bumble_dir.path())?;
    let bumble_schema = StorageSchema::new(dataset.schema.clone(), bumble_env.max_key_size())?;

    let bumble_load = timed(|| bumble_env.bulk_load(&bumble_schema, dataset.rows.clone()))?;
    println!("load.bumbledb={:?}", bumble_load.elapsed);

    let mut sqlite = Connection::open_in_memory()?;
    sqlite.execute_batch(dataset.sqlite_schema)?;
    let sqlite_load = timed(|| (dataset.sqlite_insert)(&sqlite, &dataset.rows))?;
    println!("load.sqlite={:?}", sqlite_load.elapsed);

    for query in dataset.queries {
        let typed = parse_and_typecheck(bumble_schema.descriptor(), query.datalog)?;
        let inputs = InputBindings::from_values(query.inputs.clone());
        let params = query.sqlite_params.clone();

        let bumble_once =
            bumble_env.read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs))?;
        let sqlite_once = sqlite_count(&mut sqlite, query.sqlite, &params)?;
        if bumble_once.rows.len() != sqlite_once {
            return Err(format!(
                "{}:{} row mismatch bumbledb={} sqlite={}",
                dataset.name,
                query.name,
                bumble_once.rows.len(),
                sqlite_once
            )
            .into());
        }

        let bumble_time = timed_repeated(repeats, || {
            let rows = bumble_env
                .read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs))?
                .rows;
            black_box(rows.len());
            Ok::<_, bumbledb_lmdb::Error>(())
        })?;
        let sqlite_time = timed_repeated(repeats, || {
            let rows = sqlite_count(&mut sqlite, query.sqlite, &params)?;
            black_box(rows);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;

        println!(
            "query={} rows={} bumbledb_total={:?} bumbledb_avg={:?} sqlite_total={:?} sqlite_avg={:?}",
            query.name,
            bumble_once.rows.len(),
            bumble_time,
            avg(bumble_time, repeats),
            sqlite_time,
            avg(sqlite_time, repeats),
        );
        print_explain(&bumble_once.explain());
    }

    Ok(())
}

struct Timed<T> {
    _value: T,
    elapsed: Duration,
}

fn timed<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<Timed<T>, E> {
    let start = Instant::now();
    let value = f()?;
    Ok(Timed {
        _value: value,
        elapsed: start.elapsed(),
    })
}

fn timed_repeated<E>(repeats: u64, mut f: impl FnMut() -> Result<(), E>) -> Result<Duration, E> {
    let start = Instant::now();
    for _ in 0..repeats {
        f()?;
    }
    Ok(start.elapsed())
}

fn avg(duration: Duration, repeats: u64) -> Duration {
    if repeats == 0 {
        duration
    } else {
        duration / repeats as u32
    }
}

fn sqlite_count(
    conn: &mut Connection,
    sql: &str,
    params: &[SqlParam],
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), |_| Ok(()))?
        .count();
    Ok(rows)
}

fn print_explain(explain: &str) {
    for line in explain.lines() {
        if line.contains("relation=")
            || line.contains("variable_estimate")
            || line.contains("missing_index")
            || line.contains("free_join_node")
            || line.contains("free_join_subatom")
            || line.contains("rows_scanned")
            || line.contains("cursor_seeks")
            || line.contains("trie_intersections")
            || line.contains("variable_candidates")
            || line.contains("decoded_values")
            || line.contains("dictionary_reverse_lookups")
            || line.contains("encoded_comparisons_evaluated")
            || line.contains("decoded_comparisons_evaluated")
            || line.contains("materialized_output_values")
            || line.contains("trie_open")
            || line.contains("trie_up")
            || line.contains("trie_next")
            || line.contains("trie_seek")
            || line.contains("trie_key_reads")
            || line.contains("output_rows")
        {
            println!("  {line}");
        }
    }
}

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
        rows: bumbledb_lmdb::benchmark::benchmark_rows(scale),
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
                datalog: r#"
                    find ?posting ?amount
                    where
                      Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
                      Account(id: ?account, holder: $holder)
                      ?t >= $start
                      ?t < $end
                "#,
                inputs: vec![
                    ("holder", Value::Ref(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    (
                        "end",
                        Value::Timestamp(TimestampMicros((scale as i64 * 3 + 1) * 10)),
                    ),
                ],
                sqlite: r#"
                    SELECT p.id, p.amount FROM posting p
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
                datalog: r#"
                    find ?instrument sum(?amount)
                    where
                      Posting(id: ?posting, account: ?account, instrument: ?instrument, amount: ?amount, at: ?t)
                      Account(id: ?account, holder: $holder)
                "#,
                inputs: vec![("holder", Value::Ref(1))],
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
                datalog: r#"
                    find ?posting ?account
                    where
                      PostingTag(posting: ?posting, tag: $tag)
                      Posting(id: ?posting, account: ?account)
                "#,
                inputs: vec![("tag", Value::Symbol(1))],
                sqlite: r#"
                    SELECT p.id, p.account FROM posting_tag t
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
                    RelationKind::Entity,
                    vec![
                        id_field("SailorId", "Sailor"),
                        FieldDescriptor::new("name", ValueType::String),
                        FieldDescriptor::new("rating", ValueType::U64).range_indexed(),
                        FieldDescriptor::new("age", ValueType::I64),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Boat",
                    RelationKind::Entity,
                    vec![
                        id_field("BoatId", "Boat"),
                        FieldDescriptor::new("name", ValueType::String),
                        FieldDescriptor::new(
                            "color",
                            ValueType::Symbol {
                                name: "Color".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_color", ["color", "id"])),
                RelationDescriptor::new(
                    "Reserve",
                    RelationKind::Edge,
                    vec![
                        ref_field("SailorId", "sailor", "Sailor"),
                        ref_field("BoatId", "boat", "Boat"),
                        FieldDescriptor::new("day", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["sailor", "boat", "day"]),
                ),
            ],
        ),
        rows: sailors_rows(sailors),
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
                datalog: r#"
                    find ?sailor ?rating
                    where
                      Reserve(sailor: ?sailor, boat: ?boat)
                      Boat(id: ?boat, color: $color)
                      Sailor(id: ?sailor, rating: ?rating)
                "#,
                inputs: vec![("color", Value::Symbol(1))],
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
                datalog: r#"
                    find ?boat ?day
                    where
                      Reserve(sailor: $sailor, boat: ?boat, day: ?day)
                      ?day >= $start
                      ?day < $end
                "#,
                inputs: vec![
                    ("sailor", Value::Ref(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(10_000_000))),
                ],
                sqlite: "SELECT boat, day FROM reserve WHERE sailor = ?1 AND day >= ?2 AND day < ?3",
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64(10_000_000),
                ],
            },
            BenchQuery {
                name: "high_rating_red_boats",
                datalog: r#"
                    find ?sailor ?boat
                    where
                      Sailor(id: ?sailor, rating: ?rating)
                      Reserve(sailor: ?sailor, boat: ?boat)
                      Boat(id: ?boat, color: $color)
                      ?rating >= $min_rating
                "#,
                inputs: vec![("color", Value::Symbol(1)), ("min_rating", Value::U64(7))],
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
                    RelationKind::Entity,
                    vec![
                        id_field("AId", "A"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "B",
                    RelationKind::Entity,
                    vec![
                        id_field("BId", "B"),
                        ref_field("AId", "a", "A"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "C",
                    RelationKind::Entity,
                    vec![
                        id_field("CId", "C"),
                        ref_field("BId", "b", "B"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "D",
                    RelationKind::Entity,
                    vec![
                        id_field("DId", "D"),
                        ref_field("CId", "c", "C"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "EdgeAB",
                    RelationKind::Edge,
                    vec![ref_field("AId", "a", "A"), ref_field("BId", "b", "B")],
                    PrimaryKeyDescriptor::new(["a", "b"]),
                ),
                RelationDescriptor::new(
                    "EdgeAC",
                    RelationKind::Edge,
                    vec![ref_field("AId", "a", "A"), ref_field("CId", "c", "C")],
                    PrimaryKeyDescriptor::new(["a", "c"]),
                ),
                RelationDescriptor::new(
                    "EdgeBC",
                    RelationKind::Edge,
                    vec![ref_field("BId", "b", "B"), ref_field("CId", "c", "C")],
                    PrimaryKeyDescriptor::new(["b", "c"]),
                ),
            ],
        ),
        rows: join_stress_rows(n),
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
                datalog: r#"
                    find ?d
                    where
                      A(id: $a)
                      B(id: ?b, a: $a)
                      C(id: ?c, b: ?b)
                      D(id: ?d, c: ?c)
                "#,
                inputs: vec![("a", Value::Id(1))],
                sqlite: "SELECT d.id FROM a JOIN b ON b.a = a.id JOIN c ON c.b = b.id JOIN d ON d.c = c.id WHERE a.id = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "triangle_count",
                datalog: r#"
                    find count(?a)
                    where
                      EdgeAB(a: ?a, b: ?b)
                      EdgeAC(a: ?a, c: ?c)
                      EdgeBC(b: ?b, c: ?c)
                "#,
                inputs: vec![],
                sqlite: "SELECT COUNT(eab.a) FROM edge_ab eab JOIN edge_ac eac ON eac.a = eab.a JOIN edge_bc ebc ON ebc.b = eab.b AND ebc.c = eac.c",
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
                    RelationKind::Entity,
                    vec![
                        id_field("CustomerId", "Customer"),
                        FieldDescriptor::new(
                            "nation",
                            ValueType::Symbol {
                                name: "Nation".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_nation", ["nation", "id"])),
                RelationDescriptor::new(
                    "Supplier",
                    RelationKind::Entity,
                    vec![
                        id_field("SupplierId", "Supplier"),
                        FieldDescriptor::new(
                            "nation",
                            ValueType::Symbol {
                                name: "Nation".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_nation", ["nation", "id"])),
                RelationDescriptor::new(
                    "Part",
                    RelationKind::Entity,
                    vec![
                        id_field("PartId", "Part"),
                        FieldDescriptor::new(
                            "brand",
                            ValueType::Symbol {
                                name: "Brand".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Orders",
                    RelationKind::Entity,
                    vec![
                        id_field("OrderId", "Orders"),
                        ref_field("CustomerId", "customer", "Customer"),
                        FieldDescriptor::new("order_date", ValueType::TimestampMicros)
                            .range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "LineItem",
                    RelationKind::Entity,
                    vec![
                        id_field("LineItemId", "LineItem"),
                        ref_field("OrderId", "order", "Orders"),
                        ref_field("PartId", "part", "Part"),
                        ref_field("SupplierId", "supplier", "Supplier"),
                        FieldDescriptor::new("quantity", ValueType::I64),
                        FieldDescriptor::new("extended_price", ValueType::Decimal { scale: 2 }),
                        FieldDescriptor::new("ship_date", ValueType::TimestampMicros)
                            .range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
            ],
        ),
        rows: tpch_rows(n),
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
                datalog: r#"
                    find ?customer sum(?price)
                    where
                      Customer(id: ?customer, nation: $nation)
                      Orders(id: ?order, customer: ?customer)
                      LineItem(order: ?order, extended_price: ?price, ship_date: ?ship)
                      ?ship >= $start
                      ?ship < $end
                "#,
                inputs: vec![
                    ("nation", Value::Symbol(1)),
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
                datalog: r#"
                    find ?line ?order
                    where
                      Supplier(id: ?supplier, nation: $nation)
                      LineItem(id: ?line, order: ?order, supplier: ?supplier)
                      Orders(id: ?order, customer: ?customer)
                "#,
                inputs: vec![("nation", Value::Symbol(2))],
                sqlite: r#"
                    SELECT l.id, o.id FROM supplier s
                    JOIN lineitem l ON l.supplier = s.id
                    JOIN orders o ON o.id = l.ord
                    WHERE s.nation = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(2)],
            },
        ],
    }
}

fn sailors_rows(sailors: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    for sid in 1..=sailors {
        rows.push(Row::new(
            "Sailor",
            [
                ("id", Value::Id(sid)),
                ("name", Value::String(format!("sailor-{sid}"))),
                ("rating", Value::U64((sid % 10) + 1)),
                ("age", Value::I64(18 + (sid % 50) as i64)),
            ],
        ));
    }
    let boats = (sailors / 4).max(10);
    for bid in 1..=boats {
        rows.push(Row::new(
            "Boat",
            [
                ("id", Value::Id(bid)),
                ("name", Value::String(format!("boat-{bid}"))),
                ("color", Value::Symbol((bid % 3) + 1)),
            ],
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for sid in 1..=sailors {
        for offset in 0..5 {
            let bid = ((sid + offset * 7) % boats) + 1;
            let day = ((sid * 10 + offset) as i64) * 86_400;
            if seen.insert((sid, bid, day)) {
                rows.push(Row::new(
                    "Reserve",
                    [
                        ("sailor", Value::Ref(sid)),
                        ("boat", Value::Ref(bid)),
                        ("day", Value::Timestamp(TimestampMicros(day))),
                    ],
                ));
            }
        }
    }
    rows
}

fn join_stress_rows(n: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    for id in 1..=n {
        rows.push(Row::new(
            "A",
            [("id", Value::Id(id)), ("k", Value::Symbol(id % 10))],
        ));
        rows.push(Row::new(
            "B",
            [
                ("id", Value::Id(id)),
                ("a", Value::Ref(((id - 1) % n) + 1)),
                ("k", Value::Symbol(id % 10)),
            ],
        ));
        rows.push(Row::new(
            "C",
            [
                ("id", Value::Id(id)),
                ("b", Value::Ref(((id - 1) % n) + 1)),
                ("k", Value::Symbol(id % 10)),
            ],
        ));
        rows.push(Row::new(
            "D",
            [
                ("id", Value::Id(id)),
                ("c", Value::Ref(((id - 1) % n) + 1)),
                ("k", Value::Symbol(id % 10)),
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
                rows.push(Row::new(
                    "EdgeAB",
                    [("a", Value::Ref(a)), ("b", Value::Ref(b))],
                ));
            }
            if ac.insert((a, c)) {
                rows.push(Row::new(
                    "EdgeAC",
                    [("a", Value::Ref(a)), ("c", Value::Ref(c))],
                ));
            }
            if bc.insert((b, c)) {
                rows.push(Row::new(
                    "EdgeBC",
                    [("b", Value::Ref(b)), ("c", Value::Ref(c))],
                ));
            }
        }
    }
    rows
}

fn tpch_rows(n: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    for id in 1..=n {
        rows.push(Row::new(
            "Customer",
            [
                ("id", Value::Id(id)),
                ("nation", Value::Symbol((id % 5) + 1)),
            ],
        ));
        rows.push(Row::new(
            "Supplier",
            [
                ("id", Value::Id(id)),
                ("nation", Value::Symbol((id % 7) + 1)),
            ],
        ));
        rows.push(Row::new(
            "Part",
            [
                ("id", Value::Id(id)),
                ("brand", Value::Symbol((id % 11) + 1)),
            ],
        ));
        rows.push(Row::new(
            "Orders",
            [
                ("id", Value::Id(id)),
                ("customer", Value::Ref(((id - 1) % n) + 1)),
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
            rows.push(Row::new(
                "LineItem",
                [
                    ("id", Value::Id(line)),
                    ("order", Value::Ref(order)),
                    ("part", Value::Ref(((order + offset) % n) + 1)),
                    ("supplier", Value::Ref(((order + offset * 3) % n) + 1)),
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
    rows
}

fn insert_ledger_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Holder" => {
                tx.execute(
                    "INSERT INTO holder (id, name) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id"), text(row, "name")],
                )?;
            }
            "Account" => {
                tx.execute(
                    "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), rf(row, "holder"), symbol(row, "currency")],
                )?;
            }
            "Instrument" => {
                tx.execute(
                    "INSERT INTO instrument (id, symbol) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id"), text(row, "symbol")],
                )?;
            }
            "JournalEntry" => {
                tx.execute(
                    "INSERT INTO journal_entry (id, source, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), rf(row, "source"), ts(row, "created_at")],
                )?;
            }
            "Posting" => {
                tx.execute("INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(row, "id"), rf(row, "entry"), rf(row, "account"), rf(row, "instrument"), dec(row, "amount"), ts(row, "at")])?;
            }
            "PostingTag" => {
                tx.execute(
                    "INSERT INTO posting_tag (posting, tag) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "posting"), symbol(row, "tag")],
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
    rows: &[Row],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Sailor" => {
                tx.execute(
                    "INSERT INTO sailor (id, name, rating, age) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        id(row, "id"),
                        text(row, "name"),
                        u64v(row, "rating"),
                        i64v(row, "age")
                    ],
                )?;
            }
            "Boat" => {
                tx.execute(
                    "INSERT INTO boat (id, name, color) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), text(row, "name"), symbol(row, "color")],
                )?;
            }
            "Reserve" => {
                tx.execute(
                    "INSERT INTO reserve (sailor, boat, day) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(row, "sailor"), rf(row, "boat"), ts(row, "day")],
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
    rows: &[Row],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "A" => {
                tx.execute(
                    "INSERT INTO a (id, k) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id"), symbol(row, "k")],
                )?;
            }
            "B" => {
                tx.execute(
                    "INSERT INTO b (id, a, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), rf(row, "a"), symbol(row, "k")],
                )?;
            }
            "C" => {
                tx.execute(
                    "INSERT INTO c (id, b, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), rf(row, "b"), symbol(row, "k")],
                )?;
            }
            "D" => {
                tx.execute(
                    "INSERT INTO d (id, c, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), rf(row, "c"), symbol(row, "k")],
                )?;
            }
            "EdgeAB" => {
                tx.execute(
                    "INSERT INTO edge_ab (a, b) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "a"), rf(row, "b")],
                )?;
            }
            "EdgeAC" => {
                tx.execute(
                    "INSERT INTO edge_ac (a, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "a"), rf(row, "c")],
                )?;
            }
            "EdgeBC" => {
                tx.execute(
                    "INSERT INTO edge_bc (b, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "b"), rf(row, "c")],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_tpch_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Customer" => {
                tx.execute(
                    "INSERT INTO customer (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id"), symbol(row, "nation")],
                )?;
            }
            "Supplier" => {
                tx.execute(
                    "INSERT INTO supplier (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id"), symbol(row, "nation")],
                )?;
            }
            "Part" => {
                tx.execute(
                    "INSERT INTO part (id, brand) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id"), symbol(row, "brand")],
                )?;
            }
            "Orders" => {
                tx.execute(
                    "INSERT INTO orders (id, customer, order_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id"), rf(row, "customer"), ts(row, "order_date")],
                )?;
            }
            "LineItem" => {
                tx.execute("INSERT INTO lineitem (id, ord, part, supplier, quantity, extended_price, ship_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(row, "id"), rf(row, "order"), rf(row, "part"), rf(row, "supplier"), i64v(row, "quantity"), dec(row, "extended_price"), ts(row, "ship_date")])?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn id(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Id(v) => *v as i64,
        other => panic!("expected id {field}, got {other:?}"),
    }
}
pub(crate) fn rf(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Ref(v) => *v as i64,
        other => panic!("expected ref {field}, got {other:?}"),
    }
}
pub(crate) fn symbol(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Symbol(v) => *v as i64,
        other => panic!("expected symbol {field}, got {other:?}"),
    }
}
pub(crate) fn dec(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Decimal(DecimalRaw(v)) => *v as i64,
        other => panic!("expected decimal {field}, got {other:?}"),
    }
}
pub(crate) fn ts(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Timestamp(TimestampMicros(v)) => *v,
        other => panic!("expected timestamp {field}, got {other:?}"),
    }
}
pub(crate) fn u64v(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::U64(v) => *v as i64,
        other => panic!("expected u64 {field}, got {other:?}"),
    }
}
pub(crate) fn i64v(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::I64(v) => *v,
        other => panic!("expected i64 {field}, got {other:?}"),
    }
}
pub(crate) fn text(row: &Row, field: &str) -> String {
    match row.value(field).unwrap() {
        Value::String(v) => v.clone(),
        other => panic!("expected string {field}, got {other:?}"),
    }
}

pub(crate) fn id_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Id {
            name: id_type.to_owned(),
            relation: relation.to_owned(),
        },
    )
}

pub(crate) fn ref_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Ref {
            name: id_type.to_owned(),
            target_relation: target.to_owned(),
        },
    )
}
