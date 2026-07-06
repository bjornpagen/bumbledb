//! The join-order stress scenario (JOB-inspired): an IMDb-shaped
//! snowflake with skewed fan-ins and correlated predicates. The Join
//! Order Benchmark's thesis is that realistic, correlated, skewed data
//! punishes bad join orders by orders of magnitude — this is that
//! pressure, expressed in the engine's conjunctive subset (no LIKE, no
//! OR, no outer joins; selectivity comes from enums, ranges, and
//! interned-string points instead).

use bumbledb::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};

use super::{mix, Scenario, ScenarioQuery};
use crate::gen::Rng;

bumbledb::schema! {
    relation Kind {
        id: u64 as JKindId, serial,
        name: str,
        unique(name),
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
        unique(word),
    }
    relation Movie {
        id: u64 as JMovieId, serial,
        title: str,
        year: i64,
        kind: u64 as JKindId, fk(Kind.id),
    }
    relation CastInfo {
        movie: u64 as JMovieId, fk(Movie.id),
        person: u64 as JPersonId, fk(Person.id),
        role: enum Role { Actor, Actress, Director, Producer, Writer, Composer, Editor, Extra },
        unique(movie, person),
    }
    relation MovieCompany {
        movie: u64 as JMovieId, fk(Movie.id),
        company: u64 as JCompanyId, fk(Company.id),
        unique(movie, company),
    }
    relation MovieKeyword {
        movie: u64 as JMovieId, fk(Movie.id),
        keyword: u64 as JKeywordId, fk(Keyword.id),
        unique(movie, keyword),
    }
}

/// Relation ids by declaration order.
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

fn s(text: String) -> Value {
    Value::String(text.into_bytes().into())
}

/// One deterministic row (pure in (seed, relation, index) — params can
/// recompute any row's content).
#[allow(clippy::cast_possible_truncation)]
fn row(seed: u64, rel: bumbledb::RelationId, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, rel.0, i));
    match rel {
        ids::KIND => vec![Value::U64(i), s(format!("kind-{i}"))],
        ids::COMPANY => vec![
            Value::U64(i),
            s(format!("company-{i:05}")),
            Value::Enum((i % 8) as u8),
        ],
        ids::PERSON => vec![
            Value::U64(i),
            s(format!("person-{i:06}")),
            Value::Enum((rng.range(3)) as u8),
        ],
        ids::KEYWORD => vec![Value::U64(i), s(format!("kw-{i:05}"))],
        ids::MOVIE => {
            // Year correlates with kind: kind k spans a 30-year band —
            // the correlated-predicate trap (year filters and kind
            // filters are NOT independent, exactly what static
            // independence assumptions get wrong).
            let kind = rng.range(KINDS);
            let band_start = 1930 + i64::try_from(kind).expect("small") * 10;
            let year = band_start + i64::try_from(rng.range(40)).expect("small");
            vec![
                Value::U64(i),
                s(format!("movie-{i:06}")),
                Value::I64(year),
                Value::U64(kind),
            ]
        }
        ids::CAST_INFO => {
            // (movie, person) unique: derive both from i injectively,
            // with the skew on the person side.
            let movie = i % MOVIES;
            let person = if rng.chance(1, 4) {
                rng.range(HOT_PEOPLE)
            } else {
                HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE)
            };
            // Uniqueness fix-up: mix i into the person draw's low bits
            // deterministically; collisions on (movie, person) are
            // deduplicated by set semantics on the engine side, so the
            // pair must be injective for the SQLite mirror. Derive
            // person from a per-movie slot instead.
            let slot = i / MOVIES;
            let person = (person + slot) % PEOPLE;
            vec![
                Value::U64(movie),
                Value::U64(person),
                Value::Enum((rng.range(8)) as u8),
            ]
        }
        ids::MOVIE_COMPANY => {
            let movie = i % MOVIES;
            let slot = i / MOVIES;
            let company = (rng.range(COMPANIES) + slot) % COMPANIES;
            vec![Value::U64(movie), Value::U64(company)]
        }
        ids::MOVIE_KEYWORD => {
            let movie = i % MOVIES;
            let slot = i / MOVIES;
            let keyword = if rng.chance(1, 4) {
                rng.range(HOT_KEYWORDS)
            } else {
                HOT_KEYWORDS + rng.range(KEYWORDS - HOT_KEYWORDS)
            };
            let keyword = (keyword + slot) % KEYWORDS;
            vec![Value::U64(movie), Value::U64(keyword)]
        }
        other => unreachable!("no such relation {other:?}"),
    }
}

/// Compound-unique relations can collide on their derived pairs; the
/// loader deduplicates per relation so both engines load the identical
/// fact set (set semantics native on ours, INSERT OR IGNORE-free on
/// theirs).
fn distinct_rows(seed: u64, rel: bumbledb::RelationId, n: u64) -> Vec<Vec<Value>> {
    let mut loaded = std::collections::HashSet::new();
    let mut out = Vec::new();
    for i in 0..n {
        let r = row(seed, rel, i);
        let key = (r[0].clone(), r[1].clone());
        if loaded.insert(format!("{key:?}")) {
            out.push(r);
        }
    }
    out
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// j1 — one hot person, one cold person, one mid, one miss: fan-in skew
/// on a 2-atom FK walk.
fn filmography() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(1), param(0)), (FieldId(0), var(2))],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![
                    (FieldId(0), var(2)),
                    (FieldId(1), var(0)),
                    (FieldId(2), var(1)),
                ],
            },
        ],
        predicates: vec![],
    }
}

use bumbledb::FieldId;

fn filmography_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, 1));
    vec![
        vec![Value::U64(rng.range(HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(PEOPLE + 1_000_000)],
    ]
}

/// j2 — costars: the self-join through a shared movie, hot vs cold.
fn costars() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), param(0))],
            },
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(0))],
            },
        ],
        predicates: vec![],
    }
}

/// j3 — keyword × kind: two interned-string/enum-selective dimensions
/// pinching a 3-way join from both sides.
fn keyword_kind() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::KEYWORD,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), param(0))],
            },
            Atom {
                relation: ids::MOVIE_KEYWORD,
                bindings: vec![(FieldId(1), var(2)), (FieldId(0), var(3))],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![
                    (FieldId(0), var(3)),
                    (FieldId(1), var(0)),
                    (FieldId(2), var(1)),
                    (FieldId(3), var(4)),
                ],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: var(1),
            rhs: param(1),
        }],
    }
}

fn keyword_kind_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, 3));
    let kw = |k: u64| s(format!("kw-{k:05}"));
    vec![
        vec![kw(rng.range(HOT_KEYWORDS)), Value::I64(1980)],
        vec![
            kw(HOT_KEYWORDS + rng.range(KEYWORDS - HOT_KEYWORDS)),
            Value::I64(1960),
        ],
        vec![
            kw(HOT_KEYWORDS + rng.range(KEYWORDS - HOT_KEYWORDS)),
            Value::I64(2000),
        ],
        vec![s("kw-never-a-keyword".to_owned()), Value::I64(1980)],
    ]
}

/// j4 — the JOB-shaped 5-way: fact table pinched by three dimension
/// filters (gender, country, year window) on alternating sides.
fn five_way() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(3))],
            },
            Atom {
                relation: ids::PERSON,
                bindings: vec![
                    (FieldId(0), var(3)),
                    (FieldId(1), var(0)),
                    (FieldId(2), param(0)),
                ],
            },
            Atom {
                relation: ids::MOVIE_COMPANY,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(4))],
            },
            Atom {
                relation: ids::COMPANY,
                bindings: vec![
                    (FieldId(0), var(4)),
                    (FieldId(1), var(1)),
                    (FieldId(2), param(1)),
                ],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![(FieldId(0), var(2)), (FieldId(2), var(5))],
            },
        ],
        predicates: vec![
            Comparison {
                op: CmpOp::Ge,
                lhs: var(5),
                rhs: param(2),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(5),
                rhs: param(3),
            },
        ],
    }
}

fn five_way_params(_: u64) -> Vec<Vec<Value>> {
    // Gender enum, country enum, year window: tight, mid, wide, empty.
    vec![
        vec![
            Value::Enum(0),
            Value::Enum(2),
            Value::I64(1990),
            Value::I64(1995),
        ],
        vec![
            Value::Enum(1),
            Value::Enum(0),
            Value::I64(1970),
            Value::I64(1990),
        ],
        vec![
            Value::Enum(2),
            Value::Enum(5),
            Value::I64(1930),
            Value::I64(2020),
        ],
        vec![
            Value::Enum(0),
            Value::Enum(7),
            Value::I64(2020),
            Value::I64(1930),
        ],
    ]
}

/// j5 — kind/country rollup over the full join: Min(year) and Count per
/// (country) — the aggregate face of join-order stress.
fn country_rollup() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::MOVIE_COMPANY,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(3))],
            },
            Atom {
                relation: ids::COMPANY,
                bindings: vec![(FieldId(0), var(3)), (FieldId(2), var(0))],
            },
            Atom {
                relation: ids::MOVIE,
                bindings: vec![(FieldId(0), var(2)), (FieldId(2), var(1))],
            },
        ],
        predicates: vec![],
    }
}

/// j6 — keyword neighborhood: movies sharing any keyword with a
/// person's movies — the fan-out explosion a bad order makes fatal.
fn keyword_neighborhood() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::CAST_INFO,
                bindings: vec![(FieldId(1), param(0)), (FieldId(0), var(1))],
            },
            Atom {
                relation: ids::MOVIE_KEYWORD,
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(2))],
            },
            Atom {
                relation: ids::MOVIE_KEYWORD,
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(2))],
            },
        ],
        predicates: vec![],
    }
}

fn person_params(seed: u64, salt: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, salt));
    vec![
        vec![Value::U64(rng.range(HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(PEOPLE + 1_000_000)],
    ]
}

/// The scenario registration.
#[must_use]
pub fn scenario() -> Scenario {
    Scenario {
        name: "joins",
        about: "JOB-style join-order stress: skewed fan-ins, correlated predicates",
        schema,
        rows: |seed| {
            vec![
                (ids::KIND, boxed(seed, ids::KIND, KINDS)),
                (ids::COMPANY, boxed(seed, ids::COMPANY, COMPANIES)),
                (ids::PERSON, boxed(seed, ids::PERSON, PEOPLE)),
                (ids::KEYWORD, boxed(seed, ids::KEYWORD, KEYWORDS)),
                (ids::MOVIE, boxed(seed, ids::MOVIE, MOVIES)),
                (
                    ids::CAST_INFO,
                    Box::new(distinct_rows(seed, ids::CAST_INFO, CASTS).into_iter()),
                ),
                (
                    ids::MOVIE_COMPANY,
                    Box::new(distinct_rows(seed, ids::MOVIE_COMPANY, MOVIE_COMPANIES).into_iter()),
                ),
                (
                    ids::MOVIE_KEYWORD,
                    Box::new(distinct_rows(seed, ids::MOVIE_KEYWORD, MOVIE_KEYWORDS).into_iter()),
                ),
            ]
        },
        extra_indexes: &[
            "CREATE INDEX ix_movie_year ON \"Movie\"(\"year\")",
            "CREATE INDEX ix_movie_kind ON \"Movie\"(\"kind\")",
            "CREATE INDEX ix_cast_person ON \"CastInfo\"(\"person\")",
            "CREATE INDEX ix_mk_keyword ON \"MovieKeyword\"(\"keyword\")",
            "CREATE INDEX ix_mc_company ON \"MovieCompany\"(\"company\")",
            "CREATE INDEX ix_person_gender ON \"Person\"(\"gender\")",
            "CREATE INDEX ix_company_country ON \"Company\"(\"country\")",
        ],
        queries: || {
            vec![
                ScenarioQuery {
                    name: "j1_filmography",
                    query: filmography,
                    params: filmography_params,
                    about: "2-atom FK walk under 25%-hot fan-in skew",
                },
                ScenarioQuery {
                    name: "j2_costars",
                    query: costars,
                    params: |seed| person_params(seed, 2),
                    about: "self-join through the fact table, hot vs cold",
                },
                ScenarioQuery {
                    name: "j3_keyword_kind",
                    query: keyword_kind,
                    params: keyword_kind_params,
                    about: "3-way pinched by string point + year range",
                },
                ScenarioQuery {
                    name: "j4_five_way",
                    query: five_way,
                    params: five_way_params,
                    about: "JOB-shaped 5-way, dims filter both sides",
                },
                ScenarioQuery {
                    name: "j5_country_rollup",
                    query: country_rollup,
                    params: |_| vec![vec![]],
                    about: "full-join rollup: Min(year)+Count by country",
                },
                ScenarioQuery {
                    name: "j6_keyword_neighborhood",
                    query: keyword_neighborhood,
                    params: |seed| person_params(seed, 6),
                    about: "fan-out explosion through shared keywords",
                },
            ]
        },
    }
}

fn boxed(seed: u64, rel: bumbledb::RelationId, n: u64) -> Box<dyn Iterator<Item = Vec<Value>>> {
    Box::new((0..n).map(move |i| row(seed, rel, i)))
}
