use crate::families::{Kind, WriteFamily};

/// The write and cold families — all `Kind::Report` (the suite ruling:
/// "every family must win" is the read set; writes and cold are
/// described honestly, never gated).
#[must_use]
pub fn write_families() -> &'static [WriteFamily] {
    use crate::harness::Protocol;
    &[
        WriteFamily {
            name: "commit_single",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        // The witnessed-write row (the PRD-18 spine debt): commit_single
        // through `Db::write_from` with a fresh snapshot witness per
        // sample — the delta against commit_single prices the witness
        // (one generation read + one integer compare). SQLite-unpaired
        // by decision: SQLite has no snapshot-witness surface, and a
        // BEGIN-IMMEDIATE + user-version emulation would time the
        // emulation, not the engine.
        WriteFamily {
            name: "commit_witnessed",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_batch",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 4,
                samples: 32,
            },
        },
        WriteFamily {
            name: "bulk",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 1,
                samples: 8,
            },
        },
        WriteFamily {
            name: "cold_containment_walk",
            kind: Kind::Report,
            protocol: Protocol::COLD,
        },
    ]
}
