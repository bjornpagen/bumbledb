use bumbledb::Value;

use super::{ids, mix, COMPANIES, HOT_KEYWORDS, HOT_PEOPLE, KEYWORDS, KINDS, MOVIES, PEOPLE};
use crate::gen::Rng;

pub(super) fn s(text: String) -> Value {
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
pub(super) fn distinct_rows(seed: u64, rel: bumbledb::RelationId, n: u64) -> Vec<Vec<Value>> {
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

pub(super) fn boxed(
    seed: u64,
    rel: bumbledb::RelationId,
    n: u64,
) -> Box<dyn Iterator<Item = Vec<Value>>> {
    Box::new((0..n).map(move |i| row(seed, rel, i)))
}
