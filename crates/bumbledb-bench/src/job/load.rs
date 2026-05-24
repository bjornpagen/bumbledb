use std::collections::BTreeSet;
use std::path::Path;

use bumbledb_lmdb::{Fact, Value};
use csv::StringRecord;

pub(super) fn load_job_facts(dir: &Path, limit: Option<usize>) -> Result<Vec<Fact>, String> {
    let source_limit = limit.map(|limit| limit.saturating_mul(3));
    let small_limit = source_limit.map(|limit| (limit / 100).clamp(100, 10_000));
    let entity_limit = source_limit.map(|limit| (limit / 20).clamp(1_000, 50_000));
    let fact_limit = source_limit.map(|limit| (limit / 12).clamp(1_000, 100_000));
    let cast_limit = source_limit.map(|limit| (limit / 5).clamp(1_000, 200_000));
    let mut facts = Vec::new();
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

    read_job_csv(dir, "company_type.csv", small_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        company_types.insert(id);
        facts.push(Fact::new(
            "CompanyType",
            [("id", s(id)), ("kind", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "info_type.csv", small_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        info_types.insert(id);
        facts.push(Fact::new(
            "InfoType",
            [("id", s(id)), ("info", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "kind_type.csv", small_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        kind_types.insert(id);
        facts.push(Fact::new(
            "KindType",
            [("id", s(id)), ("kind", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "link_type.csv", small_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        link_types.insert(id);
        facts.push(Fact::new(
            "LinkType",
            [("id", s(id)), ("link", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "role_type.csv", small_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        role_types.insert(id);
        facts.push(Fact::new(
            "RoleType",
            [("id", s(id)), ("role", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "keyword.csv", entity_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        keywords.insert(id);
        facts.push(Fact::new(
            "Keyword",
            [("id", s(id)), ("keyword", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "company_name.csv", entity_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        companies.insert(id);
        facts.push(Fact::new(
            "CompanyName",
            [("id", s(id)), ("country_code", text(get(record, 2)))],
        ));
        true
    })?;
    read_job_csv(dir, "char_name.csv", entity_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        characters.insert(id);
        facts.push(Fact::new("CharName", [("id", s(id))]));
        true
    })?;
    read_job_csv(dir, "name.csv", entity_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        names.insert(id);
        facts.push(Fact::new(
            "Name",
            [("id", s(id)), ("gender", text(get(record, 4)))],
        ));
        true
    })?;
    read_job_csv(dir, "title.csv", entity_limit, |record| {
        let id = parse_u64(get(record, 0));
        let kind = parse_u64(get(record, 3));
        if id == 0 || !kind_types.contains(&kind) {
            return false;
        }
        titles.insert(id);
        facts.push(Fact::new(
            "Title",
            [
                ("id", s(id)),
                ("kind", s(kind)),
                ("production_year", i(get(record, 4))),
                ("episode_nr", i(get(record, 9))),
            ],
        ));
        true
    })?;

    read_job_csv(dir, "aka_name.csv", fact_limit, |record| {
        let person = parse_u64(get(record, 1));
        if !names.contains(&person) {
            return false;
        }
        facts.push(Fact::new("AkaName", [("person", s(person))]));
        true
    })?;
    read_job_csv(dir, "cast_info.csv", cast_limit, |record| {
        let person = parse_u64(get(record, 1));
        let movie = parse_u64(get(record, 2));
        let person_role = parse_u64(get(record, 3));
        let role = parse_u64(get(record, 6));
        if !(names.contains(&person)
            && titles.contains(&movie)
            && characters.contains(&person_role)
            && role_types.contains(&role))
        {
            return false;
        }
        facts.push(Fact::new(
            "CastInfo",
            [
                ("person", s(person)),
                ("movie", s(movie)),
                ("person_role", s(person_role)),
                ("role", s(role)),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "movie_companies.csv", fact_limit, |record| {
        let movie = parse_u64(get(record, 1));
        let company = parse_u64(get(record, 2));
        let company_type = parse_u64(get(record, 3));
        if !(titles.contains(&movie)
            && companies.contains(&company)
            && company_types.contains(&company_type))
        {
            return false;
        }
        facts.push(Fact::new(
            "MovieCompanies",
            [
                ("movie", s(movie)),
                ("company", s(company)),
                ("company_type", s(company_type)),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "movie_info_idx.csv", fact_limit, |record| {
        let movie = parse_u64(get(record, 1));
        let info_type = parse_u64(get(record, 2));
        if !(titles.contains(&movie) && info_types.contains(&info_type)) {
            return false;
        }
        facts.push(Fact::new(
            "MovieInfoIdx",
            [("movie", s(movie)), ("info_type", s(info_type))],
        ));
        true
    })?;
    read_job_csv(dir, "movie_info.csv", fact_limit, |record| {
        let movie = parse_u64(get(record, 1));
        let info_type = parse_u64(get(record, 2));
        if !(titles.contains(&movie) && info_types.contains(&info_type)) {
            return false;
        }
        facts.push(Fact::new(
            "MovieInfo",
            [("movie", s(movie)), ("info_type", s(info_type))],
        ));
        true
    })?;
    read_job_csv(dir, "movie_keyword.csv", fact_limit, |record| {
        let movie = parse_u64(get(record, 1));
        let keyword = parse_u64(get(record, 2));
        if !(titles.contains(&movie) && keywords.contains(&keyword)) {
            return false;
        }
        facts.push(Fact::new(
            "MovieKeyword",
            [("movie", s(movie)), ("keyword", s(keyword))],
        ));
        true
    })?;
    read_job_csv(dir, "movie_link.csv", fact_limit, |record| {
        let movie = parse_u64(get(record, 1));
        let linked_movie = parse_u64(get(record, 2));
        let link_type = parse_u64(get(record, 3));
        if !(titles.contains(&movie)
            && titles.contains(&linked_movie)
            && link_types.contains(&link_type))
        {
            return false;
        }
        facts.push(Fact::new(
            "MovieLink",
            [
                ("movie", s(movie)),
                ("linked_movie", s(linked_movie)),
                ("link_type", s(link_type)),
            ],
        ));
        true
    })?;

    Ok(facts)
}

fn read_job_csv(
    dir: &Path,
    file: &str,
    limit: Option<usize>,
    mut f: impl FnMut(&StringRecord) -> bool,
) -> Result<(), String> {
    let path = dir.join(file);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(&path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let mut accepted = 0usize;
    for record in reader.records() {
        if limit.is_some_and(|limit| accepted >= limit) {
            break;
        }
        let record = record.map_err(|error| format!("{}: {error}", path.display()))?;
        if f(&record) {
            accepted += 1;
        }
    }
    Ok(())
}

fn get(record: &StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or("")
}
fn text(value: &str) -> Value {
    Value::String(if value.is_empty() || value == r"\N" {
        String::new()
    } else {
        value.to_owned()
    })
}
fn s(value: u64) -> Value {
    Value::Serial(value)
}
fn i(value: &str) -> Value {
    Value::I64(value.parse().unwrap_or(0))
}
fn parse_u64(value: &str) -> u64 {
    value.parse().unwrap_or(0)
}
