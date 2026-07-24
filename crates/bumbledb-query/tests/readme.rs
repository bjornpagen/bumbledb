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

/// The quickstart fence, compiled from one token stream: the `schema!`
/// interior at item position, the body in a runnable fn with the fence's
/// three free names bound the way the prose around the fence describes
/// them (a store path, no params, a reusable answer buffer). The names
/// arrive as macro inputs so they share the fence tokens' hygiene.
macro_rules! quickstart_fence {
    (free { $path:ident, $params:ident, $results:ident }
     schema { $($s:tt)* } body { $($b:tt)* }) => {
        mod quickstart {
            bumbledb::schema! { $($s)* }
            pub const SCHEMA_SOURCE: &str = stringify!($($s)*);
            pub const BODY_SOURCE: &str = stringify!($($b)*);
            pub fn run($path: &std::path::Path) -> Result<usize, Box<dyn std::error::Error>> {
                let $params: [bumbledb::BindValue<'static>; 0] = [];
                let mut $results = bumbledb::Answers::default();
                $($b)*
                Ok($results.len())
            }
        }
    };
}

quickstart_fence!(
    free { path, params, results }
    schema {
        pub Ledger;

        closed relation Region as RegionId = { Na, Eu, Apac, Latam };
        closed relation Status as StatusId = { Open, Frozen, Closed };

        relation Holder {
            id: u64 as HolderId, fresh,
            name: str,
            region: u64 as RegionId,
        }
        relation Account {
            id: u64 as AccountId, fresh,
            holder: u64 as HolderId,
            status: u64 as StatusId,
            opened_at: i64,
        }

        Account(holder) <= Holder(id);
        Holder(region)  <= Region(id);
        Account(status) <= Status(id);
    }
    body {
        let db = bumbledb::Db::create(path, Ledger)?;

        db.write(|tx| {
            let holder: HolderId = tx.alloc()?;
            tx.insert(&Holder { id: holder, name: "alice", region: Region::Eu.id() })?;
            let account: AccountId = tx.alloc()?;
            tx.insert(&Account { id: account, holder, status: Status::Open.id(), opened_at: 17_000_000 })?;
            Ok(())
        })?;

        let q = bumbledb_query::query!(Ledger {
            (h, name) | Holder(id: h, name), Account(holder: h, status == Status::Open);
        });
        let mut prepared = db.prepare(&q)?;
        db.read(|snap| {
            snap.execute(&mut prepared, &params, &mut results)?;
            Ok(())
        })?;
    }
);

/// The closed-relation payload fence (`schema!`-interior syntax), spliced
/// into a compiled schema beside the two open relations its statements
/// quantify over.
macro_rules! payload_fence {
    ($($t:tt)*) => {
        mod payload {
            bumbledb::schema! {
                pub Payload;

                relation Attempt     { id: u64 as AttemptId, fresh, kind: u64 as KindId }
                relation Certificate { id: u64 as CertificateId, fresh, kind: u64 as KindId }

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
    let expected_quickstart = format!(
        "bumbledb::schema!{{{}}}{}",
        normalize(quickstart::SCHEMA_SOURCE),
        normalize(quickstart::BODY_SOURCE)
    );
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
    assert_eq!(recipes, 30, "the cookbook's numbered recipe census");
    assert!(
        README.contains("thirty worked schemas"),
        "the README's spelled-out cookbook count must match the {recipes}-recipe cookbook"
    );
}
