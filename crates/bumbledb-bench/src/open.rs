use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, Literal, TypedQuery};
use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};
use bumbledb_lmdb::{Fact, Value};
use csv::{ReaderBuilder, StringRecord};
use rusqlite::Connection;

use crate::{
    BenchQuery, Config, Dataset, SqlParam, i64v, id, rf, serial_field, serial_key_field, symbol,
    text, ts, u64v,
};

#[derive(Clone, Debug)]
pub(crate) enum FactSource {
    Job { dir: PathBuf, limit: Option<usize> },
}

pub(crate) fn stream_facts(
    source: &FactSource,
    emit: impl FnMut(Fact) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<usize, Box<dyn std::error::Error>> {
    match source {
        FactSource::Job { dir, limit } => stream_job_facts(dir, *limit, emit),
    }
}

pub(crate) fn insert_sqlite_streaming(
    source: &FactSource,
    conn: &mut Connection,
) -> Result<usize, Box<dyn std::error::Error>> {
    match source {
        FactSource::Job { dir, limit } => {
            let tx = conn.unchecked_transaction()?;
            let inserted = stream_job_facts(dir, *limit, |fact| {
                insert_job_sqlite_fact(&tx, &fact)?;
                Ok(())
            })?;
            tx.commit()?;
            Ok(inserted)
        }
    }
}

pub(crate) fn open_datasets(config: &Config) -> Result<Vec<Dataset>, Box<dyn std::error::Error>> {
    let mut datasets = Vec::new();
    if let Some(path) = &config.imdb_dir {
        datasets.push(imdb_dataset(Path::new(path), config.open_limit)?);
    }
    if let Some(path) = &config.job_dir {
        datasets.push(job_dataset(Path::new(path), config.open_limit)?);
    }
    if let Some(path) = &config.tpch_dir {
        datasets.push(tpch_open_dataset(Path::new(path), config.open_limit)?);
    }
    if let Some(path) = &config.lahman_dir {
        datasets.push(lahman_dataset(Path::new(path), config.open_limit)?);
    }
    if let Some(path) = &config.ldbc_dir {
        datasets.push(ldbc_dataset(Path::new(path), config.open_limit)?);
    }
    Ok(datasets)
}

fn job_dataset(dir: &Path, limit: Option<usize>) -> Result<Dataset, Box<dyn std::error::Error>> {
    Ok(Dataset {
        name: "job",
        schema: job_schema(),
        facts: Vec::new(),
        fact_source: Some(FactSource::Job {
            dir: dir.to_path_buf(),
            limit,
        }),
        sqlite_schema: JOB_SQLITE_SCHEMA,
        sqlite_insert: insert_job_sqlite,
        queries: job_queries(),
    })
}

fn stream_job_facts(
    dir: &Path,
    limit: Option<usize>,
    mut emit: impl FnMut(Fact) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let dimension_limit = scaled_limit(limit, 20);
    let name_limit = scaled_limit(limit, 10);
    let fact_limit = scaled_limit(limit, 40);
    let cast_limit = scaled_limit(limit, 80);
    let mut emitted = 0usize;
    let mut comp_cast_types = BTreeSet::new();
    let mut company_types = BTreeSet::new();
    let mut info_types = BTreeSet::new();
    let mut kind_types = BTreeSet::new();
    let mut link_types = BTreeSet::new();
    let mut role_types = BTreeSet::new();
    let mut keywords = BTreeSet::new();
    let mut companies = BTreeSet::new();
    let mut characters = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut titles = BTreeSet::new();
    macro_rules! emit_fact {
        ($fact:expr) => {{
            emit($fact)?;
            emitted += 1;
        }};
    }

    read_job_csv(dir, "comp_cast_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        comp_cast_types.insert(id);
        emit(Fact::new(
            "CompCastType",
            [
                ("id", Value::Serial(id)),
                ("kind", Value::String(job_text(get(&record, 1)))),
            ],
        ))?;
        emitted += 1;
        Ok(true)
    })?;
    read_job_csv(dir, "company_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        company_types.insert(id);
        emit_fact!(Fact::new(
            "CompanyType",
            [
                ("id", Value::Serial(id)),
                ("kind", Value::String(job_text(get(&record, 1)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "info_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        info_types.insert(id);
        emit_fact!(Fact::new(
            "InfoType",
            [
                ("id", Value::Serial(id)),
                ("info", Value::String(job_text(get(&record, 1)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "kind_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        kind_types.insert(id);
        emit_fact!(Fact::new(
            "KindType",
            [
                ("id", Value::Serial(id)),
                ("kind", Value::String(job_text(get(&record, 1)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "link_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        link_types.insert(id);
        emit_fact!(Fact::new(
            "LinkType",
            [
                ("id", Value::Serial(id)),
                ("link", Value::String(job_text(get(&record, 1)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "role_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        role_types.insert(id);
        emit_fact!(Fact::new(
            "RoleType",
            [
                ("id", Value::Serial(id)),
                ("role", Value::String(job_text(get(&record, 1)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "keyword.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        keywords.insert(id);
        emit_fact!(Fact::new(
            "Keyword",
            [
                ("id", Value::Serial(id)),
                ("keyword", Value::String(job_text(get(&record, 1)))),
                ("phonetic_code", Value::String(job_text(get(&record, 2)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "company_name.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        companies.insert(id);
        emit_fact!(Fact::new(
            "CompanyName",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(job_text(get(&record, 1)))),
                ("country_code", Value::String(job_text(get(&record, 2)))),
                ("imdb_id", Value::I64(parse_optional_i64(get(&record, 3)))),
                ("name_pcode_nf", Value::String(job_text(get(&record, 4)))),
                ("name_pcode_sf", Value::String(job_text(get(&record, 5)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "char_name.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        characters.insert(id);
        emit_fact!(Fact::new(
            "CharName",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(job_text(get(&record, 1)))),
                ("imdb_index", Value::String(job_text(get(&record, 2)))),
                ("imdb_id", Value::I64(parse_optional_i64(get(&record, 3)))),
                ("name_pcode_nf", Value::String(job_text(get(&record, 4)))),
                ("surname_pcode", Value::String(job_text(get(&record, 5)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "name.csv", name_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        names.insert(id);
        emit_fact!(Fact::new(
            "Name",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(job_text(get(&record, 1)))),
                ("imdb_index", Value::String(job_text(get(&record, 2)))),
                ("imdb_id", Value::I64(parse_optional_i64(get(&record, 3)))),
                ("gender", Value::String(job_text(get(&record, 4)))),
                ("name_pcode_cf", Value::String(job_text(get(&record, 5)))),
                ("name_pcode_nf", Value::String(job_text(get(&record, 6)))),
                ("surname_pcode", Value::String(job_text(get(&record, 7)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "title.csv", limit, |record| {
        let id = parse_u64(get(&record, 0));
        let kind = parse_u64(get(&record, 3));
        if id == 0 || !kind_types.contains(&kind) {
            return Ok(false);
        }
        titles.insert(id);
        emit_fact!(Fact::new(
            "Title",
            [
                ("id", Value::Serial(id)),
                ("title", Value::String(job_text(get(&record, 1)))),
                ("imdb_index", Value::String(job_text(get(&record, 2)))),
                ("kind", Value::Serial(kind)),
                (
                    "production_year",
                    Value::I64(parse_optional_i64(get(&record, 4))),
                ),
                ("imdb_id", Value::I64(parse_optional_i64(get(&record, 5)))),
                ("phonetic_code", Value::String(job_text(get(&record, 6)))),
                (
                    "episode_of",
                    Value::U64(parse_optional_u64(get(&record, 7))),
                ),
                ("season_nr", Value::I64(parse_optional_i64(get(&record, 8)))),
                (
                    "episode_nr",
                    Value::I64(parse_optional_i64(get(&record, 9))),
                ),
                ("series_years", Value::String(job_text(get(&record, 10)))),
            ],
        ));
        Ok(true)
    })?;

    read_job_csv(dir, "aka_name.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let person = parse_u64(get(&record, 1));
        if id == 0 || !names.contains(&person) {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "AkaName",
            [
                ("id", Value::Serial(id)),
                ("person", Value::Serial(person)),
                ("name", Value::String(job_text(get(&record, 2)))),
                ("imdb_index", Value::String(job_text(get(&record, 3)))),
                ("name_pcode_cf", Value::String(job_text(get(&record, 4)))),
                ("name_pcode_nf", Value::String(job_text(get(&record, 5)))),
                ("surname_pcode", Value::String(job_text(get(&record, 6)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "aka_title.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let kind = parse_u64(get(&record, 4));
        if id == 0 || !(titles.contains(&movie) && kind_types.contains(&kind)) {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "AkaTitle",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("title", Value::String(job_text(get(&record, 2)))),
                ("imdb_index", Value::String(job_text(get(&record, 3)))),
                ("kind", Value::Serial(kind)),
                (
                    "production_year",
                    Value::I64(parse_optional_i64(get(&record, 5))),
                ),
                ("phonetic_code", Value::String(job_text(get(&record, 6)))),
                (
                    "episode_of",
                    Value::U64(parse_optional_u64(get(&record, 7))),
                ),
                ("season_nr", Value::I64(parse_optional_i64(get(&record, 8)))),
                (
                    "episode_nr",
                    Value::I64(parse_optional_i64(get(&record, 9))),
                ),
                ("note", Value::String(job_text(get(&record, 10)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "cast_info.csv", cast_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let person = parse_u64(get(&record, 1));
        let movie = parse_u64(get(&record, 2));
        let person_role = parse_optional_u64(get(&record, 3));
        let role = parse_u64(get(&record, 6));
        if id == 0
            || !(names.contains(&person)
                && titles.contains(&movie)
                && characters.contains(&person_role)
                && role_types.contains(&role))
        {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "CastInfo",
            [
                ("id", Value::Serial(id)),
                ("person", Value::Serial(person)),
                ("movie", Value::Serial(movie)),
                ("person_role", Value::Serial(person_role)),
                ("note", Value::String(job_text(get(&record, 4)))),
                ("nr_order", Value::I64(parse_optional_i64(get(&record, 5)))),
                ("role", Value::Serial(role)),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "complete_cast.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let subject = parse_u64(get(&record, 2));
        let status = parse_u64(get(&record, 3));
        if id == 0
            || !(titles.contains(&movie)
                && comp_cast_types.contains(&subject)
                && comp_cast_types.contains(&status))
        {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "CompleteCast",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("subject", Value::Serial(subject)),
                ("status", Value::Serial(status)),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "movie_companies.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let company = parse_u64(get(&record, 2));
        let company_type = parse_u64(get(&record, 3));
        if id == 0
            || !(titles.contains(&movie)
                && companies.contains(&company)
                && company_types.contains(&company_type))
        {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "MovieCompanies",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("company", Value::Serial(company)),
                ("company_type", Value::Serial(company_type)),
                ("note", Value::String(job_text(get(&record, 4)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "movie_info.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let info_type = parse_u64(get(&record, 2));
        if id == 0 || !(titles.contains(&movie) && info_types.contains(&info_type)) {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "MovieInfo",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("info_type", Value::Serial(info_type)),
                ("info", Value::String(job_text(get(&record, 3)))),
                ("note", Value::String(job_text(get(&record, 4)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "movie_info_idx.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let info_type = parse_u64(get(&record, 2));
        if id == 0 || !(titles.contains(&movie) && info_types.contains(&info_type)) {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "MovieInfoIdx",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("info_type", Value::Serial(info_type)),
                ("info", Value::String(job_text(get(&record, 3)))),
                ("note", Value::String(job_text(get(&record, 4)))),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "movie_keyword.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let keyword = parse_u64(get(&record, 2));
        if id == 0 || !(titles.contains(&movie) && keywords.contains(&keyword)) {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "MovieKeyword",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("keyword", Value::Serial(keyword)),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "movie_link.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let movie = parse_u64(get(&record, 1));
        let linked_movie = parse_u64(get(&record, 2));
        let link_type = parse_u64(get(&record, 3));
        if id == 0
            || !(titles.contains(&movie)
                && titles.contains(&linked_movie)
                && link_types.contains(&link_type))
        {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "MovieLink",
            [
                ("id", Value::Serial(id)),
                ("movie", Value::Serial(movie)),
                ("linked_movie", Value::Serial(linked_movie)),
                ("link_type", Value::Serial(link_type)),
            ],
        ));
        Ok(true)
    })?;
    read_job_csv(dir, "person_info.csv", fact_limit, |record| {
        let id = parse_u64(get(&record, 0));
        let person = parse_u64(get(&record, 1));
        let info_type = parse_u64(get(&record, 2));
        if id == 0 || !(names.contains(&person) && info_types.contains(&info_type)) {
            return Ok(false);
        }
        emit_fact!(Fact::new(
            "PersonInfo",
            [
                ("id", Value::Serial(id)),
                ("person", Value::Serial(person)),
                ("info_type", Value::Serial(info_type)),
                ("info", Value::String(job_text(get(&record, 3)))),
                ("note", Value::String(job_text(get(&record, 4)))),
            ],
        ));
        Ok(true)
    })?;

    Ok(emitted)
}
include!("open/job_schema.rs");

include!("open/job_query_list.rs");

include!("open/job_query_builders.rs");

include!("open/job_field_helpers.rs");

include!("open/imdb.rs");

include!("open/tpch.rs");

include!("open/lahman.rs");

include!("open/ldbc.rs");

include!("open/csv_readers.rs");

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

fn insert_job_sqlite(conn: &Connection, facts: &[Fact]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        insert_job_sqlite_fact(&tx, fact)?;
    }
    tx.commit()?;
    Ok(())
}

fn insert_job_sqlite_fact(
    tx: &rusqlite::Transaction<'_>,
    fact: &Fact,
) -> Result<(), Box<dyn std::error::Error>> {
    match fact.relation() {
        "AkaName" => {
            tx.execute("INSERT INTO aka_name (id, person_id, name, imdb_index, name_pcode_cf, name_pcode_nf, surname_pcode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(fact, "id")?, rf(fact, "person")?, text(fact, "name")?, text(fact, "imdb_index")?, text(fact, "name_pcode_cf")?, text(fact, "name_pcode_nf")?, text(fact, "surname_pcode")?])?;
        }
        "AkaTitle" => {
            tx.execute("INSERT INTO aka_title (id, movie_id, title, imdb_index, kind_id, production_year, phonetic_code, episode_of_id, season_nr, episode_nr, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, text(fact, "title")?, text(fact, "imdb_index")?, rf(fact, "kind")?, i64v(fact, "production_year")?, text(fact, "phonetic_code")?, u64v(fact, "episode_of")?, i64v(fact, "season_nr")?, i64v(fact, "episode_nr")?, text(fact, "note")?])?;
        }
        "CastInfo" => {
            tx.execute("INSERT INTO cast_info (id, person_id, movie_id, person_role_id, note, nr_order, role_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(fact, "id")?, rf(fact, "person")?, rf(fact, "movie")?, rf(fact, "person_role")?, text(fact, "note")?, i64v(fact, "nr_order")?, rf(fact, "role")?])?;
        }
        "CharName" => {
            tx.execute("INSERT INTO char_name (id, name, imdb_index, imdb_id, name_pcode_nf, surname_pcode) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(fact, "id")?, text(fact, "name")?, text(fact, "imdb_index")?, i64v(fact, "imdb_id")?, text(fact, "name_pcode_nf")?, text(fact, "surname_pcode")?])?;
        }
        "CompCastType" => {
            tx.execute(
                "INSERT INTO comp_cast_type (id, kind) VALUES (?1, ?2)",
                rusqlite::params![id(fact, "id")?, text(fact, "kind")?],
            )?;
        }
        "CompanyName" => {
            tx.execute("INSERT INTO company_name (id, name, country_code, imdb_id, name_pcode_nf, name_pcode_sf) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(fact, "id")?, text(fact, "name")?, text(fact, "country_code")?, i64v(fact, "imdb_id")?, text(fact, "name_pcode_nf")?, text(fact, "name_pcode_sf")?])?;
        }
        "CompanyType" => {
            tx.execute(
                "INSERT INTO company_type (id, kind) VALUES (?1, ?2)",
                rusqlite::params![id(fact, "id")?, text(fact, "kind")?],
            )?;
        }
        "CompleteCast" => {
            tx.execute("INSERT INTO complete_cast (id, movie_id, subject_id, status_id) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, rf(fact, "subject")?, rf(fact, "status")?])?;
        }
        "InfoType" => {
            tx.execute(
                "INSERT INTO info_type (id, info) VALUES (?1, ?2)",
                rusqlite::params![id(fact, "id")?, text(fact, "info")?],
            )?;
        }
        "Keyword" => {
            tx.execute(
                "INSERT INTO keyword (id, keyword, phonetic_code) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    id(fact, "id")?,
                    text(fact, "keyword")?,
                    text(fact, "phonetic_code")?
                ],
            )?;
        }
        "KindType" => {
            tx.execute(
                "INSERT INTO kind_type (id, kind) VALUES (?1, ?2)",
                rusqlite::params![id(fact, "id")?, text(fact, "kind")?],
            )?;
        }
        "LinkType" => {
            tx.execute(
                "INSERT INTO link_type (id, link) VALUES (?1, ?2)",
                rusqlite::params![id(fact, "id")?, text(fact, "link")?],
            )?;
        }
        "MovieCompanies" => {
            tx.execute("INSERT INTO movie_companies (id, movie_id, company_id, company_type_id, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, rf(fact, "company")?, rf(fact, "company_type")?, text(fact, "note")?])?;
        }
        "MovieInfo" => {
            tx.execute("INSERT INTO movie_info (id, movie_id, info_type_id, info, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, rf(fact, "info_type")?, text(fact, "info")?, text(fact, "note")?])?;
        }
        "MovieInfoIdx" => {
            tx.execute("INSERT INTO movie_info_idx (id, movie_id, info_type_id, info, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, rf(fact, "info_type")?, text(fact, "info")?, text(fact, "note")?])?;
        }
        "MovieKeyword" => {
            tx.execute(
                "INSERT INTO movie_keyword (id, movie_id, keyword_id) VALUES (?1, ?2, ?3)",
                rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, rf(fact, "keyword")?],
            )?;
        }
        "MovieLink" => {
            tx.execute("INSERT INTO movie_link (id, movie_id, linked_movie_id, link_type_id) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id(fact, "id")?, rf(fact, "movie")?, rf(fact, "linked_movie")?, rf(fact, "link_type")?])?;
        }
        "Name" => {
            tx.execute("INSERT INTO name (id, name, imdb_index, imdb_id, gender, name_pcode_cf, name_pcode_nf, surname_pcode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", rusqlite::params![id(fact, "id")?, text(fact, "name")?, text(fact, "imdb_index")?, i64v(fact, "imdb_id")?, text(fact, "gender")?, text(fact, "name_pcode_cf")?, text(fact, "name_pcode_nf")?, text(fact, "surname_pcode")?])?;
        }
        "PersonInfo" => {
            tx.execute("INSERT INTO person_info (id, person_id, info_type_id, info, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(fact, "id")?, rf(fact, "person")?, rf(fact, "info_type")?, text(fact, "info")?, text(fact, "note")?])?;
        }
        "RoleType" => {
            tx.execute(
                "INSERT INTO role_type (id, role) VALUES (?1, ?2)",
                rusqlite::params![id(fact, "id")?, text(fact, "role")?],
            )?;
        }
        "Title" => {
            tx.execute("INSERT INTO title (id, title, imdb_index, kind_id, production_year, imdb_id, phonetic_code, episode_of_id, season_nr, episode_nr, series_years) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", rusqlite::params![id(fact, "id")?, text(fact, "title")?, text(fact, "imdb_index")?, rf(fact, "kind")?, i64v(fact, "production_year")?, i64v(fact, "imdb_id")?, text(fact, "phonetic_code")?, u64v(fact, "episode_of")?, i64v(fact, "season_nr")?, i64v(fact, "episode_nr")?, text(fact, "series_years")?])?;
        }
        _ => {}
    }
    Ok(())
}

fn insert_imdb_sqlite(conn: &Connection, facts: &[Fact]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Title" => {
                tx.execute("INSERT INTO title (id, title_type, primary_title, start_year) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id(fact, "id")?, symbol(fact, "title_type")?, text(fact, "primary_title")?, i64v(fact, "start_year")?])?;
            }
            "Name" => {
                tx.execute(
                    "INSERT INTO name (id, name, birth_year) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        text(fact, "name")?,
                        i64v(fact, "birth_year")?
                    ],
                )?;
            }
            "TitleRating" => {
                tx.execute(
                    "INSERT INTO title_rating (title, rating, votes) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        rf(fact, "title")?,
                        i64v(fact, "rating")?,
                        i64v(fact, "votes")?
                    ],
                )?;
            }
            "Principal" => {
                tx.execute("INSERT INTO principal (title, name, category, ordering) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![rf(fact, "title")?, rf(fact, "name")?, symbol(fact, "category")?, u64v(fact, "ordering")?])?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_lahman_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Player" => {
                tx.execute(
                    "INSERT INTO player (id, first, last) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, text(fact, "first")?, text(fact, "last")?],
                )?;
            }
            "Team" => {
                tx.execute(
                    "INSERT INTO team (id, year, league, name) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        id(fact, "id")?,
                        i64v(fact, "year")?,
                        text(fact, "league")?,
                        text(fact, "name")?
                    ],
                )?;
            }
            "Batting" => {
                tx.execute("INSERT INTO batting (player, team, year, games, hits) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![rf(fact, "player")?, rf(fact, "team")?, i64v(fact, "year")?, i64v(fact, "games")?, i64v(fact, "hits")?])?;
            }
            "Salary" => {
                tx.execute(
                    "INSERT INTO salary (player, team, year, salary) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        rf(fact, "player")?,
                        rf(fact, "team")?,
                        i64v(fact, "year")?,
                        i64v(fact, "salary")?
                    ],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_ldbc_sqlite(conn: &Connection, facts: &[Fact]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Person" => {
                tx.execute(
                    "INSERT INTO person (id, first, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, text(fact, "first")?, ts(fact, "created")?],
                )?;
            }
            "Post" => {
                tx.execute(
                    "INSERT INTO post (id, creator, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "creator")?, ts(fact, "created")?],
                )?;
            }
            "Knows" => {
                tx.execute(
                    "INSERT OR IGNORE INTO knows (person1, person2, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        rf(fact, "person1")?,
                        rf(fact, "person2")?,
                        ts(fact, "created")?
                    ],
                )?;
            }
            "Likes" => {
                tx.execute(
                    "INSERT OR IGNORE INTO likes (person, post, created) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(fact, "person")?, rf(fact, "post")?, ts(fact, "created")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
