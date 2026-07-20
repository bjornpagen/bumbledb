//! The shared-machine scheduling boost (owner ruling, 2026-07-20): a
//! bench night may run on a machine crowded with background agents, and
//! the bench outranks them — the measuring thread claims the highest
//! scheduling priority attainable **without sudo**. On macOS that is
//! the `QOS_CLASS_USER_INTERACTIVE` class via
//! `pthread_set_qos_class_self_np` (a public darwin API, declared here
//! raw — the dependency quarantine allows no libc crate); elsewhere the
//! claim is a no-op.
//!
//! The switch is the `BUMBLEDB_BENCH_BOOST=1` environment variable,
//! consumed once at the dispatch seam (`main`, gated on
//! [`crate::cli::Cmd::runs_measurements`]) — default OFF, so `cargo
//! test` and every non-measuring subcommand never boost. Engaging is
//! recorded process-wide: the boost IS ambient scheduler state, and the
//! record ([`Engaged`]) mirrors it so provenance
//! ([`crate::report::Provenance::shared`]) can never claim an idle
//! machine for a boosted run.

use std::sync::OnceLock;

/// The switch: `BUMBLEDB_BENCH_BOOST=1` boosts, unset/empty/`0` does
/// not, anything else is a refusal naming the remedy.
pub const ENV: &str = "BUMBLEDB_BENCH_BOOST";

/// The provenance spelling of the claimed scheduling class.
#[cfg(target_os = "macos")]
pub const QOS_LABEL: &str = "qos-user-interactive";
/// The provenance spelling where no `QoS` API exists (the no-op claim).
#[cfg(not(target_os = "macos"))]
pub const QOS_LABEL: &str = "noop";

#[cfg(target_os = "macos")]
const CLAIM_LOG: &str = "scheduler boost: user-interactive QoS claimed";
#[cfg(not(target_os = "macos"))]
const CLAIM_LOG: &str = "scheduler boost: requested — no-op on this platform";

/// The record of an engaged boost. Existing at all means the claim
/// succeeded; `load_start` is the 1/5/15-minute load-average sample
/// taken at engagement (the start of the measuring subcommand).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Engaged {
    /// The claimed class, as provenance spells it ([`QOS_LABEL`]).
    pub boost: &'static str,
    /// 1/5/15-minute load averages at engagement; an unsampled slot
    /// reads -1.0 ([`loadavg`]).
    pub load_start: [f64; 3],
}

static ENGAGED: OnceLock<Engaged> = OnceLock::new();

/// The pure switch semantics, separated for testing.
fn wants_boost(value: Option<&str>) -> Result<bool, String> {
    match value {
        None | Some("" | "0") => Ok(false),
        Some("1") => Ok(true),
        Some(other) => Err(format!(
            "{ENV} must be 1 (boost) or 0/unset (no boost), got `{other}`"
        )),
    }
}

/// Reads [`ENV`] and, when it says boost, claims the `QoS` class, records
/// the engagement (with load-at-start), and logs one line to stderr.
/// The dispatch seam calls this once per measuring subcommand; it is
/// idempotent.
///
/// # Errors
///
/// A malformed [`ENV`] value, or a failed `QoS` claim (macOS only) — both
/// name the remedy.
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

/// The engaged boost, if any — provenance construction reads this
/// ([`crate::report::provenance`]) so a boosted run stamps itself.
#[must_use]
pub fn engaged() -> Option<Engaged> {
    ENGAGED.get().copied()
}

/// Promotes the calling thread to `QOS_CLASS_USER_INTERACTIVE` — the
/// highest scheduling class attainable without privileges.
///
/// # Errors
///
/// The nonzero return code of `pthread_set_qos_class_self_np`.
#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "public darwin API (pthread/qos.h); the dependency quarantine allows no libc crate, so the binding is declared raw"
)]
pub fn claim_qos() -> Result<(), String> {
    #[expect(non_camel_case_types, reason = "the darwin header's spelling")]
    type qos_class_t = core::ffi::c_uint;
    // <sys/qos.h>: QOS_CLASS_USER_INTERACTIVE = 0x21.
    const QOS_CLASS_USER_INTERACTIVE: qos_class_t = 0x21;
    unsafe extern "C" {
        fn pthread_set_qos_class_self_np(
            qos_class: qos_class_t,
            relative_priority: core::ffi::c_int,
        ) -> core::ffi::c_int;
    }
    // SAFETY: an FFI call with no pointers; it changes only the calling
    // thread's scheduling class.
    let rc = unsafe { pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0) };
    if rc == 0 {
        Ok(())
    } else {
        Err(format!(
            "pthread_set_qos_class_self_np returned {rc} — run without {ENV}=1"
        ))
    }
}

/// The no-op fallback: only darwin exposes the `QoS` API.
///
/// # Errors
///
/// Never — the signature is the platform-independent seam
/// (`unnecessary_wraps` never fires here: the fn is exported API).
#[cfg(not(target_os = "macos"))]
pub fn claim_qos() -> Result<(), String> {
    Ok(())
}

/// The 1/5/15-minute load averages via `getloadavg` (portable BSD/glibc
/// API, declared raw under the dependency quarantine). A slot the call
/// never filled reads -1.0 — an explicit unsampled marker, never a fake
/// zero load.
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
    // exactly its length; getloadavg fills at most `nelem` slots.
    let _filled = unsafe { getloadavg(load.as_mut_ptr(), 3) };
    // A failed or partial call leaves the -1.0 markers in place.
    load
}

/// The no-getloadavg fallback: every slot unsampled.
#[cfg(not(unix))]
#[must_use]
pub fn loadavg() -> [f64; 3] {
    [-1.0; 3]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The switch is exactly {unset, "", "0"} off / "1" on; junk is a
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

    /// The smoke the ruling asked for: the `QoS` claim itself returns
    /// success on macOS (rc 0 asserted; no timing behavior). Each test
    /// runs on its own thread, so the promotion leaks nowhere.
    #[test]
    fn the_qos_claim_succeeds() {
        claim_qos().expect("the user-interactive QoS claim succeeds");
    }

    /// Every load slot is either a real sample (>= 0) or the explicit
    /// -1.0 unsampled marker — never garbage.
    #[test]
    fn loadavg_slots_are_samples_or_markers() {
        for slot in loadavg() {
            assert!(
                slot >= 0.0 || (slot + 1.0).abs() < f64::EPSILON,
                "slot {slot} is neither a sample nor the -1.0 marker"
            );
        }
    }

    /// `bash -n` accepts the night orchestrator and its usage text
    /// names the --shared flag (the shared-machine night switch).
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
