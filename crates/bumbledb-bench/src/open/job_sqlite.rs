use bumbledb_lmdb::Fact;
use rusqlite::Connection;

use crate::{i64v, id, rf, text, u64v};

pub(super) fn insert_job_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        insert_job_sqlite_fact(&tx, fact)?;
    }
    tx.commit()?;
    Ok(())
}

pub(super) fn insert_job_sqlite_fact(
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
