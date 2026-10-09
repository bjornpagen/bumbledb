//! `bind` needs every named param.
//@ error: __BumbledbUnset
//@ line: 21

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: uuid as AttemptId,
        student: uuid as StudentId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn bound() -> Vec<bumbledb::ParamArg<'static>> {
    let attempts_for = bumbledb::query!(Grades {
        (score) | Attempt(id, student == ?student, score), score > ?floor;
    });
    attempts_for.bind(bumbledb::params! {
        student: bumbledb::Uuid::from_bytes([1; 16])
    })
}
