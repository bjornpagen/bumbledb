use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{
    FieldDescriptor, IndexDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind,
    SchemaDescriptor, ValueType,
};
use bumbledb_lmdb::{Row, Value};
use csv::{ReaderBuilder, StringRecord};
use rusqlite::Connection;

use crate::{
    BenchQuery, Config, Dataset, SqlParam, i64v, id, id_field, ref_field, rf, symbol, text, ts,
    u64v,
};

pub(crate) fn open_datasets(config: &Config) -> Result<Vec<Dataset>, Box<dyn std::error::Error>> {
    let mut datasets = Vec::new();
    if let Some(path) = &config.imdb_dir {
        datasets.push(imdb_dataset(Path::new(path), config.scale)?);
    }
    if let Some(path) = &config.tpch_dir {
        datasets.push(tpch_open_dataset(Path::new(path), config.scale)?);
    }
    if let Some(path) = &config.lahman_dir {
        datasets.push(lahman_dataset(Path::new(path), config.scale)?);
    }
    if let Some(path) = &config.ldbc_dir {
        datasets.push(ldbc_dataset(Path::new(path), config.scale)?);
    }
    Ok(datasets)
}

fn imdb_dataset(dir: &Path, scale: u64) -> Result<Dataset, Box<dyn std::error::Error>> {
    let limit = scale.max(1) as usize;
    let mut title_ids = BTreeMap::new();
    let mut name_ids = BTreeMap::new();
    let mut symbols = Symbols::default();
    let mut rows = Vec::new();

    let title_path = require_file(dir, "title.basics.tsv")?;
    let mut title_reader = tsv_reader(&title_path)?;
    for record in title_reader.records().take(limit) {
        let record = record?;
        let tconst = get(&record, 0);
        let id = (title_ids.len() + 1) as u64;
        title_ids.insert(tconst.to_owned(), id);
        rows.push(Row::new(
            "Title",
            [
                ("id", Value::Id(id)),
                ("title_type", Value::Symbol(symbols.id(get(&record, 1)))),
                ("primary_title", Value::String(get(&record, 2).to_owned())),
                (
                    "start_year",
                    Value::I64(parse_optional_i64(get(&record, 5))),
                ),
            ],
        ));
    }

    let name_path = require_file(dir, "name.basics.tsv")?;
    let mut name_reader = tsv_reader(&name_path)?;
    for record in name_reader.records().take(limit) {
        let record = record?;
        let nconst = get(&record, 0);
        let id = (name_ids.len() + 1) as u64;
        name_ids.insert(nconst.to_owned(), id);
        rows.push(Row::new(
            "Name",
            [
                ("id", Value::Id(id)),
                ("name", Value::String(get(&record, 1).to_owned())),
                (
                    "birth_year",
                    Value::I64(parse_optional_i64(get(&record, 2))),
                ),
            ],
        ));
    }

    let ratings_path = require_file(dir, "title.ratings.tsv")?;
    let mut ratings_reader = tsv_reader(&ratings_path)?;
    for record in ratings_reader.records() {
        let record = record?;
        let Some(title) = title_ids.get(get(&record, 0)).copied() else {
            continue;
        };
        rows.push(Row::new(
            "TitleRating",
            [
                ("title", Value::Ref(title)),
                ("rating", Value::I64(parse_rating_x10(get(&record, 1)))),
                ("votes", Value::I64(parse_optional_i64(get(&record, 2)))),
            ],
        ));
    }

    let mut sample_name = 1;
    let mut sample_category = symbols.id("actor");
    let principals_path = require_file(dir, "title.principals.tsv")?;
    let mut principals_reader = tsv_reader(&principals_path)?;
    for record in principals_reader.records() {
        let record = record?;
        let Some(title) = title_ids.get(get(&record, 0)).copied() else {
            continue;
        };
        let Some(name) = name_ids.get(get(&record, 2)).copied() else {
            continue;
        };
        let category = symbols.id(get(&record, 3));
        sample_name = name;
        sample_category = category;
        rows.push(Row::new(
            "Principal",
            [
                ("title", Value::Ref(title)),
                ("name", Value::Ref(name)),
                ("category", Value::Symbol(category)),
                ("ordering", Value::U64(parse_optional_u64(get(&record, 1)))),
            ],
        ));
    }

    Ok(Dataset {
        name: "imdb",
        schema: imdb_schema(),
        rows,
        sqlite_schema: r#"
            CREATE TABLE title (id INTEGER PRIMARY KEY, title_type INTEGER NOT NULL, primary_title TEXT NOT NULL, start_year INTEGER NOT NULL);
            CREATE TABLE name (id INTEGER PRIMARY KEY, name TEXT NOT NULL, birth_year INTEGER NOT NULL);
            CREATE TABLE title_rating (title INTEGER PRIMARY KEY, rating INTEGER NOT NULL, votes INTEGER NOT NULL);
            CREATE TABLE principal (title INTEGER NOT NULL, name INTEGER NOT NULL, category INTEGER NOT NULL, ordering INTEGER NOT NULL, PRIMARY KEY (title, name, category, ordering));
            CREATE INDEX principal_name ON principal(name, title);
            CREATE INDEX principal_category ON principal(category, title);
            CREATE INDEX rating_rating ON title_rating(rating, title);
        "#,
        sqlite_insert: insert_imdb_sqlite,
        queries: vec![
            BenchQuery {
                name: "person_high_rated_titles",
                datalog: r#"
                    find ?title ?rating
                    where
                      Principal(name: $name, title: ?title, category: $category)
                      TitleRating(title: ?title, rating: ?rating)
                      ?rating >= $min_rating
                "#,
                inputs: vec![
                    ("name", Value::Ref(sample_name)),
                    ("category", Value::Symbol(sample_category)),
                    ("min_rating", Value::I64(70)),
                ],
                sqlite: r#"
                    SELECT p.title, r.rating FROM principal p
                    JOIN title_rating r ON r.title = p.title
                    WHERE p.name = ?1 AND p.category = ?2 AND r.rating >= ?3
                "#,
                sqlite_params: vec![
                    SqlParam::I64(sample_name as i64),
                    SqlParam::I64(sample_category as i64),
                    SqlParam::I64(70),
                ],
            },
            BenchQuery {
                name: "category_rating_join",
                datalog: r#"
                    find ?title ?name
                    where
                      Principal(title: ?title, name: ?name, category: $category)
                      TitleRating(title: ?title, rating: ?rating)
                      ?rating >= $min_rating
                "#,
                inputs: vec![
                    ("category", Value::Symbol(sample_category)),
                    ("min_rating", Value::I64(80)),
                ],
                sqlite: r#"
                    SELECT p.title, p.name FROM principal p
                    JOIN title_rating r ON r.title = p.title
                    WHERE p.category = ?1 AND r.rating >= ?2
                "#,
                sqlite_params: vec![SqlParam::I64(sample_category as i64), SqlParam::I64(80)],
            },
        ],
    })
}

fn imdb_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "ImdbPublicDb",
        vec![
            RelationDescriptor::new(
                "Title",
                RelationKind::Entity,
                vec![
                    id_field("TitleId", "Title"),
                    FieldDescriptor::new(
                        "title_type",
                        ValueType::Symbol {
                            name: "TitleType".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("primary_title", ValueType::String),
                    FieldDescriptor::new("start_year", ValueType::I64).range_indexed(),
                ],
                bumbledb_core::schema::PrimaryKeyDescriptor::new(["id"]),
            ),
            RelationDescriptor::new(
                "Name",
                RelationKind::Entity,
                vec![
                    id_field("NameId", "Name"),
                    FieldDescriptor::new("name", ValueType::String),
                    FieldDescriptor::new("birth_year", ValueType::I64).range_indexed(),
                ],
                bumbledb_core::schema::PrimaryKeyDescriptor::new(["id"]),
            ),
            RelationDescriptor::new(
                "TitleRating",
                RelationKind::Entity,
                vec![
                    ref_field("TitleId", "title", "Title"),
                    FieldDescriptor::new("rating", ValueType::I64).range_indexed(),
                    FieldDescriptor::new("votes", ValueType::I64),
                ],
                bumbledb_core::schema::PrimaryKeyDescriptor::new(["title"]),
            ),
            RelationDescriptor::new(
                "Principal",
                RelationKind::Edge,
                vec![
                    ref_field("TitleId", "title", "Title"),
                    ref_field("NameId", "name", "Name"),
                    FieldDescriptor::new(
                        "category",
                        ValueType::Symbol {
                            name: "Category".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("ordering", ValueType::U64),
                ],
                bumbledb_core::schema::PrimaryKeyDescriptor::new([
                    "title", "name", "category", "ordering",
                ]),
            )
            .with_index(IndexDescriptor::permutation(
                "by_category",
                ["category", "title", "name"],
            )),
        ],
    )
}

fn tpch_open_dataset(dir: &Path, scale: u64) -> Result<Dataset, Box<dyn std::error::Error>> {
    let limit = scale.max(1) as usize;
    let mut rows = Vec::new();
    let mut customers = BTreeSet::new();
    let mut suppliers = BTreeSet::new();
    let mut parts = BTreeSet::new();
    let mut orders = BTreeSet::new();
    read_pipe(dir, "customer.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        customers.insert(id);
        rows.push(Row::new(
            "Customer",
            [
                ("id", Value::Id(id)),
                ("nation", Value::Symbol(parse_u64(get(&record, 3)))),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "supplier.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        suppliers.insert(id);
        rows.push(Row::new(
            "Supplier",
            [
                ("id", Value::Id(id)),
                ("nation", Value::Symbol(parse_u64(get(&record, 3)))),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "part.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        parts.insert(id);
        rows.push(Row::new(
            "Part",
            [
                ("id", Value::Id(id)),
                ("brand", Value::String(get(&record, 3).to_owned())),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "orders.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        let customer = parse_u64(get(&record, 1));
        if !customers.contains(&customer) {
            return Ok(());
        }
        orders.insert(id);
        rows.push(Row::new(
            "Orders",
            [
                ("id", Value::Id(id)),
                ("customer", Value::Ref(customer)),
                (
                    "order_date",
                    Value::Timestamp(TimestampMicros(parse_date(get(&record, 4)))),
                ),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "lineitem.tbl", limit * 4, |record| {
        let order = parse_u64(get(&record, 0));
        let part = parse_u64(get(&record, 1));
        let supplier = parse_u64(get(&record, 2));
        if !(orders.contains(&order) && parts.contains(&part) && suppliers.contains(&supplier)) {
            return Ok(());
        }
        rows.push(Row::new(
            "LineItem",
            [
                ("id", Value::Id(rows.len() as u64 + 1)),
                ("order", Value::Ref(order)),
                ("part", Value::Ref(part)),
                ("supplier", Value::Ref(supplier)),
                ("quantity", Value::I64(parse_decimal_i64(get(&record, 4)))),
                (
                    "extended_price",
                    Value::Decimal(DecimalRaw(parse_decimal_i128(get(&record, 5)))),
                ),
                (
                    "ship_date",
                    Value::Timestamp(TimestampMicros(parse_date(get(&record, 10)))),
                ),
            ],
        ));
        Ok(())
    })?;

    let mut dataset = super_tpch_dataset();
    dataset.name = "tpch-open";
    dataset.rows = rows;
    Ok(dataset)
}

fn lahman_dataset(dir: &Path, scale: u64) -> Result<Dataset, Box<dyn std::error::Error>> {
    let limit = scale.max(1) as usize;
    let mut player_ids = BTreeMap::new();
    let mut team_ids = BTreeMap::new();
    let mut rows = Vec::new();

    read_csv(dir, "People.csv", limit, |headers, record| {
        let player_id = col(headers, record, &["playerID"]);
        let id = (player_ids.len() + 1) as u64;
        player_ids.insert(player_id.to_owned(), id);
        rows.push(Row::new(
            "Player",
            [
                ("id", Value::Id(id)),
                (
                    "first",
                    Value::String(col(headers, record, &["nameFirst"]).to_owned()),
                ),
                (
                    "last",
                    Value::String(col(headers, record, &["nameLast"]).to_owned()),
                ),
            ],
        ));
        Ok(())
    })?;

    read_csv(dir, "Teams.csv", limit * 4, |headers, record| {
        let key = format!(
            "{}:{}",
            col(headers, record, &["yearID"]),
            col(headers, record, &["teamID"])
        );
        let id = (team_ids.len() + 1) as u64;
        team_ids.insert(key, id);
        rows.push(Row::new(
            "Team",
            [
                ("id", Value::Id(id)),
                (
                    "year",
                    Value::I64(parse_optional_i64(col(headers, record, &["yearID"]))),
                ),
                (
                    "league",
                    Value::String(col(headers, record, &["lgID"]).to_owned()),
                ),
                (
                    "name",
                    Value::String(col(headers, record, &["name"]).to_owned()),
                ),
            ],
        ));
        Ok(())
    })?;

    read_csv(dir, "Batting.csv", limit * 10, |headers, record| {
        let player_key = col(headers, record, &["playerID"]);
        let team_key = format!(
            "{}:{}",
            col(headers, record, &["yearID"]),
            col(headers, record, &["teamID"])
        );
        let (Some(player), Some(team)) = (
            player_ids.get(player_key).copied(),
            team_ids.get(&team_key).copied(),
        ) else {
            return Ok(());
        };
        rows.push(Row::new(
            "Batting",
            [
                ("player", Value::Ref(player)),
                ("team", Value::Ref(team)),
                (
                    "year",
                    Value::I64(parse_optional_i64(col(headers, record, &["yearID"]))),
                ),
                (
                    "games",
                    Value::I64(parse_optional_i64(col(headers, record, &["G"]))),
                ),
                (
                    "hits",
                    Value::I64(parse_optional_i64(col(headers, record, &["H"]))),
                ),
            ],
        ));
        Ok(())
    })?;

    read_csv(dir, "Salaries.csv", limit * 4, |headers, record| {
        let player_key = col(headers, record, &["playerID"]);
        let team_key = format!(
            "{}:{}",
            col(headers, record, &["yearID"]),
            col(headers, record, &["teamID"])
        );
        let (Some(player), Some(team)) = (
            player_ids.get(player_key).copied(),
            team_ids.get(&team_key).copied(),
        ) else {
            return Ok(());
        };
        rows.push(Row::new(
            "Salary",
            [
                ("player", Value::Ref(player)),
                ("team", Value::Ref(team)),
                (
                    "year",
                    Value::I64(parse_optional_i64(col(headers, record, &["yearID"]))),
                ),
                (
                    "salary",
                    Value::I64(parse_optional_i64(col(headers, record, &["salary"]))),
                ),
            ],
        ));
        Ok(())
    })?;

    Ok(lahman_from_rows(rows))
}

fn ldbc_dataset(dir: &Path, scale: u64) -> Result<Dataset, Box<dyn std::error::Error>> {
    let limit = scale.max(1) as usize;
    let person_file = find_prefixed(dir, "person")?;
    let post_file = find_prefixed(dir, "post")?;
    let knows_file = find_prefixed(dir, "person_knows_person")?;
    let likes_file = find_prefixed(dir, "person_likes_post")?;
    let mut rows = Vec::new();
    let mut people = BTreeSet::new();
    let mut posts = BTreeSet::new();

    read_pipe_path(&person_file, limit, |headers, record| {
        let id = parse_u64(col(headers, record, &["id", "Person.id"]));
        people.insert(id);
        rows.push(Row::new(
            "Person",
            [
                ("id", Value::Id(id)),
                (
                    "first",
                    Value::String(col(headers, record, &["firstName", "first_name"]).to_owned()),
                ),
                (
                    "created",
                    Value::Timestamp(TimestampMicros(parse_ldbc_time(col(
                        headers,
                        record,
                        &["creationDate"],
                    )))),
                ),
            ],
        ));
        Ok(())
    })?;
    read_pipe_path(&post_file, limit * 2, |headers, record| {
        let id = parse_u64(col(headers, record, &["id", "Post.id"]));
        let creator = parse_u64(col(
            headers,
            record,
            &["creator.id", "Person.id", "personId"],
        ));
        if !people.contains(&creator) {
            return Ok(());
        }
        posts.insert(id);
        rows.push(Row::new(
            "Post",
            [
                ("id", Value::Id(id)),
                ("creator", Value::Ref(creator)),
                (
                    "created",
                    Value::Timestamp(TimestampMicros(parse_ldbc_time(col(
                        headers,
                        record,
                        &["creationDate"],
                    )))),
                ),
            ],
        ));
        Ok(())
    })?;
    read_pipe_path(&knows_file, limit * 4, |headers, record| {
        let p1 = parse_u64(col(headers, record, &["Person.id", "person1Id", "person1"]));
        let p2 = parse_u64(col_n(
            headers,
            record,
            &["Person.id", "person2Id", "person2"],
            1,
        ));
        if !(people.contains(&p1) && people.contains(&p2)) {
            return Ok(());
        }
        rows.push(Row::new(
            "Knows",
            [
                ("person1", Value::Ref(p1)),
                ("person2", Value::Ref(p2)),
                (
                    "created",
                    Value::Timestamp(TimestampMicros(parse_ldbc_time(col(
                        headers,
                        record,
                        &["creationDate"],
                    )))),
                ),
            ],
        ));
        Ok(())
    })?;
    read_pipe_path(&likes_file, limit * 4, |headers, record| {
        let person = parse_u64(col(headers, record, &["Person.id", "personId"]));
        let post = parse_u64(col(headers, record, &["Post.id", "postId"]));
        if !(people.contains(&person) && posts.contains(&post)) {
            return Ok(());
        }
        rows.push(Row::new(
            "Likes",
            [
                ("person", Value::Ref(person)),
                ("post", Value::Ref(post)),
                (
                    "created",
                    Value::Timestamp(TimestampMicros(parse_ldbc_time(col(
                        headers,
                        record,
                        &["creationDate"],
                    )))),
                ),
            ],
        ));
        Ok(())
    })?;
    Ok(ldbc_from_rows(rows))
}

fn lahman_from_rows(rows: Vec<Row>) -> Dataset {
    Dataset {
        name: "lahman",
        schema: SchemaDescriptor::new(
            "LahmanDb",
            vec![
                RelationDescriptor::new(
                    "Player",
                    RelationKind::Entity,
                    vec![
                        id_field("PlayerId", "Player"),
                        FieldDescriptor::new("first", ValueType::String),
                        FieldDescriptor::new("last", ValueType::String),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Team",
                    RelationKind::Entity,
                    vec![
                        id_field("TeamId", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("league", ValueType::String),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Batting",
                    RelationKind::Edge,
                    vec![
                        ref_field("PlayerId", "player", "Player"),
                        ref_field("TeamId", "team", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("games", ValueType::I64),
                        FieldDescriptor::new("hits", ValueType::I64),
                    ],
                    PrimaryKeyDescriptor::new(["player", "team", "year"]),
                )
                .with_index(IndexDescriptor::permutation(
                    "by_year",
                    ["year", "player", "team"],
                )),
                RelationDescriptor::new(
                    "Salary",
                    RelationKind::Edge,
                    vec![
                        ref_field("PlayerId", "player", "Player"),
                        ref_field("TeamId", "team", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("salary", ValueType::I64),
                    ],
                    PrimaryKeyDescriptor::new(["player", "team", "year"]),
                )
                .with_index(IndexDescriptor::permutation(
                    "by_year",
                    ["year", "player", "team"],
                )),
            ],
        ),
        rows,
        sqlite_schema: r#"
            CREATE TABLE player (id INTEGER PRIMARY KEY, first TEXT NOT NULL, last TEXT NOT NULL);
            CREATE TABLE team (id INTEGER PRIMARY KEY, year INTEGER NOT NULL, league TEXT NOT NULL, name TEXT NOT NULL);
            CREATE TABLE batting (player INTEGER NOT NULL, team INTEGER NOT NULL, year INTEGER NOT NULL, games INTEGER NOT NULL, hits INTEGER NOT NULL, PRIMARY KEY(player, team, year));
            CREATE TABLE salary (player INTEGER NOT NULL, team INTEGER NOT NULL, year INTEGER NOT NULL, salary INTEGER NOT NULL, PRIMARY KEY(player, team, year));
            CREATE INDEX batting_year ON batting(year, player);
            CREATE INDEX salary_year ON salary(year, player);
            CREATE INDEX batting_player ON batting(player, year);
            CREATE INDEX salary_player ON salary(player, year);
        "#,
        sqlite_insert: insert_lahman_sqlite,
        queries: vec![BenchQuery {
            name: "salary_hits_by_year",
            datalog: r#"
                    find ?player ?salary ?hits
                    where
                      Salary(player: ?player, year: $year, salary: ?salary)
                      Batting(player: ?player, year: $year, hits: ?hits)
                "#,
            inputs: vec![("year", Value::I64(2000))],
            sqlite: "SELECT s.player, s.salary, b.hits FROM salary s JOIN batting b ON b.player = s.player AND b.year = s.year WHERE s.year = ?1",
            sqlite_params: vec![SqlParam::I64(2000)],
        }],
    }
}

fn ldbc_from_rows(rows: Vec<Row>) -> Dataset {
    Dataset {
        name: "ldbc",
        schema: SchemaDescriptor::new(
            "LdbcSubsetDb",
            vec![
                RelationDescriptor::new(
                    "Person",
                    RelationKind::Entity,
                    vec![
                        id_field("PersonId", "Person"),
                        FieldDescriptor::new("first", ValueType::String),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Post",
                    RelationKind::Entity,
                    vec![
                        id_field("PostId", "Post"),
                        ref_field("PersonId", "creator", "Person"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Knows",
                    RelationKind::Edge,
                    vec![
                        ref_field("PersonId", "person1", "Person"),
                        ref_field("PersonId", "person2", "Person"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["person1", "person2"]),
                )
                .with_index(IndexDescriptor::permutation(
                    "by_person2",
                    ["person2", "person1"],
                )),
                RelationDescriptor::new(
                    "Likes",
                    RelationKind::Edge,
                    vec![
                        ref_field("PersonId", "person", "Person"),
                        ref_field("PostId", "post", "Post"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["person", "post"]),
                )
                .with_index(IndexDescriptor::permutation("by_post", ["post", "person"])),
            ],
        ),
        rows,
        sqlite_schema: r#"
            CREATE TABLE person (id INTEGER PRIMARY KEY, first TEXT NOT NULL, created INTEGER NOT NULL);
            CREATE TABLE post (id INTEGER PRIMARY KEY, creator INTEGER NOT NULL, created INTEGER NOT NULL);
            CREATE TABLE knows (person1 INTEGER NOT NULL, person2 INTEGER NOT NULL, created INTEGER NOT NULL, PRIMARY KEY(person1, person2));
            CREATE TABLE likes (person INTEGER NOT NULL, post INTEGER NOT NULL, created INTEGER NOT NULL, PRIMARY KEY(person, post));
            CREATE INDEX post_creator ON post(creator, id);
            CREATE INDEX knows_p1 ON knows(person1, person2);
            CREATE INDEX knows_p2 ON knows(person2, person1);
            CREATE INDEX likes_person ON likes(person, post);
            CREATE INDEX likes_post ON likes(post, person);
        "#,
        sqlite_insert: insert_ldbc_sqlite,
        queries: vec![
            BenchQuery {
                name: "person_likes_posts",
                datalog: r#"
                    find ?post
                    where
                      Likes(person: $person, post: ?post)
                      Post(id: ?post, creator: ?creator)
                "#,
                inputs: vec![("person", Value::Ref(1))],
                sqlite: "SELECT p.id FROM likes l JOIN post p ON p.id = l.post WHERE l.person = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "two_hop_knows",
                datalog: r#"
                    find ?friend2
                    where
                      Knows(person1: $person, person2: ?friend1)
                      Knows(person1: ?friend1, person2: ?friend2)
                "#,
                inputs: vec![("person", Value::Ref(1))],
                sqlite: "SELECT k2.person2 FROM knows k1 JOIN knows k2 ON k2.person1 = k1.person2 WHERE k1.person1 = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
        ],
    }
}

fn super_tpch_dataset() -> Dataset {
    crate::tpch_dataset(1)
}

fn read_csv(
    dir: &Path,
    file: &str,
    limit: usize,
    mut f: impl FnMut(&StringRecord, &StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    for record in reader.records().take(limit) {
        f(&headers, &record?)?;
    }
    Ok(())
}

fn read_pipe(
    dir: &Path,
    file: &str,
    limit: usize,
    mut f: impl FnMut(StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    let mut reader = ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .flexible(true)
        .from_path(path)?;
    for record in reader.records().take(limit) {
        f(record?)?;
    }
    Ok(())
}

fn read_pipe_path(
    path: &Path,
    limit: usize,
    mut f: impl FnMut(&StringRecord, &StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .delimiter(b'|')
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    for record in reader.records().take(limit) {
        f(&headers, &record?)?;
    }
    Ok(())
}

fn tsv_reader(path: &Path) -> Result<csv::Reader<std::fs::File>, Box<dyn std::error::Error>> {
    Ok(ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .from_path(path)?)
}

fn require_file(dir: &Path, file: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(file);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("missing required dataset file {}", path.display()).into())
    }
}

fn find_prefixed(dir: &Path, prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".csv")
            && (name == format!("{prefix}.csv") || name.starts_with(&format!("{prefix}_")))
        {
            candidates.push(path);
        }
    }
    candidates.sort();
    if let Some(path) = candidates.into_iter().next() {
        return Ok(path);
    }
    Err(format!(
        "missing LDBC file with prefix {prefix} in {}",
        dir.display()
    )
    .into())
}

fn get(record: &StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or("")
}

fn col<'a>(headers: &StringRecord, record: &'a StringRecord, names: &[&str]) -> &'a str {
    col_n(headers, record, names, 0)
}

fn col_n<'a>(
    headers: &StringRecord,
    record: &'a StringRecord,
    names: &[&str],
    occurrence: usize,
) -> &'a str {
    for name in names {
        let mut seen = 0;
        for (index, header) in headers.iter().enumerate() {
            if header == *name {
                if seen == occurrence {
                    return record.get(index).unwrap_or("");
                }
                seen += 1;
            }
        }
    }
    ""
}

fn parse_optional_i64(value: &str) -> i64 {
    if value.is_empty() || value == r"\N" {
        0
    } else {
        value.parse().unwrap_or(0)
    }
}

fn parse_optional_u64(value: &str) -> u64 {
    if value.is_empty() || value == r"\N" {
        0
    } else {
        value.parse().unwrap_or(0)
    }
}

fn parse_u64(value: &str) -> u64 {
    value.parse().unwrap_or(0)
}

fn parse_rating_x10(value: &str) -> i64 {
    (value.parse::<f64>().unwrap_or(0.0) * 10.0).round() as i64
}

fn parse_decimal_i64(value: &str) -> i64 {
    value.split('.').next().unwrap_or("0").parse().unwrap_or(0)
}

fn parse_decimal_i128(value: &str) -> i128 {
    (value.parse::<f64>().unwrap_or(0.0) * 100.0).round() as i128
}

fn parse_date(value: &str) -> i64 {
    let mut parts = value.split('-');
    let y = parts.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
    let m = parts.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
    let d = parts.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
    y * 10_000 + m * 100 + d
}

fn parse_ldbc_time(value: &str) -> i64 {
    if value.len() >= 10 {
        parse_date(&value[..10])
    } else {
        parse_optional_i64(value)
    }
}

#[derive(Default)]
struct Symbols {
    ids: BTreeMap<String, u64>,
}

impl Symbols {
    fn id(&mut self, value: &str) -> u64 {
        if let Some(id) = self.ids.get(value) {
            *id
        } else {
            let id = self.ids.len() as u64 + 1;
            self.ids.insert(value.to_owned(), id);
            id
        }
    }
}

fn insert_imdb_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Title" => {
                tx.execute("INSERT INTO title (id, title_type, primary_title, start_year) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id(row, "id")?, symbol(row, "title_type")?, text(row, "primary_title")?, i64v(row, "start_year")?])?;
            }
            "Name" => {
                tx.execute(
                    "INSERT INTO name (id, name, birth_year) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, text(row, "name")?, i64v(row, "birth_year")?],
                )?;
            }
            "TitleRating" => {
                tx.execute(
                    "INSERT INTO title_rating (title, rating, votes) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(row, "title")?, i64v(row, "rating")?, i64v(row, "votes")?],
                )?;
            }
            "Principal" => {
                tx.execute("INSERT INTO principal (title, name, category, ordering) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![rf(row, "title")?, rf(row, "name")?, symbol(row, "category")?, u64v(row, "ordering")?])?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_lahman_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Player" => {
                tx.execute(
                    "INSERT INTO player (id, first, last) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, text(row, "first")?, text(row, "last")?],
                )?;
            }
            "Team" => {
                tx.execute(
                    "INSERT INTO team (id, year, league, name) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        id(row, "id")?,
                        i64v(row, "year")?,
                        text(row, "league")?,
                        text(row, "name")?
                    ],
                )?;
            }
            "Batting" => {
                tx.execute("INSERT INTO batting (player, team, year, games, hits) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![rf(row, "player")?, rf(row, "team")?, i64v(row, "year")?, i64v(row, "games")?, i64v(row, "hits")?])?;
            }
            "Salary" => {
                tx.execute(
                    "INSERT INTO salary (player, team, year, salary) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        rf(row, "player")?,
                        rf(row, "team")?,
                        i64v(row, "year")?,
                        i64v(row, "salary")?
                    ],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_ldbc_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Person" => {
                tx.execute(
                    "INSERT INTO person (id, first, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, text(row, "first")?, ts(row, "created")?],
                )?;
            }
            "Post" => {
                tx.execute(
                    "INSERT INTO post (id, creator, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "creator")?, ts(row, "created")?],
                )?;
            }
            "Knows" => {
                tx.execute(
                    "INSERT OR IGNORE INTO knows (person1, person2, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        rf(row, "person1")?,
                        rf(row, "person2")?,
                        ts(row, "created")?
                    ],
                )?;
            }
            "Likes" => {
                tx.execute(
                    "INSERT OR IGNORE INTO likes (person, post, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(row, "person")?, rf(row, "post")?, ts(row, "created")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}
