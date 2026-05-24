use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use bumbledb_lmdb::{Fact, Value};

pub(super) fn load_sqlite(db_path: &Path, facts: &[Fact]) -> Result<(), String> {
    let script_path = db_path.with_extension("sql");
    if db_path.exists() {
        fs::remove_file(db_path).map_err(|error| error.to_string())?;
    }
    let mut script = String::new();
    script.push_str(SQLITE_SCHEMA);
    script.push_str("BEGIN;\n");
    for fact in facts {
        push_insert(&mut script, fact)?;
    }
    script.push_str("COMMIT;\nANALYZE;\n");
    fs::write(&script_path, script).map_err(|error| error.to_string())?;
    let status = Command::new("sqlite3")
        .arg(db_path)
        .arg(format!(".read {}", script_path.display()))
        .stdout(Stdio::null())
        .status()
        .map_err(|error| error.to_string())?;
    let _ = fs::remove_file(script_path);
    if status.success() {
        Ok(())
    } else {
        Err("sqlite load failed".to_owned())
    }
}

pub(super) fn query_sqlite(db_path: &Path, sql: &str) -> Result<Vec<Vec<Value>>, String> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    Ok(stdout
        .lines()
        .map(|line| {
            line.split('|')
                .map(|value| Value::Serial(value.parse().unwrap_or(0)))
                .collect()
        })
        .collect())
}

fn push_insert(out: &mut String, fact: &Fact) -> Result<(), String> {
    match fact.relation() {
        "CompanyName" => insert(out, "company_name", fact, &["id", "country_code"]),
        "CompanyType" => insert(out, "company_type", fact, &["id", "kind"]),
        "InfoType" => insert(out, "info_type", fact, &["id", "info"]),
        "Keyword" => insert(out, "keyword", fact, &["id", "keyword"]),
        "KindType" => insert(out, "kind_type", fact, &["id", "kind"]),
        "LinkType" => insert(out, "link_type", fact, &["id", "link"]),
        "RoleType" => insert(out, "role_type", fact, &["id", "role"]),
        "CharName" => insert(out, "char_name", fact, &["id"]),
        "Name" => insert(out, "name", fact, &["id", "gender"]),
        "Title" => insert(
            out,
            "title",
            fact,
            &["id", "kind", "production_year", "episode_nr"],
        ),
        "AkaName" => insert(out, "aka_name", fact, &["person"]),
        "CastInfo" => insert(
            out,
            "cast_info",
            fact,
            &["person", "movie", "person_role", "role"],
        ),
        "MovieCompanies" => insert(
            out,
            "movie_companies",
            fact,
            &["movie", "company", "company_type"],
        ),
        "MovieInfo" => insert(out, "movie_info", fact, &["movie", "info_type"]),
        "MovieInfoIdx" => insert(out, "movie_info_idx", fact, &["movie", "info_type"]),
        "MovieKeyword" => insert(out, "movie_keyword", fact, &["movie", "keyword"]),
        "MovieLink" => insert(
            out,
            "movie_link",
            fact,
            &["movie", "linked_movie", "link_type"],
        ),
        _ => Ok(()),
    }
}

fn insert(out: &mut String, table: &str, fact: &Fact, fields: &[&str]) -> Result<(), String> {
    out.push_str("INSERT INTO ");
    out.push_str(table);
    out.push_str(" VALUES(");
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_sql_value(
            out,
            fact.value(field)
                .ok_or_else(|| format!("missing {field}"))?,
        );
    }
    out.push_str(");\n");
    Ok(())
}

fn push_sql_value(out: &mut String, value: &Value) {
    match value {
        Value::Bool(value) => out.push_str(if *value { "1" } else { "0" }),
        Value::U64(value) | Value::Serial(value) => out.push_str(&value.to_string()),
        Value::I64(value) => out.push_str(&value.to_string()),
        Value::Enum(value) => out.push_str(&value.to_string()),
        Value::String(value) => {
            out.push('\'');
            out.push_str(&value.replace('\'', "''"));
            out.push('\'');
        }
        Value::Bytes(value) => {
            out.push('\'');
            out.push_str(&String::from_utf8_lossy(value).replace('\'', "''"));
            out.push('\'');
        }
    }
}

const SQLITE_SCHEMA: &str = r#"
CREATE TABLE company_name (id INTEGER PRIMARY KEY, country_code TEXT NOT NULL);
CREATE TABLE company_type (id INTEGER PRIMARY KEY, kind TEXT NOT NULL);
CREATE TABLE info_type (id INTEGER PRIMARY KEY, info TEXT NOT NULL);
CREATE TABLE keyword (id INTEGER PRIMARY KEY, keyword TEXT NOT NULL);
CREATE TABLE kind_type (id INTEGER PRIMARY KEY, kind TEXT NOT NULL);
CREATE TABLE link_type (id INTEGER PRIMARY KEY, link TEXT NOT NULL);
CREATE TABLE role_type (id INTEGER PRIMARY KEY, role TEXT NOT NULL);
CREATE TABLE char_name (id INTEGER PRIMARY KEY);
CREATE TABLE name (id INTEGER PRIMARY KEY, gender TEXT NOT NULL);
CREATE TABLE title (id INTEGER PRIMARY KEY, kind INTEGER NOT NULL, production_year INTEGER NOT NULL, episode_nr INTEGER NOT NULL);
CREATE TABLE aka_name (person INTEGER NOT NULL);
CREATE TABLE cast_info (person INTEGER NOT NULL, movie INTEGER NOT NULL, person_role INTEGER NOT NULL, role INTEGER NOT NULL);
CREATE TABLE movie_companies (movie INTEGER NOT NULL, company INTEGER NOT NULL, company_type INTEGER NOT NULL);
CREATE TABLE movie_info (movie INTEGER NOT NULL, info_type INTEGER NOT NULL);
CREATE TABLE movie_info_idx (movie INTEGER NOT NULL, info_type INTEGER NOT NULL);
CREATE TABLE movie_keyword (movie INTEGER NOT NULL, keyword INTEGER NOT NULL);
CREATE TABLE movie_link (movie INTEGER NOT NULL, linked_movie INTEGER NOT NULL, link_type INTEGER NOT NULL);
CREATE INDEX cast_info_movie ON cast_info(movie, role, person, person_role);
CREATE INDEX movie_companies_movie ON movie_companies(movie, company_type, company);
CREATE INDEX movie_keyword_movie ON movie_keyword(movie, keyword);
CREATE INDEX movie_info_idx_movie_type ON movie_info_idx(movie, info_type);
CREATE INDEX movie_info_movie_type ON movie_info(movie, info_type);
"#;
