//! The `lawful` home-turf world — the integrity turf nobody benches:
//! what does ADMISSION under a full law roster cost, against `SQLite`
//! carrying equivalent SQL constraints? The schema is primer-shaped
//! (the real workload at
//! `primer/src/tools/graph-builder/store/schema.ts`:
//! task/attempt/verdict/steer/steerScope with identity keys,
//! containments, the ψ-selected `steerScopeSteerRef`, and a closed
//! vocabulary with payload — the `RequiresState.terminal` shape), so
//! the lanes measure laws a real application judges, never synthetic
//! constraints. Commit throughput and rejection latency ride on top of
//! this foundation ([`lanes`] holds the six family runners, [`run`] the
//! orchestration, [`render`] the artifacts); this module is the judged
//! schema, the sizes, the family registry ([`families`]), and (in the
//! submodules) the seeded corpus, the enforcement map, and the
//! durability-paired twin loader.
//!
//! REPORT-class, never gated: lawful rows land in the artifact and
//! nothing else — no budget gate ever reads them (the standing
//! report-class law).
//!
//! The twin carries EQUIVALENT enforcement as data
//! ([`enforcement::MAP`], one row per materialized engine statement,
//! totality tested): UNIQUE for the declared keys, `REFERENCES` FKs for
//! the relation-to-relation containments, CHECK for the closed
//! vocabularies (their rosters are static schema data — no mirror
//! tables exist, and no reads exist in this world), and a trigger where
//! SQL needs one (the ψ-selected containment and the attempt window).
//! One honesty note, recorded once: with `PRAGMA foreign_keys=ON`,
//! `SQLite` checks FKs per statement (immediate), while the engine
//! judges FINAL states — for the single-insert and insert-ordered
//! cluster shapes this lane exercises, the two disciplines render the
//! same verdicts, and that agreement is exactly what the naive-parity
//! test pins ([`crate::differential::run`] against
//! [`crate::naive::NaiveDb`], verdicts and citations compared whole).

use crate::corpus_gen::Scale;
use crate::harness::Protocol;

pub mod corpus;
pub mod enforcement;
pub mod lanes;
pub mod load;
pub mod render;
pub mod run;
#[cfg(test)]
mod tests;

pub use lanes::{AttemptOp, LawCursor};
pub use run::{LawRow, run, run_with};

bumbledb::schema! {
    pub LawfulWorld;

    relation Task {
        id: u64 as LawTaskId, fresh,
        kind: u64 as LawTaskKindId,
        subject: u64,
    }
    relation Attempt {
        id: u64 as LawAttemptId, fresh,
        task: u64 as LawTaskId,
        n: u64,
    }
    relation Verdict {
        attempt: u64 as LawAttemptId,
        outcome: u64 as LawOutcomeId,
    }
    relation Steer {
        id: u64 as LawSteerId, fresh,
        kind: u64 as LawSteerKindId,
        task: u64 as LawTaskId,
    }
    relation SteerScope {
        steer: u64 as LawSteerId,
        grp: u64,
    }

    closed relation TaskKinds as LawTaskKindId = { Enrich, Author, Judge };
    closed relation SteerKinds as LawSteerKindId = { Observe, Repartition };
    closed relation Outcome as LawOutcomeId {
        terminal: bool,
    } = {
        Proposed { terminal: false },
        Accepted { terminal: true },
        Rejected { terminal: true },
    };

    Task(kind, subject) -> Task;
    Attempt(task, n) -> Attempt;
    Verdict(attempt) -> Verdict;
    SteerScope(steer, grp) -> SteerScope;

    Task(kind) <= TaskKinds(id);
    Attempt(task) <= Task(id);
    Verdict(attempt) <= Attempt(id);
    Verdict(outcome) <= Outcome(id);
    Steer(kind) <= SteerKinds(id);
    Steer(task) <= Task(id);
    SteerScope(steer) <= Steer(id | kind == Repartition);

    Task(id) <={0..8} Attempt(task);
}

/// Relation ids by declaration order (ordinary relations first, then
/// the closed vocabularies), pinned by the schema test. The colliding
/// vocabulary names carry the macro's diagnosed rename: `TaskKinds` /
/// `SteerKinds` (plural), because the id-constant space would
/// otherwise name both field `Task.kind` and a `TaskKind` relation
/// `TASK_KIND` (likewise `Steer.kind` / `SteerKind`).
pub mod ids {
    use bumbledb::RelationId;

    pub const TASK: RelationId = RelationId(0);
    pub const ATTEMPT: RelationId = RelationId(1);
    pub const VERDICT: RelationId = RelationId(2);
    pub const STEER: RelationId = RelationId(3);
    pub const STEER_SCOPE: RelationId = RelationId(4);
    pub const TASK_KINDS: RelationId = RelationId(5);
    pub const STEER_KINDS: RelationId = RelationId(6);
    pub const OUTCOME: RelationId = RelationId(7);
}

/// The validated lawful schema, memoized for the mirror's DDL assembly
/// and the comparator's field walks; the store is created from
/// [`LawfulWorld`]'s descriptor ([`load::load_stores`]).
///
/// # Panics
///
/// Never in practice: the declared lawful schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    use bumbledb::schema::ValidateDescriptor as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        LawfulWorld
            .descriptor()
            .validate()
            .expect("the lawful schema is valid")
    })
}

/// The lawful corpus shape — the seeded mass every lane starts from
/// ([`corpus::relation_rows`] derives every row from these counts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawSizes {
    /// The standing `Task` mass.
    pub tasks: u64,
    /// Seeded `Attempt` rows per task (far under the window's cap of 8,
    /// so every seeded commit is legal).
    pub attempts_per_task: u64,
    /// The standing `Steer` mass (alternating Observe/Repartition; each
    /// Repartition steer carries one `SteerScope` row).
    pub steers: u64,
}

impl LawSizes {
    /// Two size points, the crud-world precedent: `Tiny` for tests and
    /// the parity slice, one judged-write shape for every timed scale.
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        match scale {
            Scale::Tiny => Self {
                tasks: 128,
                attempts_per_task: 2,
                steers: 16,
            },
            Scale::S | Scale::M | Scale::L => Self {
                tasks: 4_096,
                attempts_per_task: 2,
                steers: 512,
            },
        }
    }
}

/// One registered lawful family: the name reports print, the honest
/// one-line description, and the registered protocol. The protocol is
/// DATA handed to the runners ([`lanes`]) at orchestration time
/// ([`run`]), never baked into a runner — tests run the same runners
/// under tiny protocols (the crud registry precedent).
#[derive(Debug, Clone, Copy)]
pub struct LawFamily {
    pub name: &'static str,
    pub about: &'static str,
    pub protocol: Protocol,
}

/// The six lawful families in THE run order — legal commits before
/// rejections, and the registry order IS the run order (the
/// orchestration iterates this slice, never reorders). The ordering is
/// load-bearing twice over: the legal lanes' shared fresh cursors must
/// see the store the window setup left (task 0 saturated, both engines'
/// counters in lockstep), and the rejection lanes burn the engine's
/// escaped fresh high-water mark past [`lanes::REJECT_ID_BASE`] (the
/// never-reissue law — an aborted explicit insert burns like a
/// committed one), so no legal commit may ever mint after them.
#[must_use]
pub fn families() -> &'static [LawFamily] {
    &[
        LawFamily {
            name: "law_commit_attempt",
            about: "one judged Attempt insert per commit under the full law roster \
                    (key + containment + window)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        LawFamily {
            name: "law_commit_cluster",
            about: "one judged 4-row cluster per commit: attempt + verdict + steer + scope \
                    — every statement family exercised in one commit",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        LawFamily {
            name: "law_reject_key",
            about: "one REFUSED duplicate-(task, n) commit per sample (Functionality cited)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        LawFamily {
            name: "law_reject_containment",
            about: "one REFUSED absent-task commit per sample (Containment cited)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        LawFamily {
            name: "law_reject_window",
            about: "one REFUSED 9th-attempt commit on the saturated task 0 per sample \
                    (Cardinality cited)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        LawFamily {
            name: "law_reject_scope",
            about: "one REFUSED Observe-steer scope commit per sample (the ψ containment cited)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
    ]
}
