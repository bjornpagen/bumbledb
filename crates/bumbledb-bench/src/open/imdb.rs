fn imdb_dataset(dir: &Path, limit: Option<usize>) -> Result<Dataset, Box<dyn std::error::Error>> {
    let mut title_ids = BTreeMap::new();
    let mut name_ids = BTreeMap::new();
    let mut symbols = Symbols::default();
    let mut facts = Vec::new();

    let title_path = require_file(dir, "title.basics.tsv")?;
    let mut title_reader = tsv_reader(&title_path)?;
    for (read, record) in title_reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        let record = record?;
        let tconst = get(&record, 0);
        let id = (title_ids.len() + 1) as u64;
        title_ids.insert(tconst.to_owned(), id);
        facts.push(Fact::new(
            "Title",
            [
                ("id", Value::Serial(id)),
                ("title_type", Value::U64(symbols.id(get(&record, 1)))),
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
    for (read, record) in name_reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        let record = record?;
        let nconst = get(&record, 0);
        let id = (name_ids.len() + 1) as u64;
        name_ids.insert(nconst.to_owned(), id);
        facts.push(Fact::new(
            "Name",
            [
                ("id", Value::Serial(id)),
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
        facts.push(Fact::new(
            "TitleRating",
            [
                ("title", Value::Serial(title)),
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
        facts.push(Fact::new(
            "Principal",
            [
                ("title", Value::Serial(title)),
                ("name", Value::Serial(name)),
                ("category", Value::U64(category)),
                ("ordering", Value::U64(parse_optional_u64(get(&record, 1)))),
            ],
        ));
    }

    Ok(Dataset {
        name: "imdb",
        schema: imdb_schema(),
        facts,
        fact_source: None,
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
                build: build_imdb_person_high_rated_titles,
                inputs: vec![
                    ("name", Value::Serial(sample_name)),
                    ("category", Value::U64(sample_category)),
                    ("min_rating", Value::I64(70)),
                ],
                sqlite: r#"
                    SELECT DISTINCT p.title, r.rating FROM principal p
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
                build: build_imdb_category_rating_join,
                inputs: vec![
                    ("category", Value::U64(sample_category)),
                    ("min_rating", Value::I64(80)),
                ],
                sqlite: r#"
                    SELECT DISTINCT p.title, p.name FROM principal p
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
                vec![
                    serial_key_field("TitleId", "Title"),
                    FieldDescriptor::new("title_type", ValueType::U64),
                    FieldDescriptor::new("primary_title", ValueType::String),
                    FieldDescriptor::new("start_year", ValueType::I64).range_indexed(),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Name",
                vec![
                    serial_key_field("NameId", "Name"),
                    FieldDescriptor::new("name", ValueType::String),
                    FieldDescriptor::new("birth_year", ValueType::I64).range_indexed(),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "TitleRating",
                vec![
                    serial_field("TitleId", "title", "Title"),
                    FieldDescriptor::new("rating", ValueType::I64).range_indexed(),
                    FieldDescriptor::new("votes", ValueType::I64),
                ],
            )
            .with_unique("title", ["title"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "title",
                ["title"],
                "Title",
                "id",
            )),
            RelationDescriptor::new(
                "Principal",
                vec![
                    serial_field("TitleId", "title", "Title"),
                    serial_field("NameId", "name", "Name"),
                    FieldDescriptor::new("category", ValueType::U64),
                    FieldDescriptor::new("ordering", ValueType::U64),
                ],
            )
            .with_unique(
                "title_name_category_ordering",
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
            ))
            .with_index(IndexDescriptor::permutation(
                "by_category",
                ["category", "title", "name"],
            )),
        ],
    )
}

fn build_imdb_person_high_rated_titles(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Principal")?
        .input("name", "name")?
        .var("title", "title")?
        .input("category", "category")?
        .done()
        .rel("TitleRating")?
        .var("title", "title")?
        .var("rating", "rating")?
        .done()
        .cmp(
            OperandRef::var("rating"),
            ComparisonOperator::Gte,
            OperandRef::input("min_rating"),
        )?
        .find_var("title")?
        .find_var("rating")?
        .finish()
}

fn build_imdb_category_rating_join(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Principal")?
        .var("title", "title")?
        .var("name", "name")?
        .input("category", "category")?
        .done()
        .rel("TitleRating")?
        .var("title", "title")?
        .var("rating", "rating")?
        .done()
        .cmp(
            OperandRef::var("rating"),
            ComparisonOperator::Gte,
            OperandRef::input("min_rating"),
        )?
        .find_var("title")?
        .find_var("name")?
        .finish()
}

