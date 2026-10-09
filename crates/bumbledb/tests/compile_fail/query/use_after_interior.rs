//! `use` imports precede every rule.
//@ error: `use` imports precede every rule
//@ line: 22

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: uuid as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    let base = bumbledb::query!(Grades {
        (a) | Attempt(id: a, score: s);
    });
    bumbledb::query!(Grades {
        interior good(a) | Attempt(id: a, score: s), s > 0.5;
        use b = &base;
        (a) | good(a), b(a, s);
    })
    .into_query()
}
