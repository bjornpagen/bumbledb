//! lanes — it still runs, and must run, on bare metal before a release.

use super::volume_identity;

#[test]
fn system_temp_dir_is_not_ram_backed() {
    let identity = volume_identity(&std::env::temp_dir()).expect("identity resolves");
    assert!(
        !identity.ram_backed,
        "the system temp dir reported RAM-backed: {identity:?}"
    );
}

/// A path that does not exist yet answers with its nearest existing ancestor's
/// volume — scratch dirs are checked before creation.
#[test]
fn unborn_path_answers_with_its_ancestor() {
    let path = std::env::temp_dir().join("bumbledb-devhonesty-unborn/deeper/still");
    let identity = volume_identity(&path).expect("identity resolves");
    assert!(!identity.ram_backed);
}

#[test]
fn octal_unescape_assembles_multibyte_utf8() {
    assert_eq!(super::unescape(r"/mnt/b\303\266se\040dir"), "/mnt/böse dir");

    assert_eq!(
        super::unescape(r"/mnt/a\040b\011c\012d\134e"),
        "/mnt/a b\tc\nd\\e"
    );

    assert_eq!(super::unescape(r"a\9xb"), r"a\9xb");
}

#[cfg(target_os = "macos")]
mod on_a_live_ram_disk {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// by drop (panic paths included).
    struct ScriptRamDisk {
        name: String,
        mount: PathBuf,
    }

    enum RamDiskProbe {
        Attached(ScriptRamDisk),
        Unavailable(String),
    }

    fn script() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ramdisk.sh")
    }

    impl ScriptRamDisk {
        fn probe() -> RamDiskProbe {
            let name = format!("bumbledb-devlock-{}", std::process::id());
            let out = match Command::new("bash")
                .args([
                    script().to_str().expect("utf-8 path"),
                    "create",
                    "--size-gib",
                    "1",
                    "--name",
                    &name,
                ])
                .output()
            {
                Ok(out) => out,
                Err(e) => return RamDiskProbe::Unavailable(format!("spawn ramdisk.sh: {e}")),
            };
            if !out.status.success() {
                return RamDiskProbe::Unavailable(format!(
                    "ramdisk.sh create failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            match String::from_utf8(out.stdout) {
                Ok(s) => RamDiskProbe::Attached(Self {
                    name,
                    mount: PathBuf::from(s.trim()),
                }),
                Err(_) => RamDiskProbe::Unavailable("ramdisk.sh stdout not UTF-8".into()),
            }
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

    /// before a release: `cargo test -p bumbledb-bench

    #[test]
    #[ignore = "needs a live ram disk (hdiutil attach); run on bare metal before a release"]
    fn timed_families_refuse_a_live_ram_disk() {
        let disk = match ScriptRamDisk::probe() {
            RamDiskProbe::Attached(disk) => disk,
            RamDiskProbe::Unavailable(reason) => {
                panic!("bare-metal lane requires a live ram disk: {reason}")
            }
        };

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

        // before loading any corpus.
        let err = crate::driver::write_families::write_families(
            crate::corpus_gen::GenConfig {
                seed: 7,
                scale: crate::corpus_gen::Scale::Tiny,
            },
            &disk.mount.join("scratch"),
            &|name| name == "commit_single",
            crate::duralane::DurabilityLane::Durable,
            None,
            &mut Vec::new(),
        )
        .expect_err("a timed family on a ram disk must refuse");
        assert!(
            err.contains("device honesty") && err.contains("RAM-backed"),
            "the refusal must say why by name: {err}"
        );

        // preflight, before generating any corpus there (the corpus

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
