//! The exhume surface, end to end through the public API
//! (`docs/architecture/70-api.md` § exhume): a store created from a
//! `schema!`-declared theory — closed relations with columns, `fresh`,
//! `str`, `bytes<N>`, general and fixed-width intervals, containments
//! with selections and literal sets, a cardinality window — is read back
//! by [`bumbledb::exhume`] with NO theory in scope: every relation name,
//! field name, closed roster, and committed row arrives from the store's
//! own persisted descriptor. The fixture-surgery lanes (pre-descriptor
//! refusal + adoption, descriptor/fingerprint desync, forced version
//! mismatch) live in the crate's unit tests, where `_meta` is reachable.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::{Db, Interval, StoreKind, Theory as _, Value, exhume};

bumbledb::schema! {
    pub Exhumable;

    closed relation Grade as GradeId {
        points: u64,
    } = {
        Pass { points: 10 },
        Fail { points: 0 },
    };

    relation Learner {
        id: u64 as LearnerId, fresh,
        name: str,
        window: interval<u64>,
    }

    relation Attempt {
        id: u64 as AttemptId, fresh,
        learner: u64 as LearnerId,
        grade: u64 as GradeId,
        digest: bytes<8>,
        lease: interval<i64, 3> as Lease,
    }

    Attempt(learner) <= Learner(id);
    Attempt(grade) <= Grade(id);
    Learner(id) <={0..5} Attempt(learner | grade == Pass);
    Learner(id | name == {"ada", "grace"}) <= Learner(id);
}

/// A self-cleaning per-test store directory (the unit-test `TempDir`'s
/// integration twin).
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-exhume-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_macro_declared_store_exhumes_every_relation_by_name() {
    let dir = TempDir::new("macro-roundtrip");
    {
        let db = Db::create(&dir.0, Exhumable).expect("create");
        db.write(|tx| {
            let learner: LearnerId = tx.alloc()?;
            tx.insert(&Learner {
                id: learner,
                name: "ada",
                window: Interval::<u64>::new(1, 4).expect("interval"),
            })?;
            let attempt: AttemptId = tx.alloc()?;
            tx.insert(&Attempt {
                id: attempt,
                learner,
                grade: Grade::Pass.id(),
                digest: *b"01234567",
                lease: Lease(Interval::<i64>::new(-1, 2).expect("width 3")),
            })?;
            Ok(())
        })
        .expect("write");
    }

    let exhumed = exhume(&dir.0).expect("exhume");
    assert_eq!(exhumed.kind(), StoreKind::Durable);

    // The persisted descriptor carries the theory's identity: its
    // fingerprint is the macro theory's own.
    let schema = Exhumable.descriptor().validate().expect("valid theory");
    assert_eq!(exhumed.fingerprint(), fingerprint(&schema));

    // Names, straight off the store.
    let names: Vec<&str> = exhumed
        .descriptor()
        .relations
        .iter()
        .map(|relation| relation.name.as_ref())
        .collect();
    assert_eq!(names, ["Grade", "Learner", "Attempt"]);
    let attempt_fields: Vec<&str> = exhumed.descriptor().relations[2]
        .fields
        .iter()
        .map(|field| field.name.as_ref())
        .collect();
    assert_eq!(
        attempt_fields,
        ["id", "learner", "grade", "digest", "lease"]
    );

    // The closed roster rides the descriptor — handles AND values.
    let grade = &exhumed.descriptor().relations[0];
    let roster: Vec<(&str, &[Value])> = grade
        .extension
        .as_deref()
        .expect("Grade is closed")
        .iter()
        .map(|row| (row.handle.as_ref(), row.values.as_ref()))
        .collect();
    assert_eq!(
        roster,
        [
            ("Pass", &[Value::U64(10)][..]),
            ("Fail", &[Value::U64(0)][..]),
        ]
    );

    // Every committed row, decoded per the descriptor with no theory in
    // scope: str through `_dict`, bytes inline, the fixed-width interval
    // re-deriving its end; the closed relation scans from its roster.
    let rows = |name: &str| {
        exhumed
            .read(|snap| {
                snap.scan(exhumed.relation(name).expect("relation resolves"))?
                    .collect::<bumbledb::Result<Vec<_>>>()
            })
            .expect("scan")
    };
    assert_eq!(
        rows("Learner"),
        vec![vec![
            Value::U64(0),
            Value::String("ada".as_bytes().into()),
            Value::IntervalU64(Interval::<u64>::new(1, 4).expect("interval")),
        ]]
    );
    assert_eq!(
        rows("Attempt"),
        vec![vec![
            Value::U64(0),
            Value::U64(0),
            Value::U64(Grade::Pass.id().0),
            Value::FixedBytes(Box::from(&b"01234567"[..])),
            Value::IntervalI64(Interval::<i64>::new(-1, 2).expect("width 3")),
        ]]
    );
    assert_eq!(
        rows("Grade"),
        vec![
            vec![Value::U64(0), Value::U64(10)],
            vec![Value::U64(1), Value::U64(0)],
        ]
    );
}
