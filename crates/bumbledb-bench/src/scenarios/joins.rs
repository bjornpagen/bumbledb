//! The join-order stress scenario (JOB-inspired): an IMDb-shaped
//! snowflake with skewed fan-ins and correlated predicates. The Join
//! Order Benchmark's thesis is that realistic, correlated, skewed data
//! punishes bad join orders by orders of magnitude — this is that
//! pressure, expressed in the engine's conjunctive subset (no LIKE, no
//! OR, no outer joins; selectivity comes from enums, ranges, and
//! interned-string points instead).

use super::{mix, Scenario, ScenarioQuery};

mod corpus;
mod costars;
mod country_rollup;
mod filmography;
mod five_way;
mod keyword_kind;
mod keyword_neighborhood;
mod person_params;
mod scenario;
mod term;

pub use scenario::scenario;

bumbledb::schema! {
    pub Joins;

    relation Kind {
        id: u64 as JKindId, serial,
        name: str,
    }
    relation Company {
        id: u64 as JCompanyId, serial,
        name: str,
        country: enum Country { Us, Uk, De, Fr, Jp, In, Br, Kr },
    }
    relation Person {
        id: u64 as JPersonId, serial,
        name: str,
        gender: enum Gender { F, M, X },
    }
    relation Keyword {
        id: u64 as JKeywordId, serial,
        word: str,
    }
    relation Movie {
        id: u64 as JMovieId, serial,
        title: str,
        year: i64,
        kind: u64 as JKindId,
    }
    relation CastInfo {
        movie: u64 as JMovieId,
        person: u64 as JPersonId,
        role: enum Role { Actor, Actress, Director, Producer, Writer, Composer, Editor, Extra },
    }
    relation MovieCompany {
        movie: u64 as JMovieId,
        company: u64 as JCompanyId,
    }
    relation MovieKeyword {
        movie: u64 as JMovieId,
        keyword: u64 as JKeywordId,
    }

    Kind(name) -> Kind;
    Keyword(word) -> Keyword;
    Movie(kind) <= Kind(id);
    CastInfo(movie) <= Movie(id);
    CastInfo(person) <= Person(id);
    CastInfo(movie, person) -> CastInfo;
    MovieCompany(movie) <= Movie(id);
    MovieCompany(company) <= Company(id);
    MovieCompany(movie, company) -> MovieCompany;
    MovieKeyword(movie) <= Movie(id);
    MovieKeyword(keyword) <= Keyword(id);
    MovieKeyword(movie, keyword) -> MovieKeyword;
}

/// Relation ids by declaration order.
/// The validated scenario schema, memoized for the inspection surfaces
/// (DDL rendering, typing); the store is created from [`Joins`]'s
/// descriptor (`scenarios::load`).
///
/// # Panics
///
/// Never in practice: the declared scenario schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::SchemaDef as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Joins
            .descriptor()
            .validate()
            .expect("the scenario schema is valid")
    })
}

pub mod ids {
    use bumbledb::RelationId;
    pub const KIND: RelationId = RelationId(0);
    pub const COMPANY: RelationId = RelationId(1);
    pub const PERSON: RelationId = RelationId(2);
    pub const KEYWORD: RelationId = RelationId(3);
    pub const MOVIE: RelationId = RelationId(4);
    pub const CAST_INFO: RelationId = RelationId(5);
    pub const MOVIE_COMPANY: RelationId = RelationId(6);
    pub const MOVIE_KEYWORD: RelationId = RelationId(7);
}

/// Sizes (fixed — the scenario is one world, not a scale axis).
pub const KINDS: u64 = 7;
pub const COMPANIES: u64 = 5_000;
pub const PEOPLE: u64 = 50_000;
pub const KEYWORDS: u64 = 10_000;
pub const MOVIES: u64 = 25_000;
pub const CASTS: u64 = 250_000;
pub const MOVIE_COMPANIES: u64 = 75_000;
pub const MOVIE_KEYWORDS: u64 = 150_000;

/// Hot rows: the skew knobs. 1% of people take ~25% of cast rows;
/// 1% of keywords take ~25% of keyword rows (power-law-ish fan-in,
/// the JOB pressure).
const HOT_PEOPLE: u64 = PEOPLE / 100;
const HOT_KEYWORDS: u64 = KEYWORDS / 100;
