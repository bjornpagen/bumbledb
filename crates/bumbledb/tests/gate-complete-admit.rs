//! Population: complete judgment at admission, no-clobber publication, and
//! incremental judgment of ordinary writes afterwards.

use bumbledb::host::{DatabaseId, HostChanges, Population};
use bumbledb::{Admission, ChangeSet, Db, Error, Options, RelationId, Value, WorkContext};

mod common;

bumbledb::schema! {
    pub GateAdmit;

    relation User {
        id: u64 as UserId,
        email: str,
    }

    User(email) -> User;
}

const USER: RelationId = RelationId(0);

fn user(schema: &bumbledb::schema::Schema, id: u64, email: &str) -> ChangeSet {
    let mut builder = ChangeSet::builder(schema, common::work());
    builder
        .insert(USER, &[Value::U64(id), Value::String(email.into())])
        .expect("insert");
    builder.finish().expect("seal")
}

fn begin(dest: &std::path::Path) -> Population<GateAdmit> {
    Population::begin(
        dest,
        GateAdmit,
        DatabaseId::mint(),
        Options::default(),
        common::work(),
    )
    .expect("begin")
}

fn schema() -> bumbledb::schema::Schema {
    use bumbledb::Theory as _;
    use bumbledb::schema::ValidateDescriptor as _;
    GateAdmit.descriptor().validate().expect("schema")
}

/// Unjudged applies may stage a key conflict; admission judges the whole
/// state, rejects it, and publishes nothing.
#[test]
fn admission_rejects_a_conflicting_population() {
    let dir = common::TempDir::new("gate-admit-conflict");
    let dest = dir.path().join("store.bdb");
    let schema = schema();
    let mut population = begin(&dest);
    population
        .apply(&user(&schema, 1, "dup@ex"))
        .expect("apply");
    population
        .apply(&user(&schema, 2, "dup@ex"))
        .expect("apply");
    match population.admit(HostChanges::NONE).expect("admit") {
        Admission::Rejected(violations) => assert_eq!(violations.len(), 1),
        Admission::Accepted(_) => panic!("a key conflict was admitted"),
    }
    assert!(!dest.exists(), "nothing is published");
}

/// Two populations of one destination: the first publishes, the second is
/// refused and leaves the winner intact.
#[test]
fn a_second_population_never_overwrites() {
    let dir = common::TempDir::new("gate-admit-two");
    let dest = dir.path().join("store.bdb");
    let schema = schema();
    let mut first = begin(&dest);
    let mut second = begin(&dest);
    first.apply(&user(&schema, 1, "a@ex")).expect("apply");
    second.apply(&user(&schema, 2, "b@ex")).expect("apply");
    drop(
        first
            .admit(HostChanges::NONE)
            .expect("admit")
            .expect("accepted"),
    );
    let Err(refused) = second.admit(HostChanges::NONE) else {
        panic!("a second population published over the first");
    };
    assert!(
        matches!(refused, Error::DestinationExists { .. }),
        "{refused:?}"
    );
    let db = Db::open(&dest, GateAdmit, common::work()).expect("reopen winner");
    let pin = db.owned_read().expect("pin");
    let work = common::work();
    let frame = pin.frame(&work);
    assert!(
        frame
            .contains_dyn(USER, &[Value::U64(1), Value::String("a@ex".into())])
            .expect("read")
    );
    assert_eq!(pin.count(USER).expect("count"), 1);
}

/// After admission, ordinary writes are judged: a distinct email commits,
/// a duplicate is rejected, a pinned read sees exactly the committed rows,
/// and an empty write from the pin's witness commits unchanged.
#[test]
fn writes_after_admission_are_judged() {
    let dir = common::TempDir::new("gate-admit-after");
    let dest = dir.path().join("store.bdb");
    let schema = schema();
    let mut population = begin(&dest);
    population.apply(&user(&schema, 1, "a@ex")).expect("apply");
    let db = population
        .admit(HostChanges::NONE)
        .expect("admit")
        .expect("accepted");
    let work = WorkContext::new();
    let committed = db
        .apply(&user(db.schema(), 2, "b@ex"), &work)
        .expect("apply")
        .expect("a distinct email commits");
    assert!(committed.changed);
    let conflict = db
        .apply(&user(db.schema(), 3, "a@ex"), &work)
        .expect("apply");
    assert!(
        matches!(conflict, bumbledb::WriteOutcome::Rejected(ref violations) if violations.len() == 1),
        "{conflict:?}"
    );

    let pin = db.owned_read().expect("pin");
    assert_eq!(pin.count(USER).expect("count"), 2);
    let frame = pin.frame(&work);
    for (id, email, present) in [(1, "a@ex", true), (2, "b@ex", true), (3, "a@ex", false)] {
        let row = [Value::U64(id), Value::String(email.into())];
        assert_eq!(frame.contains_dyn(USER, &row).expect("read"), present);
    }
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .expect("empty");
    let unchanged = db
        .apply_from(&empty, &pin.witness(), &work)
        .expect("apply")
        .expect("the witness is current");
    assert!(!unchanged.changed);
}
