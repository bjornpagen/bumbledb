use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{AggregateFunction, ComparisonOperator, Literal, TypedQuery};
use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};
use bumbledb_lmdb::{Row, Value};
use csv::{ReaderBuilder, StringRecord};
use rusqlite::Connection;

use crate::{
    BenchQuery, Config, Dataset, SqlParam, i64v, id, rf, serial_field, serial_key_field, symbol,
    text, ts, u64v,
};

#[derive(Clone, Debug)]
pub(crate) enum RowSource {
    Job { dir: PathBuf, limit: Option<usize> },
}

pub(crate) fn stream_rows(
    source: &RowSource,
    emit: impl FnMut(Row) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<usize, Box<dyn std::error::Error>> {
    match source {
        RowSource::Job { dir, limit } => stream_job_rows(dir, *limit, emit),
    }
}

pub(crate) fn insert_sqlite_streaming(
    source: &RowSource,
    conn: &mut Connection,
) -> Result<usize, Box<dyn std::error::Error>> {
    match source {
        RowSource::Job { dir, limit } => {
            let tx = conn.unchecked_transaction()?;
            let inserted = stream_job_rows(dir, *limit, |row| {
                insert_job_sqlite_row(&tx, &row)?;
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
        rows: Vec::new(),
        row_source: Some(RowSource::Job {
            dir: dir.to_path_buf(),
            limit,
        }),
        sqlite_schema: JOB_SQLITE_SCHEMA,
        sqlite_insert: insert_job_sqlite,
        queries: job_queries(),
    })
}

fn stream_job_rows(
    dir: &Path,
    limit: Option<usize>,
    mut emit: impl FnMut(Row) -> Result<(), Box<dyn std::error::Error>>,
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
    macro_rules! emit_row {
        ($row:expr) => {{
            emit($row)?;
            emitted += 1;
        }};
    }

    read_job_csv(dir, "comp_cast_type.csv", dimension_limit, |record| {
        let id = parse_u64(get(&record, 0));
        if id == 0 {
            return Ok(false);
        }
        comp_cast_types.insert(id);
        emit(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
        emit_row!(Row::new(
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
fn job_schema() -> SchemaDescriptor {
    let mut relations = vec![
        job_relation(
            "AkaName",
            vec![
                serial_key_field("AkaNameId", "AkaName"),
                serial_field("NameId", "person", "Name"),
                job_string_field("name"),
                job_string_field("imdb_index"),
                job_string_field("name_pcode_cf"),
                job_string_field("name_pcode_nf"),
                job_string_field("surname_pcode"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_person_id",
            ["person", "id"],
        )),
        job_relation(
            "AkaTitle",
            vec![
                serial_key_field("AkaTitleId", "AkaTitle"),
                serial_field("TitleId", "movie", "Title"),
                job_string_field("title"),
                job_string_field("imdb_index"),
                serial_field("KindTypeId", "kind", "KindType"),
                job_range_i64_field("production_year"),
                job_string_field("phonetic_code"),
                job_u64_field("episode_of"),
                job_i64_field("season_nr"),
                job_range_i64_field("episode_nr"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_kind",
            ["movie", "kind", "id"],
        )),
        job_relation(
            "CastInfo",
            vec![
                serial_key_field("CastInfoId", "CastInfo"),
                serial_field("NameId", "person", "Name"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("CharNameId", "person_role", "CharName"),
                job_string_field("note"),
                job_i64_field("nr_order"),
                serial_field("RoleTypeId", "role", "RoleType"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_role_person",
            ["movie", "role", "person", "person_role", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_person_movie",
            ["person", "movie", "role", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_person_role_movie",
            ["person_role", "movie", "person", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_role_movie",
            ["role", "movie", "person", "id"],
        )),
        job_relation(
            "CharName",
            vec![
                serial_key_field("CharNameId", "CharName"),
                job_string_field("name"),
                job_string_field("imdb_index"),
                job_i64_field("imdb_id"),
                job_string_field("name_pcode_nf"),
                job_string_field("surname_pcode"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_name", ["name", "id"])),
        job_relation(
            "CompCastType",
            vec![
                serial_key_field("CompCastTypeId", "CompCastType"),
                job_string_field("kind"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_kind", ["kind", "id"])),
        job_relation(
            "CompanyName",
            vec![
                serial_key_field("CompanyNameId", "CompanyName"),
                job_string_field("name"),
                job_string_field("country_code"),
                job_i64_field("imdb_id"),
                job_string_field("name_pcode_nf"),
                job_string_field("name_pcode_sf"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_country",
            ["country_code", "id"],
        ))
        .with_index(IndexDescriptor::permutation("by_name", ["name", "id"])),
        job_relation(
            "CompanyType",
            vec![
                serial_key_field("CompanyTypeId", "CompanyType"),
                job_string_field("kind"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_kind", ["kind", "id"])),
        job_relation(
            "CompleteCast",
            vec![
                serial_key_field("CompleteCastId", "CompleteCast"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("CompCastTypeId", "subject", "CompCastType"),
                serial_field("CompCastTypeId", "status", "CompCastType"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_subject_status",
            ["movie", "subject", "status", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_subject_status",
            ["subject", "status", "movie", "id"],
        )),
        job_relation(
            "InfoType",
            vec![
                serial_key_field("InfoTypeId", "InfoType"),
                job_string_field("info"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_info", ["info", "id"])),
        job_relation(
            "Keyword",
            vec![
                serial_key_field("KeywordId", "Keyword"),
                job_string_field("keyword"),
                job_string_field("phonetic_code"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_keyword",
            ["keyword", "id"],
        )),
        job_relation(
            "KindType",
            vec![
                serial_key_field("KindTypeId", "KindType"),
                job_string_field("kind"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_kind", ["kind", "id"])),
        job_relation(
            "LinkType",
            vec![
                serial_key_field("LinkTypeId", "LinkType"),
                job_string_field("link"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_link", ["link", "id"])),
        job_relation(
            "MovieCompanies",
            vec![
                serial_key_field("MovieCompaniesId", "MovieCompanies"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("CompanyNameId", "company", "CompanyName"),
                serial_field("CompanyTypeId", "company_type", "CompanyType"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_company_type",
            ["movie", "company_type", "company", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_company_movie",
            ["company", "movie", "company_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_company_type_movie",
            ["company_type", "movie", "company", "id"],
        )),
        job_relation(
            "MovieInfo",
            vec![
                serial_key_field("MovieInfoId", "MovieInfo"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("InfoTypeId", "info_type", "InfoType"),
                job_string_field("info"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_type",
            ["movie", "info_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_type_movie",
            ["info_type", "movie", "id"],
        )),
        job_relation(
            "MovieInfoIdx",
            vec![
                serial_key_field("MovieInfoIdxId", "MovieInfoIdx"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("InfoTypeId", "info_type", "InfoType"),
                job_string_field("info"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_type",
            ["movie", "info_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_type_movie",
            ["info_type", "movie", "id"],
        )),
        job_relation(
            "MovieKeyword",
            vec![
                serial_key_field("MovieKeywordId", "MovieKeyword"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("KeywordId", "keyword", "Keyword"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_keyword",
            ["movie", "keyword", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_keyword_movie",
            ["keyword", "movie", "id"],
        )),
        job_relation(
            "MovieLink",
            vec![
                serial_key_field("MovieLinkId", "MovieLink"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("TitleId", "linked_movie", "Title"),
                serial_field("LinkTypeId", "link_type", "LinkType"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_linked",
            ["movie", "linked_movie", "link_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_linked",
            ["linked_movie", "movie", "link_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_link_type_movie",
            ["link_type", "movie", "linked_movie", "id"],
        )),
        job_relation(
            "Name",
            vec![
                serial_key_field("NameId", "Name"),
                job_string_field("name"),
                job_string_field("imdb_index"),
                job_i64_field("imdb_id"),
                job_string_field("gender"),
                job_string_field("name_pcode_cf"),
                job_string_field("name_pcode_nf"),
                job_string_field("surname_pcode"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_gender", ["gender", "id"]))
        .with_index(IndexDescriptor::permutation("by_name", ["name", "id"])),
        job_relation(
            "PersonInfo",
            vec![
                serial_key_field("PersonInfoId", "PersonInfo"),
                serial_field("NameId", "person", "Name"),
                serial_field("InfoTypeId", "info_type", "InfoType"),
                job_string_field("info"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_person_info_type",
            ["person", "info_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_info_type_person",
            ["info_type", "person", "id"],
        )),
        job_relation(
            "RoleType",
            vec![
                serial_key_field("RoleTypeId", "RoleType"),
                job_string_field("role"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_role", ["role", "id"])),
        job_relation(
            "Title",
            vec![
                serial_key_field("TitleId", "Title"),
                job_string_field("title"),
                job_string_field("imdb_index"),
                serial_field("KindTypeId", "kind", "KindType"),
                job_range_i64_field("production_year"),
                job_i64_field("imdb_id"),
                job_string_field("phonetic_code"),
                job_u64_field("episode_of"),
                job_i64_field("season_nr"),
                job_range_i64_field("episode_nr"),
                job_string_field("series_years"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_kind_year",
            ["kind", "production_year", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_year",
            ["production_year", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_episode",
            ["episode_nr", "id"],
        )),
    ];
    relations.sort_by_key(|relation| job_relation_order(&relation.name));
    add_serial_foreign_keys(SchemaDescriptor::new("JoinOrderBenchmarkDb", relations))
}

fn job_relation(name: impl Into<String>, fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    RelationDescriptor::new(name, fields).with_covering_unique("id", ["id"])
}

fn add_serial_foreign_keys(mut schema: SchemaDescriptor) -> SchemaDescriptor {
    for relation in &mut schema.relations {
        for field in relation.fields.clone() {
            let ValueType::Serial {
                owning_relation, ..
            } = field.value_type
            else {
                continue;
            };
            if owning_relation == relation.name {
                continue;
            }
            relation.constraints.push(ConstraintDescriptor::foreign_key(
                field.name.clone(),
                [field.name],
                owning_relation,
                "id",
            ));
        }
    }
    schema
}

fn job_relation_order(name: &str) -> usize {
    match name {
        "CompCastType" => 0,
        "CompanyName" => 1,
        "CompanyType" => 2,
        "InfoType" => 3,
        "Keyword" => 4,
        "KindType" => 5,
        "LinkType" => 6,
        "RoleType" => 7,
        "CharName" => 8,
        "Name" => 9,
        "Title" => 10,
        "AkaName" => 11,
        "AkaTitle" => 12,
        "CastInfo" => 13,
        "CompleteCast" => 14,
        "MovieCompanies" => 15,
        "MovieInfo" => 16,
        "MovieInfoIdx" => 17,
        "MovieKeyword" => 18,
        "MovieLink" => 19,
        "PersonInfo" => 20,
        _ => usize::MAX,
    }
}

fn job_queries() -> Vec<BenchQuery> {
    // Bumbledb follows set/Codd aggregate semantics: global count over an empty
    // input returns a single row containing 0. Keep equivalent SQLite COUNT(*)
    // queries free of HAVING COUNT(*) > 0, or empty JOB slices will mismatch.
    vec![
        BenchQuery {
            name: "job_broad_cast_keyword_company",
            build: build_job_broad_cast_keyword_company,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM title t
                JOIN cast_info ci ON ci.movie_id = t.id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                JOIN company_type ct ON ct.id = mc.company_type_id
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_broad_movie_info_star",
            build: build_job_broad_movie_info_star,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM title t
                JOIN cast_info ci ON ci.movie_id = t.id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_type ct ON ct.id = mc.company_type_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                JOIN movie_info mi ON mi.movie_id = t.id
                JOIN info_type it ON it.id = mi.info_type_id
                JOIN movie_info_idx mi_idx ON mi_idx.movie_id = t.id
                JOIN info_type it_idx ON it_idx.id = mi_idx.info_type_id
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q01_top_production",
            build: build_job_q01_top_production,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM company_type ct
                JOIN movie_companies mc ON mc.company_type_id = ct.id
                JOIN movie_info_idx mi_idx ON mi_idx.movie_id = mc.movie_id
                JOIN info_type it ON it.id = mi_idx.info_type_id
                JOIN title t ON t.id = mc.movie_id
                WHERE ct.kind = 'production companies'
                  AND it.info = 'top 250 rank'
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q09_voice_us_actor",
            build: build_job_q09_voice_us_actor,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM aka_name an
                JOIN name n ON n.id = an.person_id
                JOIN cast_info ci ON ci.person_id = n.id
                JOIN char_name chn ON chn.id = ci.person_role_id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN title t ON t.id = ci.movie_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                WHERE cn.country_code = '[us]'
                  AND n.gender = 'm'
                  AND rt.role = 'actor'
                  AND t.production_year BETWEEN 2005 AND 2015
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q16_character_title_us",
            build: build_job_q16_character_title_us,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM aka_name an
                JOIN name n ON n.id = an.person_id
                JOIN cast_info ci ON ci.person_id = n.id
                JOIN title t ON t.id = ci.movie_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                WHERE cn.country_code = '[us]'
                  AND k.keyword = 'character-name-in-title'
                  AND t.episode_nr >= 50
                  AND t.episode_nr < 100
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q24_voice_keyword_actor",
            build: build_job_q24_voice_keyword_actor,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM aka_name an
                JOIN name n ON n.id = an.person_id
                JOIN cast_info ci ON ci.person_id = n.id
                JOIN char_name chn ON chn.id = ci.person_role_id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN title t ON t.id = ci.movie_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                WHERE cn.country_code = '[us]'
                  AND k.keyword = 'hero'
                  AND n.gender = 'm'
                  AND rt.role = 'actor'
                  AND t.production_year > 2010
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_movie_link_bridge",
            build: build_job_movie_link_bridge,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM movie_link ml
                JOIN link_type lt ON lt.id = ml.link_type_id
                JOIN title t1 ON t1.id = ml.movie_id
                JOIN title t2 ON t2.id = ml.linked_movie_id
                JOIN movie_companies mc1 ON mc1.movie_id = t1.id
                JOIN company_name cn1 ON cn1.id = mc1.company_id
                JOIN movie_companies mc2 ON mc2.movie_id = t2.id
                JOIN company_name cn2 ON cn2.id = mc2.company_id
                JOIN movie_info_idx mi_idx1 ON mi_idx1.movie_id = t1.id
                JOIN info_type it1 ON it1.id = mi_idx1.info_type_id
                JOIN movie_info_idx mi_idx2 ON mi_idx2.movie_id = t2.id
                JOIN info_type it2 ON it2.id = mi_idx2.info_type_id
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q33_linked_series_companies",
            build: build_job_q33_linked_series_companies,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT COUNT(*)
                FROM company_name cn1
                JOIN movie_companies mc1 ON mc1.company_id = cn1.id
                JOIN title t1 ON t1.id = mc1.movie_id
                JOIN kind_type kt1 ON kt1.id = t1.kind_id
                JOIN movie_link ml ON ml.movie_id = t1.id
                JOIN link_type lt ON lt.id = ml.link_type_id
                JOIN title t2 ON t2.id = ml.linked_movie_id
                JOIN kind_type kt2 ON kt2.id = t2.kind_id
                JOIN movie_companies mc2 ON mc2.movie_id = t2.id
                JOIN company_name cn2 ON cn2.id = mc2.company_id
                WHERE cn1.country_code = '[us]'
                  AND kt1.kind = 'tv series'
                  AND kt2.kind = 'tv series'
                  AND lt.link = 'sequel'
                  AND t2.production_year BETWEEN 2005 AND 2008
            "#,
            sqlite_params: Vec::new(),
        },
    ]
}

fn build_job_broad_cast_keyword_company(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Title")?
        .var("id", "movie")?
        .var("kind", "kind")?
        .done()
        .rel("CastInfo")?
        .var("movie", "movie")?
        .var("person", "person")?
        .var("role", "role")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .var("company_type", "company_type")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .done()
        .rel("CompanyType")?
        .var("id", "company_type")?
        .done()
        .find_aggregate(AggregateFunction::Count, "movie")?
        .finish()
}

fn build_job_broad_movie_info_star(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Title")?
        .var("id", "movie")?
        .done()
        .rel("CastInfo")?
        .var("movie", "movie")?
        .var("role", "role")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company_type", "company_type")?
        .done()
        .rel("CompanyType")?
        .var("id", "company_type")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .done()
        .rel("MovieInfo")?
        .var("movie", "movie")?
        .var("info_type", "info_type")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie")?
        .var("info_type", "idx_info_type")?
        .done()
        .rel("InfoType")?
        .var("id", "idx_info_type")?
        .done()
        .find_aggregate(AggregateFunction::Count, "movie")?
        .finish()
}

fn build_job_q01_top_production(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("CompanyType")?
        .var("id", "company_type")?
        .string("kind", "production companies")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type")?
        .string("info", "top 250 rank")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company_type", "company_type")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie")?
        .var("info_type", "info_type")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .done()
        .find_aggregate(AggregateFunction::Count, "movie")?
        .finish()
}

fn build_job_q09_voice_us_actor(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("AkaName")?
        .var("person", "person")?
        .done()
        .rel("CastInfo")?
        .var("person", "person")?
        .var("movie", "movie")?
        .var("person_role", "character")?
        .var("role", "role")?
        .done()
        .rel("CharName")?
        .var("id", "character")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .string("country_code", "[us]")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .done()
        .rel("Name")?
        .var("id", "person")?
        .string("gender", "m")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .string("role", "actor")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .var("production_year", "year")?
        .done()
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gte,
            OperandRef::literal(Literal::Integer(2005)),
        )?
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Lte,
            OperandRef::literal(Literal::Integer(2015)),
        )?
        .find_aggregate(AggregateFunction::Count, "movie")?
        .finish()
}

fn build_job_q16_character_title_us(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("AkaName")?
        .var("person", "person")?
        .done()
        .rel("CastInfo")?
        .var("person", "person")?
        .var("movie", "movie")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .string("country_code", "[us]")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .string("keyword", "character-name-in-title")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Name")?
        .var("id", "person")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .var("episode_nr", "episode")?
        .done()
        .cmp(
            OperandRef::var("episode"),
            ComparisonOperator::Gte,
            OperandRef::literal(Literal::Integer(50)),
        )?
        .cmp(
            OperandRef::var("episode"),
            ComparisonOperator::Lt,
            OperandRef::literal(Literal::Integer(100)),
        )?
        .find_aggregate(AggregateFunction::Count, "movie")?
        .finish()
}

fn build_job_q24_voice_keyword_actor(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("AkaName")?
        .var("person", "person")?
        .done()
        .rel("CastInfo")?
        .var("person", "person")?
        .var("movie", "movie")?
        .var("person_role", "character")?
        .var("role", "role")?
        .done()
        .rel("CharName")?
        .var("id", "character")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .string("country_code", "[us]")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .string("keyword", "hero")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Name")?
        .var("id", "person")?
        .string("gender", "m")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .string("role", "actor")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .var("production_year", "year")?
        .done()
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gt,
            OperandRef::literal(Literal::Integer(2010)),
        )?
        .find_var("movie")?
        .finish()
}

fn build_job_movie_link_bridge(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("MovieLink")?
        .var("movie", "movie1")?
        .var("linked_movie", "movie2")?
        .var("link_type", "link_type")?
        .done()
        .rel("LinkType")?
        .var("id", "link_type")?
        .done()
        .rel("Title")?
        .var("id", "movie1")?
        .done()
        .rel("Title")?
        .var("id", "movie2")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie1")?
        .var("company", "company1")?
        .done()
        .rel("CompanyName")?
        .var("id", "company1")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie2")?
        .var("company", "company2")?
        .done()
        .rel("CompanyName")?
        .var("id", "company2")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie1")?
        .var("info_type", "info_type1")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type1")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie2")?
        .var("info_type", "info_type2")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type2")?
        .done()
        .find_aggregate(AggregateFunction::Count, "movie1")?
        .finish()
}

fn build_job_q33_linked_series_companies(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("CompanyName")?
        .var("id", "company1")?
        .string("country_code", "[us]")?
        .done()
        .rel("CompanyName")?
        .var("id", "company2")?
        .done()
        .rel("KindType")?
        .var("id", "kind1")?
        .string("kind", "tv series")?
        .done()
        .rel("KindType")?
        .var("id", "kind2")?
        .string("kind", "tv series")?
        .done()
        .rel("LinkType")?
        .var("id", "link_type")?
        .string("link", "sequel")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie1")?
        .var("company", "company1")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie2")?
        .var("company", "company2")?
        .done()
        .rel("MovieLink")?
        .var("movie", "movie1")?
        .var("linked_movie", "movie2")?
        .var("link_type", "link_type")?
        .done()
        .rel("Title")?
        .var("id", "movie1")?
        .var("kind", "kind1")?
        .done()
        .rel("Title")?
        .var("id", "movie2")?
        .var("kind", "kind2")?
        .var("production_year", "year2")?
        .done()
        .cmp(
            OperandRef::var("year2"),
            ComparisonOperator::Gte,
            OperandRef::literal(Literal::Integer(2005)),
        )?
        .cmp(
            OperandRef::var("year2"),
            ComparisonOperator::Lte,
            OperandRef::literal(Literal::Integer(2008)),
        )?
        .find_aggregate(AggregateFunction::Count, "movie1")?
        .finish()
}

fn job_string_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::String)
}

fn job_i64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::I64)
}

fn job_range_i64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::I64).range_indexed()
}

fn job_u64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::U64)
}

const JOB_SQLITE_SCHEMA: &str = r#"
    CREATE TABLE aka_name (id INTEGER PRIMARY KEY, person_id INTEGER NOT NULL, name TEXT NOT NULL, imdb_index TEXT NOT NULL, name_pcode_cf TEXT NOT NULL, name_pcode_nf TEXT NOT NULL, surname_pcode TEXT NOT NULL);
    CREATE TABLE aka_title (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, title TEXT NOT NULL, imdb_index TEXT NOT NULL, kind_id INTEGER NOT NULL, production_year INTEGER NOT NULL, phonetic_code TEXT NOT NULL, episode_of_id INTEGER NOT NULL, season_nr INTEGER NOT NULL, episode_nr INTEGER NOT NULL, note TEXT NOT NULL);
    CREATE TABLE cast_info (id INTEGER PRIMARY KEY, person_id INTEGER NOT NULL, movie_id INTEGER NOT NULL, person_role_id INTEGER NOT NULL, note TEXT NOT NULL, nr_order INTEGER NOT NULL, role_id INTEGER NOT NULL);
    CREATE TABLE char_name (id INTEGER PRIMARY KEY, name TEXT NOT NULL, imdb_index TEXT NOT NULL, imdb_id INTEGER NOT NULL, name_pcode_nf TEXT NOT NULL, surname_pcode TEXT NOT NULL);
    CREATE TABLE comp_cast_type (id INTEGER PRIMARY KEY, kind TEXT NOT NULL);
    CREATE TABLE company_name (id INTEGER PRIMARY KEY, name TEXT NOT NULL, country_code TEXT NOT NULL, imdb_id INTEGER NOT NULL, name_pcode_nf TEXT NOT NULL, name_pcode_sf TEXT NOT NULL);
    CREATE TABLE company_type (id INTEGER PRIMARY KEY, kind TEXT NOT NULL);
    CREATE TABLE complete_cast (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, subject_id INTEGER NOT NULL, status_id INTEGER NOT NULL);
    CREATE TABLE info_type (id INTEGER PRIMARY KEY, info TEXT NOT NULL);
    CREATE TABLE keyword (id INTEGER PRIMARY KEY, keyword TEXT NOT NULL, phonetic_code TEXT NOT NULL);
    CREATE TABLE kind_type (id INTEGER PRIMARY KEY, kind TEXT NOT NULL);
    CREATE TABLE link_type (id INTEGER PRIMARY KEY, link TEXT NOT NULL);
    CREATE TABLE movie_companies (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, company_id INTEGER NOT NULL, company_type_id INTEGER NOT NULL, note TEXT NOT NULL);
    CREATE TABLE movie_info (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, info_type_id INTEGER NOT NULL, info TEXT NOT NULL, note TEXT NOT NULL);
    CREATE TABLE movie_info_idx (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, info_type_id INTEGER NOT NULL, info TEXT NOT NULL, note TEXT NOT NULL);
    CREATE TABLE movie_keyword (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, keyword_id INTEGER NOT NULL);
    CREATE TABLE movie_link (id INTEGER PRIMARY KEY, movie_id INTEGER NOT NULL, linked_movie_id INTEGER NOT NULL, link_type_id INTEGER NOT NULL);
    CREATE TABLE name (id INTEGER PRIMARY KEY, name TEXT NOT NULL, imdb_index TEXT NOT NULL, imdb_id INTEGER NOT NULL, gender TEXT NOT NULL, name_pcode_cf TEXT NOT NULL, name_pcode_nf TEXT NOT NULL, surname_pcode TEXT NOT NULL);
    CREATE TABLE person_info (id INTEGER PRIMARY KEY, person_id INTEGER NOT NULL, info_type_id INTEGER NOT NULL, info TEXT NOT NULL, note TEXT NOT NULL);
    CREATE TABLE role_type (id INTEGER PRIMARY KEY, role TEXT NOT NULL);
    CREATE TABLE title (id INTEGER PRIMARY KEY, title TEXT NOT NULL, imdb_index TEXT NOT NULL, kind_id INTEGER NOT NULL, production_year INTEGER NOT NULL, imdb_id INTEGER NOT NULL, phonetic_code TEXT NOT NULL, episode_of_id INTEGER NOT NULL, season_nr INTEGER NOT NULL, episode_nr INTEGER NOT NULL, series_years TEXT NOT NULL);

    CREATE INDEX aka_name_person ON aka_name(person_id, id);
    CREATE INDEX aka_title_movie ON aka_title(movie_id, kind_id, id);
    CREATE INDEX cast_info_movie ON cast_info(movie_id, role_id, person_id, person_role_id, id);
    CREATE INDEX cast_info_person ON cast_info(person_id, movie_id, role_id, id);
    CREATE INDEX cast_info_person_role ON cast_info(person_role_id, movie_id, person_id, id);
    CREATE INDEX cast_info_role ON cast_info(role_id, movie_id, person_id, id);
    CREATE INDEX char_name_name ON char_name(name, id);
    CREATE INDEX comp_cast_type_kind ON comp_cast_type(kind, id);
    CREATE INDEX company_name_country ON company_name(country_code, id);
    CREATE INDEX company_name_name ON company_name(name, id);
    CREATE INDEX company_type_kind ON company_type(kind, id);
    CREATE INDEX complete_cast_movie ON complete_cast(movie_id, subject_id, status_id, id);
    CREATE INDEX complete_cast_subject_status ON complete_cast(subject_id, status_id, movie_id, id);
    CREATE INDEX info_type_info ON info_type(info, id);
    CREATE INDEX keyword_keyword ON keyword(keyword, id);
    CREATE INDEX kind_type_kind ON kind_type(kind, id);
    CREATE INDEX link_type_link ON link_type(link, id);
    CREATE INDEX movie_companies_movie ON movie_companies(movie_id, company_type_id, company_id, id);
    CREATE INDEX movie_companies_company ON movie_companies(company_id, movie_id, company_type_id, id);
    CREATE INDEX movie_companies_type ON movie_companies(company_type_id, movie_id, company_id, id);
    CREATE INDEX movie_info_movie_type ON movie_info(movie_id, info_type_id, id);
    CREATE INDEX movie_info_type_movie ON movie_info(info_type_id, movie_id, id);
    CREATE INDEX movie_info_idx_movie_type ON movie_info_idx(movie_id, info_type_id, id);
    CREATE INDEX movie_info_idx_type_movie ON movie_info_idx(info_type_id, movie_id, id);
    CREATE INDEX movie_keyword_movie ON movie_keyword(movie_id, keyword_id, id);
    CREATE INDEX movie_keyword_keyword ON movie_keyword(keyword_id, movie_id, id);
    CREATE INDEX movie_link_movie ON movie_link(movie_id, linked_movie_id, link_type_id, id);
    CREATE INDEX movie_link_linked ON movie_link(linked_movie_id, movie_id, link_type_id, id);
    CREATE INDEX movie_link_type ON movie_link(link_type_id, movie_id, linked_movie_id, id);
    CREATE INDEX name_gender ON name(gender, id);
    CREATE INDEX name_name ON name(name, id);
    CREATE INDEX person_info_person ON person_info(person_id, info_type_id, id);
    CREATE INDEX person_info_type ON person_info(info_type_id, person_id, id);
    CREATE INDEX role_type_role ON role_type(role, id);
    CREATE INDEX title_kind_year ON title(kind_id, production_year, id);
    CREATE INDEX title_year ON title(production_year, id);
    CREATE INDEX title_episode ON title(episode_nr, id);
"#;

fn imdb_dataset(dir: &Path, limit: Option<usize>) -> Result<Dataset, Box<dyn std::error::Error>> {
    let mut title_ids = BTreeMap::new();
    let mut name_ids = BTreeMap::new();
    let mut symbols = Symbols::default();
    let mut rows = Vec::new();

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
        rows.push(Row::new(
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
        rows.push(Row::new(
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
        rows.push(Row::new(
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
        rows.push(Row::new(
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
        rows,
        row_source: None,
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
                build: build_imdb_category_rating_join,
                inputs: vec![
                    ("category", Value::U64(sample_category)),
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
                vec![
                    serial_key_field("TitleId", "Title"),
                    FieldDescriptor::new("title_type", ValueType::U64),
                    FieldDescriptor::new("primary_title", ValueType::String),
                    FieldDescriptor::new("start_year", ValueType::I64).range_indexed(),
                ],
            )
            .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "Name",
                vec![
                    serial_key_field("NameId", "Name"),
                    FieldDescriptor::new("name", ValueType::String),
                    FieldDescriptor::new("birth_year", ValueType::I64).range_indexed(),
                ],
            )
            .with_covering_unique("id", ["id"]),
            RelationDescriptor::new(
                "TitleRating",
                vec![
                    serial_field("TitleId", "title", "Title"),
                    FieldDescriptor::new("rating", ValueType::I64).range_indexed(),
                    FieldDescriptor::new("votes", ValueType::I64),
                ],
            )
            .with_covering_unique("title", ["title"])
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
            .with_covering_unique(
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

fn tpch_open_dataset(
    dir: &Path,
    limit: Option<usize>,
) -> Result<Dataset, Box<dyn std::error::Error>> {
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
                ("id", Value::Serial(id)),
                ("nation", Value::U64(parse_u64(get(&record, 3)))),
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
                ("id", Value::Serial(id)),
                ("nation", Value::U64(parse_u64(get(&record, 3)))),
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
                ("id", Value::Serial(id)),
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
                ("id", Value::Serial(id)),
                ("customer", Value::Serial(customer)),
                (
                    "order_date",
                    Value::Timestamp(TimestampMicros(parse_date(get(&record, 4)))),
                ),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "lineitem.tbl", scaled_limit(limit, 4), |record| {
        let order = parse_u64(get(&record, 0));
        let part = parse_u64(get(&record, 1));
        let supplier = parse_u64(get(&record, 2));
        if !(orders.contains(&order) && parts.contains(&part) && suppliers.contains(&supplier)) {
            return Ok(());
        }
        rows.push(Row::new(
            "LineItem",
            [
                ("id", Value::Serial(rows.len() as u64 + 1)),
                ("order", Value::Serial(order)),
                ("part", Value::Serial(part)),
                ("supplier", Value::Serial(supplier)),
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
    dataset.row_source = None;
    Ok(dataset)
}

fn lahman_dataset(dir: &Path, limit: Option<usize>) -> Result<Dataset, Box<dyn std::error::Error>> {
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
                ("id", Value::Serial(id)),
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

    read_csv(
        dir,
        "Teams.csv",
        scaled_limit(limit, 4),
        |headers, record| {
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
                    ("id", Value::Serial(id)),
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
        },
    )?;

    read_csv(
        dir,
        "Batting.csv",
        scaled_limit(limit, 10),
        |headers, record| {
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
                    ("player", Value::Serial(player)),
                    ("team", Value::Serial(team)),
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
        },
    )?;

    read_csv(
        dir,
        "Salaries.csv",
        scaled_limit(limit, 4),
        |headers, record| {
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
                    ("player", Value::Serial(player)),
                    ("team", Value::Serial(team)),
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
        },
    )?;

    Ok(lahman_from_rows(rows))
}

fn ldbc_dataset(dir: &Path, limit: Option<usize>) -> Result<Dataset, Box<dyn std::error::Error>> {
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
        rows.push(Row::new(
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
        rows.push(Row::new(
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
        rows.push(Row::new(
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
                    vec![
                        serial_key_field("PlayerId", "Player"),
                        FieldDescriptor::new("first", ValueType::String),
                        FieldDescriptor::new("last", ValueType::String),
                    ],
                )
                .with_covering_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Team",
                    vec![
                        serial_key_field("TeamId", "Team"),
                        FieldDescriptor::new("year", ValueType::I64).range_indexed(),
                        FieldDescriptor::new("league", ValueType::String),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                )
                .with_covering_unique("id", ["id"]),
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
                .with_covering_unique("player_team_year", ["player", "team", "year"])
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
                .with_covering_unique("player_team_year", ["player", "team", "year"])
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
        rows,
        row_source: None,
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
            sqlite: "SELECT s.player, s.salary, b.hits FROM salary s JOIN batting b ON b.player = s.player AND b.year = s.year WHERE s.year = ?1",
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

fn ldbc_from_rows(rows: Vec<Row>) -> Dataset {
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
                .with_covering_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Post",
                    vec![
                        serial_key_field("PostId", "Post"),
                        serial_field("PersonId", "creator", "Person"),
                        FieldDescriptor::new("created", ValueType::TimestampMicros).range_indexed(),
                    ],
                )
                .with_covering_unique("id", ["id"])
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
                .with_covering_unique("person1_person2", ["person1", "person2"])
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
                .with_covering_unique("person_post", ["person", "post"])
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
        rows,
        row_source: None,
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
                sqlite: "SELECT p.id FROM likes l JOIN post p ON p.id = l.post WHERE l.person = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "two_hop_knows",
                build: build_ldbc_two_hop_knows,
                inputs: vec![("person", Value::Serial(1))],
                sqlite: "SELECT k2.person2 FROM knows k1 JOIN knows k2 ON k2.person1 = k1.person2 WHERE k1.person1 = ?1",
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

fn read_job_csv(
    dir: &Path,
    file: &str,
    accepted_limit: Option<usize>,
    mut f: impl FnMut(StringRecord) -> Result<bool, Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    eprintln!(
        "[bench:job] reading {} limit={}",
        file,
        accepted_limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "full".to_owned())
    );
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)?;
    let mut accepted = 0;
    let mut read = 0usize;
    for record in reader.records() {
        if reached_limit(accepted, accepted_limit) {
            break;
        }
        read += 1;
        if f(record?)? {
            accepted += 1;
            if accepted % 100_000 == 0 {
                eprintln!("[bench:job] {} accepted={} read={}", file, accepted, read);
            }
        }
    }
    eprintln!(
        "[bench:job] finished {} accepted={} read={}",
        file, accepted, read
    );
    Ok(())
}

fn job_text(value: &str) -> String {
    if value.is_empty() || value == r"\N" {
        String::new()
    } else {
        value.to_owned()
    }
}

fn read_csv(
    dir: &Path,
    file: &str,
    limit: Option<usize>,
    mut f: impl FnMut(&StringRecord, &StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    for (read, record) in reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        f(&headers, &record?)?;
    }
    Ok(())
}

fn read_pipe(
    dir: &Path,
    file: &str,
    limit: Option<usize>,
    mut f: impl FnMut(StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    let mut reader = ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .flexible(true)
        .from_path(path)?;
    for (read, record) in reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        f(record?)?;
    }
    Ok(())
}

fn read_pipe_path(
    path: &Path,
    limit: Option<usize>,
    mut f: impl FnMut(&StringRecord, &StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .delimiter(b'|')
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    for (read, record) in reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
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

fn insert_job_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        insert_job_sqlite_row(&tx, row)?;
    }
    tx.commit()?;
    Ok(())
}

fn insert_job_sqlite_row(
    tx: &rusqlite::Transaction<'_>,
    row: &Row,
) -> Result<(), Box<dyn std::error::Error>> {
    match row.relation() {
        "AkaName" => {
            tx.execute("INSERT INTO aka_name (id, person_id, name, imdb_index, name_pcode_cf, name_pcode_nf, surname_pcode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(row, "id")?, rf(row, "person")?, text(row, "name")?, text(row, "imdb_index")?, text(row, "name_pcode_cf")?, text(row, "name_pcode_nf")?, text(row, "surname_pcode")?])?;
        }
        "AkaTitle" => {
            tx.execute("INSERT INTO aka_title (id, movie_id, title, imdb_index, kind_id, production_year, phonetic_code, episode_of_id, season_nr, episode_nr, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", rusqlite::params![id(row, "id")?, rf(row, "movie")?, text(row, "title")?, text(row, "imdb_index")?, rf(row, "kind")?, i64v(row, "production_year")?, text(row, "phonetic_code")?, u64v(row, "episode_of")?, i64v(row, "season_nr")?, i64v(row, "episode_nr")?, text(row, "note")?])?;
        }
        "CastInfo" => {
            tx.execute("INSERT INTO cast_info (id, person_id, movie_id, person_role_id, note, nr_order, role_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(row, "id")?, rf(row, "person")?, rf(row, "movie")?, rf(row, "person_role")?, text(row, "note")?, i64v(row, "nr_order")?, rf(row, "role")?])?;
        }
        "CharName" => {
            tx.execute("INSERT INTO char_name (id, name, imdb_index, imdb_id, name_pcode_nf, surname_pcode) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(row, "id")?, text(row, "name")?, text(row, "imdb_index")?, i64v(row, "imdb_id")?, text(row, "name_pcode_nf")?, text(row, "surname_pcode")?])?;
        }
        "CompCastType" => {
            tx.execute(
                "INSERT INTO comp_cast_type (id, kind) VALUES (?1, ?2)",
                rusqlite::params![id(row, "id")?, text(row, "kind")?],
            )?;
        }
        "CompanyName" => {
            tx.execute("INSERT INTO company_name (id, name, country_code, imdb_id, name_pcode_nf, name_pcode_sf) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(row, "id")?, text(row, "name")?, text(row, "country_code")?, i64v(row, "imdb_id")?, text(row, "name_pcode_nf")?, text(row, "name_pcode_sf")?])?;
        }
        "CompanyType" => {
            tx.execute(
                "INSERT INTO company_type (id, kind) VALUES (?1, ?2)",
                rusqlite::params![id(row, "id")?, text(row, "kind")?],
            )?;
        }
        "CompleteCast" => {
            tx.execute("INSERT INTO complete_cast (id, movie_id, subject_id, status_id) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id(row, "id")?, rf(row, "movie")?, rf(row, "subject")?, rf(row, "status")?])?;
        }
        "InfoType" => {
            tx.execute(
                "INSERT INTO info_type (id, info) VALUES (?1, ?2)",
                rusqlite::params![id(row, "id")?, text(row, "info")?],
            )?;
        }
        "Keyword" => {
            tx.execute(
                "INSERT INTO keyword (id, keyword, phonetic_code) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    id(row, "id")?,
                    text(row, "keyword")?,
                    text(row, "phonetic_code")?
                ],
            )?;
        }
        "KindType" => {
            tx.execute(
                "INSERT INTO kind_type (id, kind) VALUES (?1, ?2)",
                rusqlite::params![id(row, "id")?, text(row, "kind")?],
            )?;
        }
        "LinkType" => {
            tx.execute(
                "INSERT INTO link_type (id, link) VALUES (?1, ?2)",
                rusqlite::params![id(row, "id")?, text(row, "link")?],
            )?;
        }
        "MovieCompanies" => {
            tx.execute("INSERT INTO movie_companies (id, movie_id, company_id, company_type_id, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(row, "id")?, rf(row, "movie")?, rf(row, "company")?, rf(row, "company_type")?, text(row, "note")?])?;
        }
        "MovieInfo" => {
            tx.execute("INSERT INTO movie_info (id, movie_id, info_type_id, info, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(row, "id")?, rf(row, "movie")?, rf(row, "info_type")?, text(row, "info")?, text(row, "note")?])?;
        }
        "MovieInfoIdx" => {
            tx.execute("INSERT INTO movie_info_idx (id, movie_id, info_type_id, info, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(row, "id")?, rf(row, "movie")?, rf(row, "info_type")?, text(row, "info")?, text(row, "note")?])?;
        }
        "MovieKeyword" => {
            tx.execute(
                "INSERT INTO movie_keyword (id, movie_id, keyword_id) VALUES (?1, ?2, ?3)",
                rusqlite::params![id(row, "id")?, rf(row, "movie")?, rf(row, "keyword")?],
            )?;
        }
        "MovieLink" => {
            tx.execute("INSERT INTO movie_link (id, movie_id, linked_movie_id, link_type_id) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![id(row, "id")?, rf(row, "movie")?, rf(row, "linked_movie")?, rf(row, "link_type")?])?;
        }
        "Name" => {
            tx.execute("INSERT INTO name (id, name, imdb_index, imdb_id, gender, name_pcode_cf, name_pcode_nf, surname_pcode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", rusqlite::params![id(row, "id")?, text(row, "name")?, text(row, "imdb_index")?, i64v(row, "imdb_id")?, text(row, "gender")?, text(row, "name_pcode_cf")?, text(row, "name_pcode_nf")?, text(row, "surname_pcode")?])?;
        }
        "PersonInfo" => {
            tx.execute("INSERT INTO person_info (id, person_id, info_type_id, info, note) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![id(row, "id")?, rf(row, "person")?, rf(row, "info_type")?, text(row, "info")?, text(row, "note")?])?;
        }
        "RoleType" => {
            tx.execute(
                "INSERT INTO role_type (id, role) VALUES (?1, ?2)",
                rusqlite::params![id(row, "id")?, text(row, "role")?],
            )?;
        }
        "Title" => {
            tx.execute("INSERT INTO title (id, title, imdb_index, kind_id, production_year, imdb_id, phonetic_code, episode_of_id, season_nr, episode_nr, series_years) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", rusqlite::params![id(row, "id")?, text(row, "title")?, text(row, "imdb_index")?, rf(row, "kind")?, i64v(row, "production_year")?, i64v(row, "imdb_id")?, text(row, "phonetic_code")?, u64v(row, "episode_of")?, i64v(row, "season_nr")?, i64v(row, "episode_nr")?, text(row, "series_years")?])?;
        }
        _ => {}
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_queries_typecheck_against_job_schema() -> Result<(), Box<dyn std::error::Error>> {
        let schema = job_schema();
        for query in job_queries() {
            (query.build)(&schema)?;
        }
        Ok(())
    }

    #[test]
    fn job_dataset_runs_against_minimal_csv_export() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        for (file, contents) in [
            ("aka_name.csv", "1,1,Jane Alias,,,,\n"),
            ("aka_title.csv", "1,1,Series Alias,,2,2012,,0,0,60,,\n"),
            ("cast_info.csv", "1,1,1,1,,1,1\n2,1,2,1,,1,1\n"),
            ("char_name.csv", "1,Heroine,,0,,,\n"),
            ("comp_cast_type.csv", "1,cast\n2,complete\n"),
            ("company_name.csv", "1,Acme,[us],0,,,\n"),
            ("company_type.csv", "1,production companies\n"),
            ("complete_cast.csv", "1,1,1,2\n"),
            (
                "info_type.csv",
                "1,top 250 rank\n2,rating\n3,release dates\n",
            ),
            ("keyword.csv", "1,character-name-in-title,\n2,hero,\n"),
            ("kind_type.csv", "1,movie\n2,tv series\n"),
            ("link_type.csv", "1,sequel\n"),
            ("movie_companies.csv", "1,1,1,1,\n2,2,1,1,\n"),
            ("movie_info.csv", "1,1,3,USA:2011,\n"),
            (
                "movie_info_idx.csv",
                "1,1,1,10,\n2,1,2,7.0,\n3,2,2,2.5,\n4,1,3,USA:2011,\n",
            ),
            ("movie_keyword.csv", "1,1,1\n2,1,2\n"),
            ("movie_link.csv", "1,1,2,1\n"),
            ("name.csv", "1,Jane Doe,,0,m,,,\n"),
            ("person_info.csv", "1,1,3,bio,note\n"),
            ("role_type.csv", "1,actor\n"),
            (
                "title.csv",
                "1,Series One,,2,2012,0,,0,0,60,\n2,Series Two,,2,2006,0,,0,0,0,\n",
            ),
        ] {
            std::fs::write(dir.path().join(file), contents)?;
        }

        let limited = job_dataset(dir.path(), Some(1))?;
        let dataset = job_dataset(dir.path(), None)?;
        assert_eq!(dataset.name, "job");
        assert_eq!(dataset.queries.len(), 8);
        let Some(limited_source) = limited.row_source.as_ref() else {
            return Err("limited JOB dataset should be streaming".into());
        };
        let Some(full_source) = dataset.row_source.as_ref() else {
            return Err("full JOB dataset should be streaming".into());
        };
        let limited_rows = stream_rows(limited_source, |_| Ok(()))?;
        let full_rows = stream_rows(full_source, |_| Ok(()))?;
        assert!(limited_rows < full_rows);

        let config = crate::Config {
            scale: 10,
            open_limit: None,
            repeats: 1,
            warmup: 0,
            datasets: vec!["job".to_owned()],
            queries: Vec::new(),
            imdb_dir: None,
            job_dir: None,
            tpch_dir: None,
            lahman_dir: None,
            ldbc_dir: None,
            preset: None,
            trace: false,
            trace_output: None,
            trace_format: crate::TraceFormat::Fmt,
            format: crate::OutputFormat::Json,
            compare_mode: crate::CompareMode::Materialized,
            fail_gates: false,
        };
        let results = crate::run_dataset(dataset, &config)?;
        assert_eq!(results.len(), 8);
        assert!(results.iter().all(|result| result.rows == 1));
        Ok(())
    }
}
