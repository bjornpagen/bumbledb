//! The join-order stress scenario (JOB-inspired): an IMDb-shaped
//! snowflake with skewed fan-ins and correlated predicates. The Join
//! Order Benchmark's thesis is that realistic, correlated, skewed data
//! punishes bad join orders by orders of magnitude — this is that
//! pressure, expressed in the engine's conjunctive subset (no LIKE, no
//! OR, no outer joins; selectivity comes from closed vocabularies,
//! ranges, and interned-string points instead).

use super::{Scenario, ScenarioQuery, mix};
use bumbledb::schema::ValidateDescriptor as _;

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
        id: u64 as JKindId, fresh,
        name: str,
    }
    relation Company {
        id: u64 as JCompanyId, fresh,
        name: str,
        country: u64 as JCountryId,
    }
    relation Person {
        id: u64 as JPersonId, fresh,
        name: str,
        gender: u64 as JGenderId,
    }
    relation Keyword {
        id: u64 as JKeywordId, fresh,
        word: str,
    }
    relation Movie {
        id: u64 as JMovieId, fresh,
        title: str,
        year: i64,
        kind: u64 as JKindId,
    }
    relation CastInfo {
        movie: u64 as JMovieId,
        person: u64 as JPersonId,
        role: u64 as JRoleId,
    }
    relation MovieCompany {
        movie: u64 as JMovieId,
        company: u64 as JCompanyId,
    }
    relation MovieKeyword {
        movie: u64 as JMovieId,
        keyword: u64 as JKeywordId,
    }

    closed relation Country as JCountryId = { Us, Uk, De, Fr, Jp, In, Br, Kr };
    closed relation Gender as JGenderId = { F, M, X };
    closed relation Role as JRoleId = {
        Actor, Actress, Director, Producer, Writer, Composer, Editor, Extra,
    };

    Kind(name) -> Kind;
    Keyword(word) -> Keyword;
    Movie(kind) <= Kind(id);
    Company(country) <= Country(id);
    Person(gender) <= Gender(id);
    CastInfo(movie) <= Movie(id);
    CastInfo(person) <= Person(id);
    CastInfo(role) <= Role(id);
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
    use bumbledb::Theory as _;
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
    pub const COUNTRY: RelationId = RelationId(8);
    pub const GENDER: RelationId = RelationId(9);
    pub const ROLE: RelationId = RelationId(10);
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
