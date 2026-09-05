//! A dense float interval literal is half-open and nonempty: start < end
//! strictly, after `-0` canonicalizes to `+0`.
//@ error: half-open and nonempty

bumbledb::schema! {
    pub Scores;

    relation Attempt {
        id: id128 as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Scores {
        (a) | Attempt(id: a, score: s), s in 1.5..0.5;
    })
    .into_query()
}
