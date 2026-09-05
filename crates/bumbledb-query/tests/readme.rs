//! Rot-proofing for the repo-root `README.md` (the front page): its two
//! `rust` fences compile and RUN against the current engine, token-pinned
//! against the compiled duplicates below — the cookbook.rs
//! duplicate-and-pin law (markdown cannot be `include!`d at item position,
//! so each fence is duplicated from the SAME tokens its pin stringifies,
//! and the sync test holds the duplication token-for-token,
//! comment-stripped: the token stream never carries comments). The
//! front-page cookbook count claim is pinned here too, against the
//! cookbook's numbered recipe headings.

use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

const README: &str = include_str!("../../../README.md");
const COOKBOOK: &str = include_str!("../../../docs/cookbook.md");

mod common;
use common::TempDir;

/// The documented quickstart is compiled from the same tokens that its drift
/// check compares. Its test drives the public create/apply/read/close lifecycle.
macro_rules! quickstart_fence {
    ($($items:tt)*) => {
        mod quickstart {
            $($items)*
            pub const SOURCE: &str = stringify!($($items)*);
            pub fn run(path: &std::path::Path) -> Result<usize, Box<dyn std::error::Error>> {
                let work = crate::common::work();
                let db = open_ledger(path, work.clone())?;
                assert!(matches!(seed(&db, &work)?, ApplyOutcome::Accepted { .. }));
                let q = bumbledb_query::query!(Ledger {
                    (h, name) | Holder(id: h, name), Account(holder: h, status == Status::Open);
                });
                let mut prepared = db.prepare(&q, work.clone())?;
                let mut results = bumbledb::Answers::default();
                let params: [bumbledb::BindValue<'static>; 0] = [];
                db.read(work.clone(), |snap| {
                    snap.execute(&mut prepared, &params, &mut results)?;
                    Ok(())
                })?;
                assert!(matches!(pin_and_close(&db, &work)?, CloseReport::Closed));
                Ok(results.len())
            }
        }
    };
}

quickstart_fence! {
use bumbledb::{ApplyExpected, ApplyOutcome, ChangeSet, ChangeSetBuilder, CloseReport, Db, Fact, WorkContext};

bumbledb::schema! {
    pub Ledger;

    closed relation Region as RegionId = { Na, Eu, Apac, Latam };
    closed relation Status as StatusId = { Open, Frozen, Closed };

    relation Holder {
        id: u64 as HolderId,
        name: str,
        region: u64 as RegionId,
    }
    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        status: u64 as StatusId,
        opened_at: i64,
    }

    Holder(id)   -> Holder;
    Account(id)  -> Account;
    Account(holder) <= Holder(id);
    Holder(region)  <= Region(id);
    Account(status) <= Status(id);
}

fn open_ledger(path: &std::path::Path, work: WorkContext) -> bumbledb::Result<Db<Ledger>> {
    Ok(Db::create(path, Ledger, work)?.expect("empty Ledger admits"))
}

fn insert_fact<'a, F: Fact<'a>>(
    draft: &mut ChangeSetBuilder<'_>,
    fact: &F,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut values = Vec::new();
    fact.append_values(&mut values)?;
    draft.insert(F::RELATION, &values)?;
    Ok(())
}

fn seed(db: &Db<Ledger>, work: &WorkContext) -> Result<ApplyOutcome, Box<dyn std::error::Error>> {
    let holder = HolderId(1);
    let account = AccountId(42);
    let mut draft = ChangeSet::builder(db.schema(), work.clone());
    insert_fact(&mut draft, &Holder { id: holder, name: "alice", region: Region::Eu.id() })?;
    insert_fact(&mut draft, &Account {
        id: account,
        holder,
        status: Status::Open.id(),
        opened_at: 17_000_000,
    })?;
    Ok(db.apply(&draft.finish()?, ApplyExpected::Any, work)?)
}

fn pin_and_close(db: &Db<Ledger>, work: &WorkContext) -> Result<CloseReport, Box<dyn std::error::Error>> {
    let snapshot = db.snapshot(work)?;
    drop(snapshot);
    Ok(db.close(work))
}
}

/// The closed-relation payload fence (`schema!`-interior syntax), spliced
/// into a compiled schema beside the two open relations its statements
/// quantify over.
macro_rules! payload_fence {
    ($($t:tt)*) => {
        mod payload {
            bumbledb::schema! {
                pub Payload;

                relation Attempt     { id: u64 as AttemptId, kind: u64 as KindId }
                relation Certificate { id: u64 as CertificateId, kind: u64 as KindId }

                $($t)*
            }
            pub const SOURCE: &str = stringify!($($t)*);
            pub fn validate() -> Result<bumbledb::Schema, bumbledb::error::SchemaError> {
                use bumbledb::Theory as _;
                use bumbledb::schema::ValidateDescriptor as _;
                Payload.descriptor().validate()
            }
        }
    };
}

payload_fence!(
    closed relation Status as StatusId = { Open, Frozen, Closed };

    closed relation Kind as KindId {
        mastered: bool,
        rank: u64,
    } = {
        DirectPass { mastered: true,  rank: 30 },
        JudgedPass { mastered: true,  rank: 20 },
        Failed     { mastered: false, rank: 10 },
    };

    Attempt(kind) <= Kind(id);
    Certificate(kind) <= Kind(id | mastered == true);
);

/// Comments and whitespace out; what remains is exactly what the token
/// stream carries (the cookbook.rs `normalize`), so a stringified
/// duplicate compares against a doc fence.
fn normalize(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// The README's fenced `rust` blocks, in order.
fn readme_fences() -> Vec<String> {
    let mut fences = Vec::new();
    let mut fence: Option<String> = None;
    for line in README.lines() {
        match &mut fence {
            None if line.trim() == "```rust" => fence = Some(String::new()),
            None => {}
            Some(block) if line.trim() == "```" => {
                fences.push(std::mem::take(block));
                fence = None;
            }
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
        }
    }
    fences
}

/// The front page carries exactly two rust fences — the quickstart and the
/// closed-relation payload example — and each is token-identical to its
/// compiled duplicate above. A new fence must land with a pin here.
#[test]
fn the_front_page_fences_match_the_compiled_copies() {
    let fences = readme_fences();
    assert_eq!(fences.len(), 2, "the README's rust fence census");
    let expected_quickstart = normalize(quickstart::SOURCE);
    assert_eq!(
        normalize(&fences[0]),
        expected_quickstart,
        "the quickstart fence drifted between README.md and its compiled duplicate"
    );
    assert_eq!(
        normalize(&fences[1]),
        normalize(payload::SOURCE),
        "the payload fence drifted between README.md and its compiled duplicate"
    );
}

/// The quickstart is not just compiled — it runs whole against a real
/// store and finds its one open account.
#[test]
fn the_quickstart_runs_against_a_real_store() {
    let dir = TempDir::new("readme-quickstart");
    let answers = quickstart::run(dir.path()).expect("the front-page quickstart runs");
    assert_eq!(answers, 1, "alice's one open account");
}

/// Both fence schemas validate against the current engine.
#[test]
fn the_front_page_schemas_validate() {
    quickstart::Ledger
        .descriptor()
        .validate()
        .expect("the quickstart schema validates");
    payload::validate().expect("the payload schema validates");
}

/// The front page's cookbook claim: the spelled-out count matches the
/// cookbook's numbered recipes (the README is otherwise the one estate
/// doc without a count pin). A recipe added to the cookbook moves the
/// README sentence and this pin in the same change.
#[test]
fn the_front_page_cookbook_count_is_the_cookbook() {
    let recipes = COOKBOOK
        .lines()
        .filter(|line| {
            line.strip_prefix("## ")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
        .count();
    assert_eq!(recipes, 32, "the cookbook's numbered recipe census");
    assert!(
        README.contains("thirty-two worked schemas"),
        "the README's spelled-out cookbook count must match the {recipes}-recipe cookbook"
    );
}
