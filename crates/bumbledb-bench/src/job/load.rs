use std::collections::BTreeSet;
use std::path::Path;

use bumbledb_lmdb::{Fact, Value};
use csv::StringRecord;

pub(super) fn load_job_facts(dir: &Path, limit: Option<usize>) -> Result<Vec<Fact>, String> {
    let dim_limit = scaled_limit(limit, 20);
    let name_limit = scaled_limit(limit, 10);
    let fact_limit = scaled_limit(limit, 40);
    let cast_limit = scaled_limit(limit, 80);
    let mut facts = Vec::new();
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

    read_job_csv(dir, "comp_cast_type.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        comp_cast_types.insert(id);
        facts.push(fact(
            "CompCastType",
            [("id", s(id)), ("kind", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "company_type.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        company_types.insert(id);
        facts.push(fact(
            "CompanyType",
            [("id", s(id)), ("kind", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "info_type.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        info_types.insert(id);
        facts.push(fact(
            "InfoType",
            [("id", s(id)), ("info", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "kind_type.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        kind_types.insert(id);
        facts.push(fact(
            "KindType",
            [("id", s(id)), ("kind", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "link_type.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        link_types.insert(id);
        facts.push(fact(
            "LinkType",
            [("id", s(id)), ("link", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "role_type.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        role_types.insert(id);
        facts.push(fact(
            "RoleType",
            [("id", s(id)), ("role", text(get(record, 1)))],
        ));
        true
    })?;
    read_job_csv(dir, "keyword.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        keywords.insert(id);
        facts.push(Fact::new(
            "Keyword",
            [
                ("id", s(id)),
                ("keyword", text(get(record, 1))),
                ("phonetic_code", text(get(record, 2))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "company_name.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        companies.insert(id);
        facts.push(Fact::new(
            "CompanyName",
            [
                ("id", s(id)),
                ("name", text(get(record, 1))),
                ("country_code", text(get(record, 2))),
                ("imdb_id", i(get(record, 3))),
                ("name_pcode_nf", text(get(record, 4))),
                ("name_pcode_sf", text(get(record, 5))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "char_name.csv", dim_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        characters.insert(id);
        facts.push(Fact::new(
            "CharName",
            [
                ("id", s(id)),
                ("name", text(get(record, 1))),
                ("imdb_index", text(get(record, 2))),
                ("imdb_id", i(get(record, 3))),
                ("name_pcode_nf", text(get(record, 4))),
                ("surname_pcode", text(get(record, 5))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "name.csv", name_limit, |record| {
        let id = parse_u64(get(record, 0));
        if id == 0 {
            return false;
        }
        names.insert(id);
        facts.push(Fact::new(
            "Name",
            [
                ("id", s(id)),
                ("name", text(get(record, 1))),
                ("imdb_index", text(get(record, 2))),
                ("imdb_id", i(get(record, 3))),
                ("gender", text(get(record, 4))),
                ("name_pcode_cf", text(get(record, 5))),
                ("name_pcode_nf", text(get(record, 6))),
                ("surname_pcode", text(get(record, 7))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "title.csv", limit, |record| {
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
                ("title", text(get(record, 1))),
                ("imdb_index", text(get(record, 2))),
                ("kind", s(kind)),
                ("production_year", i(get(record, 4))),
                ("imdb_id", i(get(record, 5))),
                ("phonetic_code", text(get(record, 6))),
                ("episode_of", u(get(record, 7))),
                ("season_nr", i(get(record, 8))),
                ("episode_nr", i(get(record, 9))),
                ("series_years", text(get(record, 10))),
            ],
        ));
        true
    })?;

    read_job_csv(dir, "aka_name.csv", fact_limit, |record| {
        let id = parse_u64(get(record, 0));
        let person = parse_u64(get(record, 1));
        if id == 0 || !names.contains(&person) {
            return false;
        }
        facts.push(Fact::new(
            "AkaName",
            [
                ("id", s(id)),
                ("person", s(person)),
                ("name", text(get(record, 2))),
                ("imdb_index", text(get(record, 3))),
                ("name_pcode_cf", text(get(record, 4))),
                ("name_pcode_nf", text(get(record, 5))),
                ("surname_pcode", text(get(record, 6))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "cast_info.csv", cast_limit, |record| {
        let id = parse_u64(get(record, 0));
        let person = parse_u64(get(record, 1));
        let movie = parse_u64(get(record, 2));
        let person_role = parse_u64(get(record, 3));
        let role = parse_u64(get(record, 6));
        if id == 0
            || !(names.contains(&person)
                && titles.contains(&movie)
                && characters.contains(&person_role)
                && role_types.contains(&role))
        {
            return false;
        }
        facts.push(Fact::new(
            "CastInfo",
            [
                ("id", s(id)),
                ("person", s(person)),
                ("movie", s(movie)),
                ("person_role", s(person_role)),
                ("note", text(get(record, 4))),
                ("nr_order", i(get(record, 5))),
                ("role", s(role)),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "movie_companies.csv", fact_limit, |record| {
        let id = parse_u64(get(record, 0));
        let movie = parse_u64(get(record, 1));
        let company = parse_u64(get(record, 2));
        let company_type = parse_u64(get(record, 3));
        if id == 0
            || !(titles.contains(&movie)
                && companies.contains(&company)
                && company_types.contains(&company_type))
        {
            return false;
        }
        facts.push(Fact::new(
            "MovieCompanies",
            [
                ("id", s(id)),
                ("movie", s(movie)),
                ("company", s(company)),
                ("company_type", s(company_type)),
                ("note", text(get(record, 4))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "movie_info_idx.csv", fact_limit, |record| {
        let id = parse_u64(get(record, 0));
        let movie = parse_u64(get(record, 1));
        let info_type = parse_u64(get(record, 2));
        if id == 0 || !(titles.contains(&movie) && info_types.contains(&info_type)) {
            return false;
        }
        facts.push(Fact::new(
            "MovieInfoIdx",
            [
                ("id", s(id)),
                ("movie", s(movie)),
                ("info_type", s(info_type)),
                ("info", text(get(record, 3))),
                ("note", text(get(record, 4))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "movie_info.csv", fact_limit, |record| {
        let id = parse_u64(get(record, 0));
        let movie = parse_u64(get(record, 1));
        let info_type = parse_u64(get(record, 2));
        if id == 0 || !(titles.contains(&movie) && info_types.contains(&info_type)) {
            return false;
        }
        facts.push(Fact::new(
            "MovieInfo",
            [
                ("id", s(id)),
                ("movie", s(movie)),
                ("info_type", s(info_type)),
                ("info", text(get(record, 3))),
                ("note", text(get(record, 4))),
            ],
        ));
        true
    })?;
    read_job_csv(dir, "movie_keyword.csv", fact_limit, |record| {
        let id = parse_u64(get(record, 0));
        let movie = parse_u64(get(record, 1));
        let keyword = parse_u64(get(record, 2));
        if id == 0 || !(titles.contains(&movie) && keywords.contains(&keyword)) {
            return false;
        }
        facts.push(Fact::new(
            "MovieKeyword",
            [("id", s(id)), ("movie", s(movie)), ("keyword", s(keyword))],
        ));
        true
    })?;
    read_job_csv(dir, "movie_link.csv", fact_limit, |record| {
        let id = parse_u64(get(record, 0));
        let movie = parse_u64(get(record, 1));
        let linked_movie = parse_u64(get(record, 2));
        let link_type = parse_u64(get(record, 3));
        if id == 0
            || !(titles.contains(&movie)
                && titles.contains(&linked_movie)
                && link_types.contains(&link_type))
        {
            return false;
        }
        facts.push(Fact::new(
            "MovieLink",
            [
                ("id", s(id)),
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

fn scaled_limit(limit: Option<usize>, multiplier: usize) -> Option<usize> {
    limit.map(|limit| limit.saturating_mul(multiplier).max(limit))
}

fn get(record: &StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or("")
}
fn text(value: &str) -> Value {
    if value.is_empty() || value == r"\N" {
        return Value::String(String::new());
    }
    if value.len() > 240 {
        return Value::String(format!(
            "#long: {}",
            blake3::hash(value.as_bytes()).to_hex()
        ));
    }
    Value::String(value.to_owned())
}
fn s(value: u64) -> Value {
    Value::Serial(value)
}
fn u(value: &str) -> Value {
    Value::U64(parse_u64(value))
}
fn i(value: &str) -> Value {
    Value::I64(value.parse().unwrap_or(0))
}
fn parse_u64(value: &str) -> u64 {
    value.parse().unwrap_or(0)
}
fn fact<const N: usize>(relation: &str, values: [(&str, Value); N]) -> Fact {
    Fact::new(relation, values)
}
