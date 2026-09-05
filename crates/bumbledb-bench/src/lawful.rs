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
        id: u64 as LawTaskId,
        kind: u64 as LawTaskKindId,
        subject: u64,
    }
    relation Attempt {
        id: u64 as LawAttemptId,
        task: u64 as LawTaskId,
        n: u64,
    }
    relation Verdict {
        attempt: u64 as LawAttemptId,
        outcome: u64 as LawOutcomeId,
    }
    relation Steer {
        id: u64 as LawSteerId,
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

    Task(id) -> Task;
    Attempt(id) -> Attempt;
    Steer(id) -> Steer;

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

/// # Panics
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawSizes {
    pub tasks: u64,

    pub attempts_per_task: u64,

    pub steers: u64,
}

impl LawSizes {
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

/// one-line description, and the registered protocol. The protocol is
#[derive(Debug, Clone, Copy)]
pub struct LawFamily {
    pub name: &'static str,
    pub about: &'static str,
    pub protocol: Protocol,
}

/// The ordering is load-bearing: the legal lanes' shared APPLICATION-OWNED id
/// cursors must see the store the window setup left (task 0 saturated, both
/// engines' counters in lockstep). Ids are ordinary supplied values — the
/// successor has no fresh generator, reservation or burn semantics — and the
/// rejection lanes deliberately choose ids at [`lanes::REJECT_ID_BASE`], far
/// above every legal cursor, so a rejected attempt can never collide with a
/// later legal insert.
#[must_use]
pub fn families() -> &'static [LawFamily] {
    &[
        LawFamily {
            name: "law_commit_attempt",
            about: "one judged Attempt insert per commit under the full law roster \
                    (key + containment + capacity)",
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
                    (Capacity cited)",
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
