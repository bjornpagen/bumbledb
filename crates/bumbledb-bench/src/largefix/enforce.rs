//! Enforcement: a sparse file is not populated data, and an unrecorded memory
//! bound is not a beyond-RAM run.

use std::path::Path;

/// Populated-data admission for the >40 GiB gate: the file's **allocated
/// blocks**, not its length, must cover `min_bytes` (with a small filesystem
/// slack tolerance). A big `set_len`/large virtual map fails here by design.
///
/// # Errors
pub fn assert_populated(path: &Path, min_bytes: u64) -> Result<(), String> {
    let length = std::fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|e| format!("stat {}: {e}", path.display()))?;
    let allocated = crate::space::census::allocated_bytes(path)?;
    if length < min_bytes {
        return Err(format!(
            "{}: length {length} B is below the {min_bytes} B fixture minimum",
            path.display()
        ));
    }
    // 95% of the minimum must be actually allocated: filesystems may compress
    // or round, but a sparse fixture is off by orders of magnitude, not 5%.
    let floor = min_bytes / 20 * 19;
    if allocated < floor {
        return Err(format!(
            "{}: only {allocated} B allocated for {length} B length — a sparse file is not \
             populated data (needs >= {floor} B allocated)",
            path.display()
        ));
    }
    Ok(())
}

/// The recorded memory bound of a beyond-RAM run. On Linux this must match
/// the actual cgroup v2 `memory.max`; elsewhere the lane is `NotApplicable`
/// (the gate runs on an isolated Linux runner).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBoundEvidence {
    pub memory_max_bytes: u64,
    pub source: &'static str,
}

/// Read the enforced bound from the current cgroup (Linux, cgroup v2).
/// `"max"` (unlimited) is a refusal: an unlimited cgroup is not a beyond-RAM
/// enforcement. Non-Linux targets refuse with `NotApplicable` semantics.
///
/// # Errors
pub fn read_cgroup_memory_max() -> Result<MemoryBoundEvidence, String> {
    #[cfg(target_os = "linux")]
    {
        let cgroup = std::fs::read_to_string("/proc/self/cgroup")
            .map_err(|e| format!("/proc/self/cgroup: {e}"))?;
        // cgroup v2: one line, `0::<path>`.
        let path = cgroup
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .ok_or_else(|| "no cgroup v2 entry — the runner must use cgroup v2".to_owned())?
            .trim();
        let file = format!("/sys/fs/cgroup{path}/memory.max");
        let raw = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
        let raw = raw.trim();
        if raw == "max" {
            return Err(
                "memory.max is `max` — an unlimited cgroup is not a beyond-RAM \
                        enforcement"
                    .to_owned(),
            );
        }
        let memory_max_bytes: u64 = raw
            .parse()
            .map_err(|_| format!("{file}: unparseable `{raw}`"))?;
        Ok(MemoryBoundEvidence {
            memory_max_bytes,
            source: "cgroup-v2 memory.max",
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(
            "beyond-RAM enforcement needs a Linux cgroup v2 runner; on this target the lane \
             is NotApplicable — never substitute an address-space rlimit, which breaks \
             legitimate sparse LMDB maps"
                .to_owned(),
        )
    }
}

/// The forbidden shortcut, named so the manifest can assert its absence: an
/// `RLIMIT_AS`-style address-space cap must never be used to fake a memory
/// bound.
pub const FORBIDDEN_ENFORCEMENT: &str = "address-space rlimit (RLIMIT_AS)";
