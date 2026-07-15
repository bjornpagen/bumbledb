//! The device-honesty instrument (docs/architecture/60-validation.md §
//! the ramdisk sanction): a RAM-backed-path detector and the refusal
//! the timed write families raise against it. Verify, differential, and
//! fuzz lanes are exempt — they check answers, not wall clocks, and may
//! run on the ram disk (`scripts/ramdisk.sh`); a *timed* number
//! measured on RAM would be a lie told with a straight face.
//!
//! Mechanism (macOS, the canonical machine): the volume identity comes
//! from the `mount` table (the longest mount-point prefix of the
//! canonicalized path — the `statfs f_fstypename` answer, reached
//! without `libc`: the quarantine is rusqlite and nothing else, so the
//! syscall arrives via `/sbin/mount`'s output instead), and RAM-disk
//! identity from `hdiutil info` (every `ram://`-backed image's device
//! nodes), with the APFS synthesized-container indirection resolved
//! through `diskutil info`'s physical store. Linux (cfg-gated, WRITTEN
//! CAREFULLY BUT UNTESTED — the owner's instruction; this machine is
//! the macOS M2 Max): `/proc/mounts` parsed the same way, `tmpfs`/
//! `ramfs` as the RAM filesystems — the std-only stand-in for
//! `statfs f_type == TMPFS_MAGIC` (`0x0102_1994`), which is unreachable
//! without `libc`.

use std::path::{Path, PathBuf};

/// One resolved volume identity: where the path actually lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeIdentity {
    /// The mount point owning the path (longest-prefix winner).
    pub mount_point: PathBuf,
    /// The filesystem type name (`hfs`, `apfs`, `tmpfs`, …).
    pub fstype: String,
    /// The device node or source the mount table names.
    pub device: String,
    /// Whether the backing store is RAM.
    pub ram_backed: bool,
}

/// The typed device-honesty refusal: a timed family was pointed at a
/// RAM-backed volume. Named, not stringly — the driver renders it at
/// the boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RamBackedRefusal {
    /// What refused to run there.
    pub family_scope: &'static str,
    /// The volume that was RAM-backed.
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
/// its nearest existing ancestor — scratch dirs are created after the
/// check) resolves onto a RAM-backed volume.
///
/// # Errors
///
/// [`RamBackedRefusal`] when the volume is RAM-backed.
///
/// # Panics
///
/// When the mount table cannot be read at all — an unreadable device
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

/// Resolves the volume identity of `path` via the platform mount table.
/// The path need not exist yet: the nearest existing ancestor answers
/// (a scratch directory is asked about before it is created).
///
/// # Errors
///
/// A message when the mount table is unreadable or names no owner.
pub fn volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
    let resolved = canonical_base(path)?;
    imp::volume_identity(&resolved)
}

/// Canonicalizes the deepest existing ancestor of `path` (symlinks like
/// macOS's `/tmp -> /private/tmp` would otherwise dodge the prefix
/// match).
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

/// The longest mount point that is a component-wise prefix of `path`,
/// over `(mount_point, fstype, device)` rows.
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

    /// Parses one `mount` line: `<device> on <mount point> (<fstype>, …)`.
    /// The mount point may contain spaces, so the parse anchors on the
    /// first ` on ` and the last ` (`.
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

    /// The base whole-disk name of a device node: `/dev/disk5s1` →
    /// `disk5` (slice suffixes stripped).
    fn base_disk(device: &str) -> Option<String> {
        let name = device.strip_prefix("/dev/")?.trim();
        let digits_end = name
            .strip_prefix("disk")?
            .find(|c: char| !c.is_ascii_digit())
            .map_or(name.len(), |i| i + 4);
        Some(name[..digits_end].to_owned())
    }

    /// Every whole-disk name backed by a `ram://` image, from
    /// `hdiutil info` (sections split by `=` rules; a section whose
    /// `image-path` starts with `ram://` owns its `/dev/disk` nodes).
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

    /// The APFS synthesized-container indirection: `diskutil info` on
    /// the volume device names its physical store(s); a ram-backed
    /// store makes the volume ram-backed.
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

// The Linux arm: WRITTEN CAREFULLY BUT UNTESTED (the owner's explicit
// instruction — the canonical machine is the macOS M2 Max, and no Linux
// host has run this). `/proc/mounts` is whitespace-separated with
// octal escapes in paths; the fields used here (source, mount point,
// fstype) are the first three.
#[cfg(target_os = "linux")]
mod imp {
    use std::path::{Path, PathBuf};

    use super::VolumeIdentity;

    /// Undoes `/proc/mounts` octal escapes (`\040` space, `\011` tab,
    /// `\012` newline, `\134` backslash).
    fn unescape(field: &str) -> String {
        let mut out = String::with_capacity(field.len());
        let mut chars = field.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            let code: String = chars.by_ref().take(3).collect();
            match u8::from_str_radix(&code, 8) {
                Ok(byte) => out.push(byte as char),
                Err(_) => {
                    out.push('\\');
                    out.push_str(&code);
                }
            }
        }
        out
    }

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
