//! A float interval literal is half-open and nonempty.
//@ error: half-open and nonempty
//@ line: 18

bumbledb::schema! {
    pub Scores;

    relation Attempt {
        id: uuid as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Scores {
        (a) | Attempt(id: a, score: s), s in 1.5..0.5;
    })
    .into_query()
}
