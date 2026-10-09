//! `bind` knows only the template's params.
//@ error: no method named `student_typo`
//@ line: 22

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
        (score) | Attempt(id, student == ?student, score);
    });
    attempts_for.bind(bumbledb::params! {
        student_typo: bumbledb::Uuid::from_bytes([1; 16])
    })
}
