//! The device-honesty instrument: a RAM-backed-path detector and the refusal
//! corpus `--dir` (`driver::bench`), the write families their scratch
//! (`driver::write_families`).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeIdentity {
    pub mount_point: PathBuf,

    pub fstype: String,

    pub device: String,

    pub ram_backed: bool,
}

/// The typed device-honesty refusal: a timed family was pointed at a RAM-backed
/// volume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RamBackedRefusal {
    /// What refused to run there.
    pub family_scope: &'static str,

    pub identity: VolumeIdentity,
}

impl std::fmt::Display for RamBackedRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "device honesty: {} refuse to time against the RAM-backed volume {} \
             ({} on {}) — timed families need a disk; the ram disk is for the \
             verify/differential/fuzz lanes (docs/architecture/60-validation.md)",
            self.family_scope,
            self.identity.mount_point.display(),
            self.identity.fstype,
            self.identity.device,
        )
    }
}

/// The timed-family gate: refuses when `path` (or, before it exists,
/// # Errors
/// # Panics
/// answer must not silently pass a timed run (tool invariant).
pub fn assert_disk_backed(
    path: &Path,
    family_scope: &'static str,
) -> Result<(), Box<RamBackedRefusal>> {
    let identity = volume_identity(path).unwrap_or_else(|e| {
        // A detector that cannot answer must not wave a timed run
        // through: tool-invariant, so it stops the run loudly.
        panic!(
            "device honesty: cannot resolve the volume identity of {}: {e}",
            path.display()
        )
    });
    if identity.ram_backed {
        return Err(Box::new(RamBackedRefusal {
            family_scope,
            identity,
        }));
    }
    Ok(())
}

/// The path need not exist yet: the nearest existing ancestor answers (a
/// scratch directory is asked about before it is created).
/// # Errors
pub fn volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
    let resolved = canonical_base(path)?;
    imp::volume_identity(&resolved)
}

fn canonical_base(path: &Path) -> Result<PathBuf, String> {
    let mut probe = path;
    loop {
        match std::fs::canonicalize(probe) {
            Ok(real) => return Ok(real),
            Err(_) => match probe.parent() {
                Some(parent) => probe = parent,
                None => return Err(format!("no existing ancestor of {}", path.display())),
            },
        }
    }
}

fn longest_prefix_owner(
    rows: Vec<(PathBuf, String, String)>,
    path: &Path,
) -> Option<(PathBuf, String, String)> {
    rows.into_iter()
        .filter(|(mnt, ..)| path.starts_with(mnt))
        .max_by_key(|(mnt, ..)| mnt.components().count())
}

#[cfg(target_os = "macos")]
mod imp {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::VolumeIdentity;

    fn read(cmd: &str, args: &[&str]) -> Result<String, String> {
        let out = Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| format!("spawn {cmd}: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "{cmd} {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        String::from_utf8(out.stdout).map_err(|e| format!("{cmd} output not UTF-8: {e}"))
    }

    fn parse_mount_line(line: &str) -> Option<(PathBuf, String, String)> {
        let (device, rest) = line.split_once(" on ")?;
        let open = rest.rfind(" (")?;
        let mount_point = &rest[..open];
        let opts = rest[open + 2..].trim_end().trim_end_matches(')');
        let fstype = opts.split(',').next()?.trim();
        Some((
            PathBuf::from(mount_point),
            fstype.to_owned(),
            device.to_owned(),
        ))
    }

    fn base_disk(device: &str) -> Option<String> {
        let name = device.strip_prefix("/dev/")?.trim();
        let digits_end = name
            .strip_prefix("disk")?
            .find(|c: char| !c.is_ascii_digit())
            .map_or(name.len(), |i| i + 4);
        Some(name[..digits_end].to_owned())
    }

    fn ram_disk_bases() -> Result<Vec<String>, String> {
        let out = read("hdiutil", &["info"])?;
        let mut bases = Vec::new();
        for section in out.split("================================================") {
            let is_ram = section.lines().any(|l| {
                l.split_once(':').is_some_and(|(k, v)| {
                    k.trim() == "image-path" && v.trim().starts_with("ram://")
                })
            });
            if !is_ram {
                continue;
            }
            for line in section.lines() {
                let token = line.split_whitespace().next().unwrap_or("");
                if token.starts_with("/dev/disk")
                    && let Some(base) = base_disk(token)
                {
                    bases.push(base);
                }
            }
        }
        Ok(bases)
    }

    fn apfs_physical_bases(device: &str) -> Vec<String> {
        let Ok(out) = read("diskutil", &["info", device]) else {
            return Vec::new();
        };
        out.lines()
            .filter(|l| l.contains("APFS Physical Store"))
            .filter_map(|l| l.rsplit(':').next())
            .filter_map(|v| base_disk(&format!("/dev/{}", v.trim())))
            .collect()
    }

    pub(super) fn volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
        let table = read("mount", &[])?;
        let rows = table.lines().filter_map(parse_mount_line).collect();
        let (mount_point, fstype, device) = super::longest_prefix_owner(rows, path)
            .ok_or_else(|| format!("no mount owns {}", path.display()))?;
        let ram_bases = ram_disk_bases()?;
        let base_is_ram = |base: &String| {
            ram_bases.contains(base)
                || (fstype == "apfs"
                    && apfs_physical_bases(&device)
                        .iter()
                        .any(|b| ram_bases.contains(b)))
        };
        let ram_backed = fstype == "tmpfs" || base_disk(&device).is_some_and(|b| base_is_ram(&b));
        Ok(VolumeIdentity {
            mount_point,
            fstype,
            device,
            ram_backed,
        })
    }
}

#[cfg(any(target_os = "linux", test))]
fn unescape(field: &str) -> String {
    let mut out = Vec::with_capacity(field.len());
    let mut chars = field.chars();
    let mut buf = [0u8; 4];
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        let code: String = chars.by_ref().take(3).collect();
        if let Ok(byte) = u8::from_str_radix(&code, 8) {
            out.push(byte);
        } else {
            out.push(b'\\');
            out.extend_from_slice(code.as_bytes());
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(target_os = "linux")]
mod imp {
    use std::path::{Path, PathBuf};

    use super::{VolumeIdentity, unescape};

    pub(super) fn volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
        let table =
            std::fs::read_to_string("/proc/mounts").map_err(|e| format!("/proc/mounts: {e}"))?;
        let rows = table
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let device = fields.next()?;
                let mount_point = fields.next()?;
                let fstype = fields.next()?;
                Some((
                    PathBuf::from(unescape(mount_point)),
                    fstype.to_owned(),
                    unescape(device),
                ))
            })
            .collect();
        let (mount_point, fstype, device) = super::longest_prefix_owner(rows, path)
            .ok_or_else(|| format!("no mount owns {}", path.display()))?;
        let ram_backed = fstype == "tmpfs" || fstype == "ramfs";
        Ok(VolumeIdentity {
            mount_point,
            fstype,
            device,
            ram_backed,
        })
    }
}

#[cfg(test)]
mod tests;
