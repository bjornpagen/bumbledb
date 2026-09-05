//! `use` imports precede every rule: declaration order is imports, then
//! interiors, then rec, then main.
//@ error: `use` imports precede every rule

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: id128 as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    let base = bumbledb_query::query!(Grades {
        (a) | Attempt(id: a, score: s);
    });
    bumbledb_query::query!(Grades {
        interior good(a) | Attempt(id: a, score: s), s > 0.5;
        use b = &base;
        (a) | good(a), b(a, s);
    })
    .into_query()
}
