use std::collections::BTreeSet;
use std::path::Path;

use bumbledb_lmdb::{Fact, Value};

use super::{
    get, job_text, parse_optional_i64, parse_optional_u64, parse_u64, read_job_csv, scaled_limit,
};

pub(super) fn stream_job_facts(
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
