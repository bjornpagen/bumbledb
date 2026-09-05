//! High-impact constants: representation bound, host policy, or measured
//! crossover. Source locations rechecked against the dirty tree (2026-09-05).
//! L20 does not change production constants; hotspot corrections go to the
//! owning lane.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantClass {
    /// Arithmetic of a selected representation. Structural test, not a sweep.
    RepresentationBound,
    /// Deliberate host/resource envelope. Typed limit, not a data-size law.
    HostPolicy,
    /// Workload/hardware choice. Qualify per named target; keep a fallback.
    MeasuredCrossover,
    /// Measurement machinery only. Must not certify a foreign machine.
    Instrumentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constant {
    pub id: &'static str,
    pub source: &'static str,
    pub value: &'static str,
    pub class: ConstantClass,
    pub owner_lane: &'static str,
    pub disposition: &'static str,
}

pub const CONSTANTS: &[Constant] = &[
    Constant {
        id: "row-key",
        source: "crates/bumbledb/src/storage/store/keys.rs ROW_KEY_LEN",
        value: "13",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L07",
        disposition: "tag+relation+row; structural equality with space::current_layout",
    },
    Constant {
        id: "membership-key",
        source: "crates/bumbledb/src/storage/store/keys.rs MEMBERSHIP_KEY_LEN",
        value: "29",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L07",
        disposition: "16-byte exact-checked BLAKE3 fingerprint; row-ID+membership kept",
    },
    Constant {
        id: "determinant-overhead",
        source: "crates/bumbledb/src/schema/compiled.rs DETERMINANT_KEY_OVERHEAD",
        value: "11",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L01",
        disposition: "tag+ProjectionId+row; not a declaration-order statement number",
    },
    Constant {
        id: "exact-scalar-width",
        source: "crates/bumbledb/src/schema/compiled.rs MAX_EXACT_SCALAR_BYTES",
        value: "16",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L01",
        disposition: "whole-key backend fit; fingerprint otherwise. No autotuner",
    },
    Constant {
        id: "host-key-max",
        source: "crates/bumbledb/src/storage/store/keys.rs HOST_KEY_MAX",
        value: "510",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L07",
        disposition: "LMDB key limit minus tag; long logical keys never enter keys",
    },
    Constant {
        id: "fp-len",
        source: "crates/bumbledb/src/storage/store/fingerprint.rs FP_LEN",
        value: "16",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L07",
        disposition: "BLAKE3 policy; AEGIS remains optional hash-probe only",
    },
    Constant {
        id: "u32-image-positions",
        source: "crates/bumbledb/src/image.rs / COLT tokens",
        value: "2^32 rows per resident image",
        class: ConstantClass::RepresentationBound,
        owner_lane: "L04",
        disposition: "switch to cursor/scratch before overflow; not a global row cap",
    },
    Constant {
        id: "max-readers",
        source: "crates/bumbledb/src/storage/store/store_env.rs MAX_READERS",
        value: "1024",
        class: ConstantClass::HostPolicy,
        owner_lane: "L07/L12",
        disposition: "typed reader-slot refusal; size from supported owners",
    },
    Constant {
        id: "map-align",
        source: "crates/bumbledb/src/storage/store/map.rs MAP_ALIGN / DEFAULT_INITIAL_MAP",
        value: "1 MiB align / 4 GiB initial",
        class: ConstantClass::HostPolicy,
        owner_lane: "L07",
        disposition: "elastic map; no 32 GiB ceiling. Map ≠ file ≠ RSS",
    },
    Constant {
        id: "scratch-ram",
        source: "crates/bumbledb/src/exec/scratch.rs DEFAULT_RAM_BYTES",
        value: "8 MiB",
        class: ConstantClass::HostPolicy,
        owner_lane: "L03",
        disposition: "RAM-first then LMDB; charged owner, not an RSS identity",
    },
    Constant {
        id: "join-batch",
        source: "crates/bumbledb/src/exec/run.rs BATCH",
        value: "128",
        class: ConstantClass::MeasuredCrossover,
        owner_lane: "L05",
        disposition: "per-target sweep; M2 miss-parallelism does not transfer",
    },
    Constant {
        id: "prefetch-floor",
        source: "crates/bumbledb/src/exec/run.rs PREFETCH_WIDTH_FLOOR",
        value: "4",
        class: ConstantClass::MeasuredCrossover,
        owner_lane: "L05",
        disposition: "hit/miss/phase; count redundant loads, not only time",
    },
    Constant {
        id: "dense-groups",
        source: "crates/bumbledb/src/exec/sink.rs DENSE_GROUPS_CAP",
        value: "4096",
        class: ConstantClass::MeasuredCrossover,
        owner_lane: "L06",
        disposition: "sparse-vs-dense; fallback preserves exact bits",
    },
    Constant {
        id: "memo-slots",
        source: "crates/bumbledb/src/api/prepared.rs MEMO_SLOTS",
        value: "4",
        class: ConstantClass::MeasuredCrossover,
        owner_lane: "L05",
        disposition: "entry count is not a memory budget; tenant cell reports charge",
    },
    Constant {
        id: "set-stride",
        source: "crates/bumbledb/src/image.rs SET_STRIDE",
        value: "16384",
        class: ConstantClass::MeasuredCrossover,
        owner_lane: "L04",
        disposition: "M2-scoped placement; qualify per Apple/Graviton/x86",
    },
    Constant {
        id: "clock-proxy",
        source: "crates/bumbledb-bench/src/clockproxy.rs CONTAMINATION_GHZ",
        value: "3.2",
        class: ConstantClass::Instrumentation,
        owner_lane: "L20",
        disposition: "Apple Silicon scoped until APP-TARGETS recalibrates",
    },
    Constant {
        id: "protocol-warm",
        source: "crates/bumbledb-bench/src/harness.rs Protocol::WARM",
        value: "32/256",
        class: ConstantClass::Instrumentation,
        owner_lane: "L20",
        disposition: "shapes evidence; not a product latency budget",
    },
];

#[must_use]
pub fn by_class(class: ConstantClass) -> impl Iterator<Item = &'static Constant> {
    CONSTANTS.iter().filter(move |c| c.class == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_constant_has_an_owner_and_a_class() {
        assert!(CONSTANTS.len() >= 12);
        for constant in CONSTANTS {
            assert!(!constant.source.is_empty(), "{}", constant.id);
            assert!(!constant.owner_lane.is_empty(), "{}", constant.id);
            assert!(!constant.disposition.is_empty(), "{}", constant.id);
        }
        assert!(by_class(ConstantClass::RepresentationBound).count() >= 4);
        assert!(by_class(ConstantClass::HostPolicy).count() >= 2);
        assert!(by_class(ConstantClass::MeasuredCrossover).count() >= 3);
    }
}
