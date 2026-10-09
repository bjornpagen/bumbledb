//! Typed templates: `query!` derefs to `Query` and carries param and
//! column names; `bind(params! { … })` binds named params in any order,
//! and `field in ?set` params bind `&[Value]` slices.

use bumbledb::{AnswerValue, Answers, BindValue, Db, Interval, ParamArg, Uuid, Value};
use bumbledb::{params, query};

use crate::common::{self, TempDir};

mod learning {
    bumbledb::schema! {
        pub Learning;

        relation Student { id: uuid as StudentId, name: str, budget: u64 }
        relation Attempt {
            id: uuid as AttemptId,
            student: uuid as StudentId,
            score: f64,
            units: u64,
            active: interval<i64>,
        }

        Student(id) -> Student;
        Attempt(id) -> Attempt;
        Attempt(student) <= Student(id);
        Student(id) <=[units]{0..budget} Attempt(student);
    }
}

use learning::{Attempt, AttemptId, Learning, Student, StudentId};

fn id(byte: u8) -> Uuid {
    Uuid::from_bytes([byte; 16])
}

fn f(value: f64) -> bumbledb::F64 {
    bumbledb::F64::from(value)
}

/// Seeds one student with three attempts (scores 0.2 / 0.6 / 0.9, units
/// 1 / 2 / 3) and a second student with one attempt (score 0.8, units 5).
fn seeded(tag: &str) -> (TempDir, Db<Learning>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), Learning, common::work())
        .expect("create the Learning store")
        .expect("accepted");
    db.write(common::work(), |tx| {
        tx.insert([
            &Student {
                id: StudentId(id(1)),
                name: "ada",
                budget: 100,
            },
            &Student {
                id: StudentId(id(2)),
                name: "grace",
                budget: 100,
            },
        ])?;
        let span = bumbledb::Interval::new(0i64, 60i64).expect("nonempty");
        tx.insert([
            &Attempt {
                id: AttemptId(id(11)),
                student: StudentId(id(1)),
                score: f(0.2),
                units: 1,
                active: span,
            },
            &Attempt {
                id: AttemptId(id(12)),
                student: StudentId(id(1)),
                score: f(0.6),
                units: 2,
                active: span,
            },
            &Attempt {
                id: AttemptId(id(13)),
                student: StudentId(id(1)),
                score: f(0.9),
                units: 3,
                active: span,
            },
            &Attempt {
                id: AttemptId(id(14)),
                student: StudentId(id(2)),
                score: f(0.8),
                units: 5,
                active: span,
            },
        ])?;
        Ok(())
    })
    .expect("seed the store")
    .unwrap();
    (dir, db)
}

fn units_of(out: &Answers) -> Vec<u64> {
    let mut units: Vec<u64> = (0..out.len())
        .map(|answer| {
            let AnswerValue::U64(value) = out.get(answer, 0) else {
                panic!("the finds project one u64 column");
            };
            value
        })
        .collect();
    units.sort_unstable();
    units
}

#[test]
fn uuid_join_and_order_compare_the_complete_value_after_reopen() {
    let dir = TempDir::new("uuid-order-join");
    let db = Db::create(dir.path(), Learning, common::work())
        .unwrap()
        .unwrap();
    let mut ids = vec![
        Uuid::nil(),
        Uuid::from_u128(1),
        Uuid::from_u128(1 << 64),
        Uuid::from_u128((1 << 64) + 1),
        Uuid::parse_str("01890abc-1234-7000-8000-000000000001").unwrap(),
        Uuid::from_u128(u128::MAX),
    ];
    ids.sort_unstable();
    db.write(common::work(), |tx| {
        for id in &ids {
            tx.insert([&Student {
                id: StudentId(*id),
                name: "uuid",
                budget: 10,
            }])?;
        }
        for (index, id) in ids.iter().enumerate().filter(|(index, _)| index % 2 == 1) {
            tx.insert([&Attempt {
                id: AttemptId(Uuid::from_u128(index as u128)),
                student: StudentId(*id),
                score: f(1.0),
                units: 1,
                active: Interval::new(0i64, 1).unwrap(),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);
    let db = Db::open(dir.path(), Learning, common::work()).unwrap();
    let joined = query!(Learning {
        (student) | Student(id: student), Attempt(student), student > ?floor;
    });
    let mut prepared = db.prepare(&joined, common::work()).unwrap();
    for floor in &ids {
        let answers = db
            .read(common::work(), |frame| {
                frame.execute_collect(&mut prepared, &[BindValue::Uuid(*floor)])
            })
            .unwrap();
        let mut actual = (0..answers.len())
            .map(|row| match answers.get(row, 0) {
                AnswerValue::Uuid(id) => id,
                other => panic!("UUID result decoded as {other:?}"),
            })
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let expected = ids
            .iter()
            .enumerate()
            .filter(|(index, id)| index % 2 == 1 && *id > floor)
            .map(|(_, id)| *id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
    // Cross-occurrence residuals use the wide Free Join comparison, rather
    // than the single-relation literal filter exercised above.
    let pairs = query!(Learning {
        (left, right) | Student(id: left), Student(id: right), left < right;
    });
    let mut pairs = db.prepare(&pairs, common::work()).unwrap();
    let answers = db
        .read(common::work(), |frame| {
            frame.execute_collect(&mut pairs, &[] as &[BindValue])
        })
        .unwrap();
    let mut actual = (0..answers.len())
        .map(|row| {
            let (AnswerValue::Uuid(left), AnswerValue::Uuid(right)) =
                (answers.get(row, 0), answers.get(row, 1))
            else {
                panic!("UUID pair")
            };
            (left, right)
        })
        .collect::<Vec<_>>();
    actual.sort_unstable();
    let expected = ids
        .iter()
        .flat_map(|left| {
            ids.iter()
                .filter(move |right| left < *right)
                .map(move |right| (*left, *right))
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

/// Named binding, in any order, answers exactly as positional binding.
#[test]
fn named_binds_match_positional_binds_order_free() {
    let attempts_for = query!(Learning {
        (units) | Attempt(id, student == ?student, score, units), score > ?floor;
    });
    assert_eq!(attempts_for.param_names(), ["student", "floor"]);
    assert_eq!(attempts_for.columns(), ["units"]);

    let (_dir, db) = seeded("typed-named-binds");
    let mut prepared = db
        .prepare(&attempts_for, common::work())
        .expect("the template validates");
    db.read(common::work(), |snap| {
        // Positional: ParamId order is first use (student, then floor).
        let positional = snap.execute_collect(
            &mut prepared,
            &[BindValue::Uuid(id(1)), BindValue::F64(f(0.5))],
        )?;
        // Named, in reverse order.
        let named = attempts_for.bind(params! { floor: 0.5f64, student: id(1) });
        let bound = snap.execute_collect(&mut prepared, &named)?;
        assert_eq!(units_of(&positional), vec![2, 3], "the seeded rows filter");
        assert_eq!(
            units_of(&bound),
            units_of(&positional),
            "order-free named binding agrees"
        );
        Ok(())
    })
    .expect("both bind spellings execute");
}

/// A `field in ?set` param binds a `&[Value]` slice; rebinding the same
/// template with a different set is an ordinary re-execution.
#[test]
fn set_params_bind_value_slices() {
    let sized = query!(Learning {
        (score) | Attempt(id, score, units in ?sizes);
    });
    assert_eq!(sized.param_names(), ["sizes"]);

    let scores_of = |out: &Answers| -> Vec<u64> {
        let mut bits: Vec<u64> = (0..out.len())
            .map(|answer| {
                let AnswerValue::F64(value) = out.get(answer, 0) else {
                    panic!("the finds project one f64 column");
                };
                value.to_bits()
            })
            .collect();
        bits.sort_unstable();
        bits
    };
    let expected = |values: &[f64]| -> Vec<u64> {
        let mut bits: Vec<u64> = values.iter().map(|v| f(*v).to_bits()).collect();
        bits.sort_unstable();
        bits
    };

    let (_dir, db) = seeded("typed-set-binds");
    let mut prepared = db
        .prepare(&sized, common::work())
        .expect("the template validates");
    db.read(common::work(), |snap| {
        let small = [Value::U64(1), Value::U64(2)];
        let out = snap.execute_collect(&mut prepared, &sized.bind(params! { sizes: &small }))?;
        assert_eq!(scores_of(&out), expected(&[0.2, 0.6]));
        let large = [Value::U64(5)];
        let out = snap.execute_collect(&mut prepared, &sized.bind(params! { sizes: &large }))?;
        assert_eq!(scores_of(&out), expected(&[0.8]));
        Ok(())
    })
    .expect("set binds execute");
}

/// A zero-param template still binds (`params! {}` is the empty builder),
/// and the untyped positional path stays available beside it.
#[test]
fn zero_param_templates_bind_empty() {
    let all = query!(Learning {
        (units) | Attempt(id, units);
    });
    assert!(
        all.param_names().is_empty(),
        "a paramless template has no names"
    );
    let (_dir, db) = seeded("typed-zero-params");
    let mut prepared = db
        .prepare(&all, common::work())
        .expect("the template validates");
    db.read(common::work(), |snap| {
        let named = all.bind(params! {});
        assert!(named.is_empty(), "no params, no args");
        let out = snap.execute_collect(&mut prepared, &named)?;
        assert_eq!(units_of(&out), vec![1, 2, 3, 5]);
        let positional: &[ParamArg<'_>] = &[];
        let out = snap.execute_collect(&mut prepared, positional)?;
        assert_eq!(units_of(&out), vec![1, 2, 3, 5]);
        Ok(())
    })
    .expect("the paramless template executes");
}

/// Column names: a projected variable's name, an aggregate's `name:` label,
/// or an unlabeled aggregate's spelling.
#[test]
fn columns_carry_head_names_and_labels() {
    let stats = query!(Learning {
        (student, total: Sum(units), Count) | Attempt(id, student, units);
    });
    assert_eq!(stats.columns(), ["student", "total", "Count"]);
    let unlabeled = query!(Learning {
        (student, Sum(units)) | Attempt(id, student, units);
    });
    assert_eq!(unlabeled.columns(), ["student", "Sum(units)"]);
}

/// `into_query()` moves out the same IR a second expansion builds.
#[test]
fn into_query_is_the_plain_ir() {
    let a = query!(Learning {
        (units) | Attempt(id, units);
    });
    let b = query!(Learning {
        (units) | Attempt(id, units);
    });
    let ir = a.into_query();
    assert_eq!(&ir, b.query(), "one notation, one IR");
    // Deref half: the borrowed view is the same IR.
    assert_eq!(ir.rules().len(), b.rules().len());
}

/// A value of the wrong kind for its slot fails at execution as
/// `ParamTypeMismatch`.
#[test]
fn wrong_value_kinds_stay_typed_bind_errors() {
    let attempts_for = query!(Learning {
        (units) | Attempt(id, student == ?student, units);
    });
    let (_dir, db) = seeded("typed-bind-mismatch");
    let mut prepared = db
        .prepare(&attempts_for, common::work())
        .expect("the template validates");
    db.read(common::work(), |snap| {
        let bound = attempts_for.bind(params! { student: 7u64 });
        let error = snap
            .execute_collect(&mut prepared, &bound)
            .expect_err("a u64 in an uuid slot refuses");
        assert!(
            matches!(error, bumbledb::Error::ParamTypeMismatch { .. }),
            "the C05 typed bind error surfaces: {error:?}"
        );
        Ok(())
    })
    .expect("the refusal is typed, not a panic");
}
