use crate::families::{Kind, WriteFamily};

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

        // `crate::windowed`): commit_single's protocol against the twin

        WriteFamily {
            name: "commit_window_baseline",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_window_admission",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_window_exclusion",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },

        WriteFamily {
            name: "commit_capacity_baseline",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_capacity_sum",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_capacity_duration",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "insert_stream",
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

        WriteFamily {
            name: "cold_containment_walk_delete",
            kind: Kind::Report,
            protocol: Protocol::COLD,
        },
    ]
}
