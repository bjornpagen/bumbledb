fn ldbc_dataset(dir: &Path, limit: Option<usize>) -> Result<Dataset, Box<dyn std::error::Error>> {
    let person_file = find_prefixed(dir, "person")?;
    let post_file = find_prefixed(dir, "post")?;
    let knows_file = find_prefixed(dir, "person_knows_person")?;
    let likes_file = find_prefixed(dir, "person_likes_post")?;
    let mut facts = Vec::new();
    let mut people = BTreeSet::new();
    let mut posts = BTreeSet::new();

    read_pipe_path(&person_file, limit, |headers, record| {
        let id = parse_u64(col(headers, record, &["id", "Person.id"]));
        people.insert(id);
        facts.push(Fact::new(
            "Person",
            [
                ("id", Value::Serial(id)),
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
    read_pipe_path(&post_file, scaled_limit(limit, 2), |headers, record| {
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
        facts.push(Fact::new(
            "Post",
            [
                ("id", Value::Serial(id)),
                ("creator", Value::Serial(creator)),
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
    read_pipe_path(&knows_file, scaled_limit(limit, 4), |headers, record| {
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
        facts.push(Fact::new(
            "Knows",
            [
                ("person1", Value::Serial(p1)),
                ("person2", Value::Serial(p2)),
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
    read_pipe_path(&likes_file, scaled_limit(limit, 4), |headers, record| {
        let person = parse_u64(col(headers, record, &["Person.id", "personId"]));
        let post = parse_u64(col(headers, record, &["Post.id", "postId"]));
        if !(people.contains(&person) && posts.contains(&post)) {
            return Ok(());
        }
        facts.push(Fact::new(
            "Likes",
            [
                ("person", Value::Serial(person)),
                ("post", Value::Serial(post)),
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
    Ok(ldbc_from_facts(facts))
}

fn lahman_from_facts(facts: Vec<Fact>) -> Dataset {
    Dataset {
        name: "lahman",
        schema: SchemaDescriptor::new(
            "LahmanDb",
            vec![
                RelationDescriptor::new(
                    "Player",
                    vec![
                        serial_key_field("PlayerId", "Player"),
                        FieldDescriptor::new("first", ValueType::String),
                        FieldDescriptor::new("last", ValueType::String),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Team",
                    vec![
                        serial_key_field("TeamId", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("league", ValueType::String),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Batting",
                    vec![
                        serial_field("PlayerId", "player", "Player"),
                        serial_field("TeamId", "team", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("games", ValueType::I64),
                        FieldDescriptor::new("hits", ValueType::I64),
                    ],
                )
                .with_unique("player_team_year", ["player", "team", "year"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "player",
                    ["player"],
                    "Player",
                    "id",
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "team",
                    ["team"],
                    "Team",
                    "id",
                ))
                .with_index(IndexDescriptor::permutation(
                    "by_year",
                    ["year", "player", "team"],
                )),
                RelationDescriptor::new(
                    "Salary",
                    vec![
                        serial_field("PlayerId", "player", "Player"),
                        serial_field("TeamId", "team", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("salary", ValueType::I64),
                    ],
                )
                .with_unique("player_team_year", ["player", "team", "year"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "player",
                    ["player"],
                    "Player",
                    "id",
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "team",
                    ["team"],
                    "Team",
                    "id",
                ))
                .with_index(IndexDescriptor::permutation(
                    "by_year",
                    ["year", "player", "team"],
                )),
            ],
        ),
        facts,
        fact_source: None,
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
            build: build_lahman_salary_hits_by_year,
            inputs: vec![("year", Value::I64(2000))],
            sqlite: "SELECT DISTINCT s.player, s.salary, b.hits FROM salary s JOIN batting b ON b.player = s.player AND b.year = s.year WHERE s.year = ?1",
            sqlite_params: vec![SqlParam::I64(2000)],
        }],
    }
}

fn build_lahman_salary_hits_by_year(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Salary")?
        .var("player", "player")?
        .input("year", "year")?
        .var("salary", "salary")?
        .done()
        .rel("Batting")?
        .var("player", "player")?
        .input("year", "year")?
        .var("hits", "hits")?
        .done()
        .find_var("player")?
        .find_var("salary")?
        .find_var("hits")?
        .finish()
}

fn ldbc_from_facts(facts: Vec<Fact>) -> Dataset {
    Dataset {
        name: "ldbc",
        schema: SchemaDescriptor::new(
            "LdbcSubsetDb",
            vec![
                RelationDescriptor::new(
                    "Person",
                    vec![
                        serial_key_field("PersonId", "Person"),
                        FieldDescriptor::new("first", ValueType::String),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Post",
                    vec![
                        serial_key_field("PostId", "Post"),
                        serial_field("PersonId", "creator", "Person"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "creator",
                    ["creator"],
                    "Person",
                    "id",
                )),
                RelationDescriptor::new(
                    "Knows",
                    vec![
                        serial_field("PersonId", "person1", "Person"),
                        serial_field("PersonId", "person2", "Person"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                )
                .with_unique("person1_person2", ["person1", "person2"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "person1",
                    ["person1"],
                    "Person",
                    "id",
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "person2",
                    ["person2"],
                    "Person",
                    "id",
                ))
                .with_index(IndexDescriptor::permutation(
                    "by_person2_person1",
                    ["person2", "person1"],
                )),
                RelationDescriptor::new(
                    "Likes",
                    vec![
                        serial_field("PersonId", "person", "Person"),
                        serial_field("PostId", "post", "Post"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                )
                .with_unique("person_post", ["person", "post"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "person",
                    ["person"],
                    "Person",
                    "id",
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "post",
                    ["post"],
                    "Post",
                    "id",
                ))
                .with_index(IndexDescriptor::permutation(
                    "by_post_person",
                    ["post", "person"],
                )),
            ],
        ),
        facts,
        fact_source: None,
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
                build: build_ldbc_person_likes_posts,
                inputs: vec![("person", Value::Serial(1))],
                sqlite: "SELECT DISTINCT p.id FROM likes l JOIN post p ON p.id = l.post WHERE l.person = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "two_hop_knows",
                build: build_ldbc_two_hop_knows,
                inputs: vec![("person", Value::Serial(1))],
                sqlite: "SELECT DISTINCT k2.person2 FROM knows k1 JOIN knows k2 ON k2.person1 = k1.person2 WHERE k1.person1 = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
        ],
    }
}

fn build_ldbc_person_likes_posts(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Likes")?
        .input("person", "person")?
        .var("post", "post")?
        .done()
        .rel("Post")?
        .var("id", "post")?
        .var("creator", "creator")?
        .done()
        .find_var("post")?
        .finish()
}

fn build_ldbc_two_hop_knows(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Knows")?
        .input("person1", "person")?
        .var("person2", "friend1")?
        .done()
        .rel("Knows")?
        .var("person1", "friend1")?
        .var("person2", "friend2")?
        .done()
        .find_var("friend2")?
        .finish()
}

fn super_tpch_dataset() -> Dataset {
    crate::tpch_dataset(1)
}

fn scaled_limit(limit: Option<usize>, multiplier: usize) -> Option<usize> {
    limit.map(|limit| limit.saturating_mul(multiplier).max(limit))
}

fn reached_limit(count: usize, limit: Option<usize>) -> bool {
    limit.is_some_and(|limit| count >= limit)
}

