use std::sync::OnceLock;

/// The switch: `BUMBLEDB_BENCH_BOOST=1` boosts, unset/empty/`0` does not,
/// anything else is a refusal naming the remedy.
pub const ENV: &str = "BUMBLEDB_BENCH_BOOST";

#[cfg(target_os = "macos")]
pub const QOS_LABEL: &str = "qos-user-interactive";

#[cfg(target_os = "linux")]
pub const QOS_LABEL: &str = "nice--10";

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const QOS_LABEL: &str = "unsupported";

#[cfg(target_os = "macos")]
const CLAIM_LOG: &str = "scheduler boost: user-interactive QoS verified (not hard P-core affinity)";
#[cfg(target_os = "linux")]
const CLAIM_LOG: &str = "scheduler boost: absolute nice -10 verified";
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const CLAIM_LOG: &str = "scheduler boost: unsupported platform";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Engaged {
    pub boost: &'static str,

    pub load_start: [f64; 3],
}

static ENGAGED: OnceLock<Engaged> = OnceLock::new();

fn wants_boost(value: Option<&str>) -> Result<bool, String> {
    match value {
        None | Some("" | "0") => Ok(false),
        Some("1") => Ok(true),
        Some(other) => Err(format!(
            "{ENV} must be 1 (boost) or 0/unset (no boost), got `{other}`"
        )),
    }
}

/// # Errors
pub fn engage_from_env() -> Result<(), String> {
    if !wants_boost(std::env::var(ENV).ok().as_deref())? {
        return Ok(());
    }
    claim_qos()?;
    let _ = ENGAGED.set(Engaged {
        boost: QOS_LABEL,
        load_start: loadavg(),
    });
    eprintln!("{CLAIM_LOG}");
    Ok(())
}

#[must_use]
pub fn engaged() -> Option<Engaged> {
    ENGAGED.get().copied()
}

/// # Errors
#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "public darwin API (pthread/qos.h); the dependency quarantine allows no libc crate, so the binding is declared raw"
)]
pub fn claim_qos() -> Result<(), String> {
    #[expect(non_camel_case_types, reason = "the darwin header's spelling")]
    type qos_class_t = core::ffi::c_uint;

    const QOS_CLASS_USER_INTERACTIVE: qos_class_t = 0x21;
    unsafe extern "C" {
        fn pthread_set_qos_class_self_np(
            qos_class: qos_class_t,
            relative_priority: core::ffi::c_int,
        ) -> core::ffi::c_int;
        fn pthread_self() -> *mut core::ffi::c_void;
        fn pthread_get_qos_class_np(
            thread: *mut core::ffi::c_void,
            qos_class: *mut qos_class_t,
            relative_priority: *mut core::ffi::c_int,
        ) -> core::ffi::c_int;
    }
    // SAFETY: an FFI call with no pointers; it changes only the calling

    let rc = unsafe { pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0) };
    if rc != 0 {
        return Err(format!("pthread_set_qos_class_self_np returned {rc}"));
    }
    let mut class = 0;
    let mut relative = 0;
    // SAFETY: pthread_self names this live thread; both output pointers refer
    // to initialized, writable scalars of the public Darwin ABI types.
    let rc = unsafe { pthread_get_qos_class_np(pthread_self(), &raw mut class, &raw mut relative) };
    if rc != 0 || class != QOS_CLASS_USER_INTERACTIVE || relative != 0 {
        return Err(format!(
            "QoS readback failed: rc={rc}, class={class}, relative={relative}"
        ));
    }
    Ok(())
}

/// # Errors
#[cfg(target_os = "linux")]
pub fn claim_qos() -> Result<(), String> {
    claim_linux_nice(-10)
}

#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "public Linux libc API; only the calling process's main thread is changed"
)]
fn claim_linux_nice(priority: core::ffi::c_int) -> Result<(), String> {
    unsafe extern "C" {
        fn setpriority(
            which: core::ffi::c_int,
            who: core::ffi::c_uint,
            priority: core::ffi::c_int,
        ) -> core::ffi::c_int;
        fn getpriority(which: core::ffi::c_int, who: core::ffi::c_uint) -> core::ffi::c_int;
        fn __errno_location() -> *mut core::ffi::c_int;
    }
    const PRIO_PROCESS: core::ffi::c_int = 0;
    // SAFETY: who=0 targets the current main thread, before benchmark work.
    // Children inherit the scheduling priority. No pointers are passed.
    if unsafe { setpriority(PRIO_PROCESS, 0, priority) } != 0 {
        return Err(format!(
            "cannot set benchmark nice to {priority}: {}. A suitable RLIMIT_NICE or CAP_SYS_NICE is required; no fallback was used",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: libc provides the calling thread's valid errno slot. Clear it
    // because -1 is also a successful getpriority result, not necessarily errno.
    let (actual, errno) = unsafe {
        *__errno_location() = 0;
        let actual = getpriority(PRIO_PROCESS, 0);
        (actual, *__errno_location())
    };
    if errno != 0 || actual != priority {
        return Err(format!(
            "nice readback failed: wanted {priority}, got {actual}, errno={errno}"
        ));
    }
    Ok(())
}

/// # Errors
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn claim_qos() -> Result<(), String> {
    Err("scheduler priority control is unsupported on this platform".to_owned())
}

#[cfg(unix)]
#[expect(
    unsafe_code,
    reason = "public POSIX API; the dependency quarantine allows no libc crate, so the binding is declared raw"
)]
#[must_use]
pub fn loadavg() -> [f64; 3] {
    unsafe extern "C" {
        fn getloadavg(loadavg: *mut f64, nelem: core::ffi::c_int) -> core::ffi::c_int;
    }
    let mut load = [-1.0f64; 3];
    // SAFETY: the pointer names a live 3-slot f64 buffer and nelem is

    let _filled = unsafe { getloadavg(load.as_mut_ptr(), 3) };

    load
}

#[cfg(not(unix))]
#[must_use]
pub fn loadavg() -> [f64; 3] {
    [-1.0; 3]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// refusal naming the variable.
    #[test]
    fn the_switch_semantics_are_pinned() {
        assert_eq!(wants_boost(None), Ok(false));
        assert_eq!(wants_boost(Some("")), Ok(false));
        assert_eq!(wants_boost(Some("0")), Ok(false));
        assert_eq!(wants_boost(Some("1")), Ok(true));
        let err = wants_boost(Some("yes")).unwrap_err();
        assert!(err.contains(ENV), "{err}");
        assert!(err.contains("yes"), "{err}");
    }

    /// success on macOS (rc 0 asserted; no timing behavior). Each test
    #[test]
    #[cfg(target_os = "macos")]
    fn the_qos_claim_succeeds() {
        claim_qos().expect("the user-interactive QoS claim succeeds");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_nice_is_absolute_and_read_back_without_privileged_escalation() {
        // Increasing niceness to 19 is permitted without capabilities. nextest
        // isolates this process; lowering its priority cannot change other tests.
        claim_linux_nice(19).expect("unprivileged priority reduction succeeds");
        claim_linux_nice(19).expect("absolute assignment does not add another 19");
    }

    #[test]
    fn loadavg_slots_are_samples_or_markers() {
        for slot in loadavg() {
            assert!(
                slot >= 0.0 || (slot + 1.0).abs() < f64::EPSILON,
                "slot {slot} is neither a sample nor the -1.0 marker"
            );
        }
    }

    #[test]
    fn the_night_script_parses_and_names_shared() {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/bench-night.sh");
        let parsed = std::process::Command::new("bash")
            .args(["-n", script])
            .status()
            .expect("bash runs");
        assert!(parsed.success(), "bash -n rejects bench-night.sh");
        let help = std::process::Command::new("bash")
            .args([script, "--help"])
            .output()
            .expect("bash runs");
        assert!(help.status.success(), "--help exits 0");
        let text = String::from_utf8_lossy(&help.stdout).into_owned()
            + &String::from_utf8_lossy(&help.stderr);
        assert!(
            text.contains("--shared"),
            "usage never names --shared: {text}"
        );
    }
}
