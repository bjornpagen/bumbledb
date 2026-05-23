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

mod csv_readers;
mod imdb;
mod job_field_helpers;
mod job_query_builders;
mod job_query_list;
mod job_schema;
mod job_sqlite;
mod job_stream;
mod lahman;
mod ldbc;
mod tpch;

use csv_readers::*;
use imdb::imdb_dataset;
use job_field_helpers::*;
use job_query_builders::*;
use job_query_list::job_queries;
use job_schema::job_schema;
use job_sqlite::{insert_job_sqlite, insert_job_sqlite_fact};
use job_stream::stream_job_facts;
use lahman::lahman_dataset;
use ldbc::{lahman_from_facts, ldbc_dataset, reached_limit, scaled_limit, super_tpch_dataset};
use tpch::tpch_open_dataset;
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
