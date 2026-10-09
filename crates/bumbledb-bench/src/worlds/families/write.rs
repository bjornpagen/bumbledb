use crate::worlds::families::WriteFamily;

#[must_use]
pub fn write_families() -> &'static [WriteFamily] {
    use crate::harness::Protocol;
    &[
        WriteFamily {
            name: "commit_single",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_witnessed",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_batch",
            protocol: Protocol {
                warmups: 4,
                samples: 32,
            },
        },
        // The windowed and capacity families (`crate::worlds::windowed`,
        // `crate::worlds::capacity`) run commit_single's protocol.
        WriteFamily {
            name: "commit_window_baseline",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_window_admission",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_window_exclusion",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_capacity_baseline",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_capacity_sum",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_capacity_duration",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "insert_stream",
            protocol: Protocol {
                warmups: 1,
                samples: 8,
            },
        },
        WriteFamily {
            name: "cold_containment_walk",
            protocol: Protocol::COLD,
        },
        WriteFamily {
            name: "cold_containment_walk_delete",
            protocol: Protocol::COLD,
        },
    ]
}
