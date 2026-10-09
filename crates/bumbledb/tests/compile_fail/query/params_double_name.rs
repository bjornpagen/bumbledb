//! `params!` takes each name once.
//@ error: params!: floor is supplied twice
//@ line: 22

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: uuid as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn bound() -> Vec<bumbledb::ParamArg<'static>> {
    let attempts_for = bumbledb::query!(Grades {
        (score) | Attempt(id, score), score > ?floor;
    });
    attempts_for.bind(bumbledb::params! {
        floor: 0.5f64,
        floor: 0.9f64
    })
}
