use super::*;

pub(super) fn job_string_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::String)
}

pub(super) fn job_i64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::I64)
}

pub(super) fn job_range_i64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::I64).range_indexed()
}

pub(super) fn job_u64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::U64)
}

pub(super) const JOB_SQLITE_SCHEMA: &str = r#"
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
