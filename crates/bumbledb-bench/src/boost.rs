use std::sync::OnceLock;

/// The switch: `BUMBLEDB_BENCH_BOOST=1` boosts, unset/empty/`0` does not,
/// anything else is a refusal naming the remedy.
pub const ENV: &str = "BUMBLEDB_BENCH_BOOST";

#[cfg(target_os = "macos")]
pub const QOS_LABEL: &str = "qos-user-interactive";

#[cfg(not(target_os = "macos"))]
pub const QOS_LABEL: &str = "noop";

#[cfg(target_os = "macos")]
const CLAIM_LOG: &str = "scheduler boost: user-interactive QoS claimed";
#[cfg(not(target_os = "macos"))]
const CLAIM_LOG: &str = "scheduler boost: requested — no-op on this platform";

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
    }
    // SAFETY: an FFI call with no pointers; it changes only the calling

    let rc = unsafe { pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0) };
    if rc == 0 {
        Ok(())
    } else {
        Err(format!(
            "pthread_set_qos_class_self_np returned {rc} — run without {ENV}=1"
        ))
    }
}

/// # Errors
#[cfg(not(target_os = "macos"))]
pub fn claim_qos() -> Result<(), String> {
    Ok(())
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
    fn the_qos_claim_succeeds() {
        claim_qos().expect("the user-interactive QoS claim succeeds");
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
