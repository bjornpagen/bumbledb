use std::path::Path;

use super::Provenance;

/// Resolves provenance from the environment (best-effort fields fall
/// back to "unknown").
#[must_use]
pub fn provenance(repo_dir: &Path) -> Provenance {
    Provenance {
        crate_version: env!("CARGO_PKG_VERSION").to_owned(),
        git_rev: git_rev(repo_dir),
        timestamp: timestamp_iso8601(),
        host: host_description(),
    }
}

fn command_line(program: &str, args: &[&str], dir: Option<&Path>) -> Option<String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_owned())
}

/// The engine git rev at runtime; "unknown" outside a repo.
#[must_use]
pub fn git_rev(repo_dir: &Path) -> String {
    command_line("git", &["rev-parse", "HEAD"], Some(repo_dir))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Best-effort host description (`sysctl -n machdep.cpu.brand_string`).
#[must_use]
pub fn host_description() -> String {
    command_line("sysctl", &["-n", "machdep.cpu.brand_string"], None)
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Civil-from-days (Howard Hinnant's algorithm) — hand-rolled ISO-8601.
pub(super) fn civil(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).expect("epoch days fit");
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The current UTC time, ISO-8601.
#[must_use]
pub fn timestamp_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    civil(secs)
}
