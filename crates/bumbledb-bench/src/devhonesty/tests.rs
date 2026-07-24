//! The device-honesty lock tests: the detector answers correctly on
//! both sides of the line, and a timed family pointed at a live ram
//! disk refuses by name. The ram-disk case attaches a real 1 GiB disk
//! through `scripts/ramdisk.sh` (this is the canonical macOS machine)
//! and detaches it on every path — the drop guard runs on panic too.

use super::volume_identity;

/// The system temp dir lives on the internal SSD: not RAM-backed.
#[test]
fn system_temp_dir_is_not_ram_backed() {
    let identity = volume_identity(&std::env::temp_dir()).expect("identity resolves");
    assert!(
        !identity.ram_backed,
        "the system temp dir reported RAM-backed: {identity:?}"
    );
}

/// A path that does not exist yet answers with its nearest existing
/// ancestor's volume — scratch dirs are checked before creation.
#[test]
fn unborn_path_answers_with_its_ancestor() {
    let path = std::env::temp_dir().join("bumbledb-devhonesty-unborn/deeper/still");
    let identity = volume_identity(&path).expect("identity resolves");
    assert!(!identity.ram_backed);
}

/// The `/proc/mounts` octal decoder assembles escaped BYTES into UTF-8
/// once at the end: a multi-byte mount path survives, where the
/// char-per-byte push it replaced read `\303\266` as two latin-1 chars.
/// Pure string logic — runs on every host, this one included.
#[test]
fn octal_unescape_assembles_multibyte_utf8() {
    // ö is the two octal-escaped bytes \303 \266; space is \040.
    assert_eq!(super::unescape(r"/mnt/b\303\266se\040dir"), "/mnt/böse dir");
    // The single-byte classics: space, tab, newline, backslash.
    assert_eq!(
        super::unescape(r"/mnt/a\040b\011c\012d\134e"),
        "/mnt/a b\tc\nd\\e"
    );
    // A malformed escape passes through verbatim, never panics.
    assert_eq!(super::unescape(r"a\9xb"), r"a\9xb");
}

#[cfg(target_os = "macos")]
mod on_a_live_ram_disk {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// A live ram disk created through the script under test, detached
    /// by drop (panic paths included).
    struct ScriptRamDisk {
        name: String,
        mount: PathBuf,
    }

    fn script() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ramdisk.sh")
    }

    impl ScriptRamDisk {
        fn create() -> Self {
            let name = format!("bumbledb-devlock-{}", std::process::id());
            let out = Command::new("bash")
                .args([
                    script().to_str().expect("utf-8 path"),
                    "create",
                    "--size-gib",
                    "1",
                    "--name",
                    &name,
                ])
                .output()
                .expect("spawn ramdisk.sh");
            assert!(
                out.status.success(),
                "ramdisk.sh create failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let mount = PathBuf::from(String::from_utf8(out.stdout).expect("utf-8").trim());
            Self { name, mount }
        }
    }

    impl Drop for ScriptRamDisk {
        fn drop(&mut self) {
            let out = Command::new("bash")
                .args([
                    script().to_str().expect("utf-8 path"),
                    "destroy",
                    "--name",
                    &self.name,
                ])
                .output()
                .expect("spawn ramdisk.sh destroy");
            assert!(
                out.status.success(),
                "ramdisk.sh destroy failed — detach {} by hand: {}",
                self.name,
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    /// The lock test: the detector calls the live ram disk RAM-backed,
    /// and the timed write families refuse it by name. One attach
    /// serves both assertions.
    #[test]
    fn timed_families_refuse_a_live_ram_disk() {
        let disk = ScriptRamDisk::create();

        // The detector side.
        let identity = volume_identity(&disk.mount).expect("identity resolves");
        assert!(
            identity.ram_backed,
            "the live ram disk was not detected: {identity:?}"
        );
        assert_eq!(identity.fstype, "hfs", "the script's default is HFS+");

        // The typed refusal, directly.
        let refusal = super::super::assert_disk_backed(&disk.mount, "the timed write families")
            .expect_err("a RAM-backed path must refuse");
        assert!(refusal.identity.ram_backed);

        // The timed write families, end to end: the driver refuses
        // before loading any corpus.
        let err = crate::driver::write_families::write_families(
            crate::corpus_gen::GenConfig {
                seed: 7,
                scale: crate::corpus_gen::Scale::Tiny,
            },
            &disk.mount.join("scratch"),
            &|name| name == "commit_single",
            crate::duralane::DurabilityLane::Durable,
        )
        .expect_err("a timed family on a ram disk must refuse");
        assert!(
            err.contains("device honesty") && err.contains("RAM-backed"),
            "the refusal must say why by name: {err}"
        );

        // The timed read families, end to end: the rule is symmetric
        // (the fixit record) — `bench --dir <ramdisk>` refuses in
        // preflight, before generating any corpus there (the corpus
        // dir on the ram disk stays empty).
        let corpus_dir = disk.mount.join("corpus");
        let err = crate::driver::cmd_bench(&crate::cli::BenchArgs {
            corpus: crate::cli::CorpusArgs {
                scale: crate::corpus_gen::Scale::Tiny,
                seed: 7,
                dir: corpus_dir.clone(),
            },
            families: None,
            samples: None,
            trace: false,
            alloc: false,
            ephemeral: false,
            proxy_per_rep: false,
            out: None,
            i_am_lying: true,
        })
        .expect_err("a timed read run against a RAM-backed --dir must refuse");
        assert!(
            err.contains("device honesty") && err.contains("RAM-backed"),
            "the read-lane refusal must say why by name: {err}"
        );
        assert!(
            !corpus_dir.exists(),
            "the refusal must land before any corpus is generated on the ram disk"
        );
    }
}
