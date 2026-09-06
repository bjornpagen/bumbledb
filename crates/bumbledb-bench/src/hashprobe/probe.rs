//! HASH-04: the pre-format BLAKE3/AEGIS candidate probe.
//!
//! Executes only in F3, on each named target (Apple Silicon first, then
//! Graviton ARM and x86-64), before physical bytes freeze. The probe:
//!
//! 1. checks one-shot/streaming byte equivalence for every candidate over the
//!    whole [`crate::hashprobe::inputs`] corpus and every split schedule —
//!    a mismatch aborts the run before any timing is reported;
//! 2. checks that 16-byte truncation is the exact prefix of the full BLAKE3
//!    digest (output width and hashing time are separate decisions);
//! 3. verifies known-answer vectors when a vector file is supplied
//!    ([`crate::hashprobe::kat`]); absence is recorded as `NotRun`, never as
//!    success;
//! 4. times each candidate per input size and alignment offset, the mixed
//!    short-fact stream, and fresh-state empty-message proxy. State reuse
//!    is not measured; reports say so explicitly.
//!
//! Outputs are bytes, compared as bytes. `TigerBeetle`'s little-endian `u128`
//! checksum convention is deliberately **not** copied into any byte codec
//! here; a persisted format must specify its own byte order explicitly.
//!
//! More bytes per second on a bulk buffer is not necessarily faster per small
//! fact: state initialization/copy dominates short inputs, which is why the
//! corpus starts at 0/8/16 bytes. Every timed call includes fresh state,
//! finalization and the returned Vec allocation; `init-empty` is not an
//! isolated constructor measurement.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::cli::HashProbeArgs;
use crate::harness::{self, Modes, Protocol, Stats};
use crate::report;

use super::inputs::{self, ProbeInput};
use super::kat;

/// Historical derive-key candidate context. Retain its label and bytes for
/// comparison with existing results; this is NOT the current row fingerprint.
pub const FINGERPRINT_PROBE_CONTEXT: &str = "bumbledb v1 2026-09-04 fact fingerprint";

/// The measured candidates. `Blake3Trunc16` shares `Blake3Full32`'s
/// compression work (only the emitted width differs); it exists as its own
/// row so index-density/comparison effects are attributed to width, not to a
/// nonexistent CPU saving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    /// Full 32-byte BLAKE3 — the authoritative-content baseline.
    Blake3Full32,
    /// First 16 bytes of the same plain BLAKE3 digest; a width baseline,
    /// not the production domain-separated row construction.
    Blake3Trunc16,
    /// Historical derive-key candidate, including context-key setup cost.
    Blake3DeriveKey16,
    /// Current row construction: BLAKE3(row domain || relation 0:u32 BE
    /// || message), first 16 bytes. Does not model determinant hashing.
    Blake3RowPrefix16,
    /// AEGIS-128L MAC, zero 16-byte key, 16-byte tag — the TigerBeetle-style
    /// AES-round candidate. A cryptographic construction with a public fixed
    /// key is a checksum, not sender authentication, and the birthday bound
    /// on its 128-bit output still applies.
    Aegis128LMac16,
}

pub const CANDIDATES: [Candidate; 5] = [
    Candidate::Blake3Full32,
    Candidate::Blake3Trunc16,
    Candidate::Blake3DeriveKey16,
    Candidate::Blake3RowPrefix16,
    Candidate::Aegis128LMac16,
];

impl Candidate {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Blake3Full32 => "blake3-full-32",
            Self::Blake3Trunc16 => "blake3-trunc-16",
            Self::Blake3DeriveKey16 => "blake3-derive-key-16",
            Self::Blake3RowPrefix16 => "blake3-row-prefix-rel0-16",
            Self::Aegis128LMac16 => "aegis-128l-mac-16",
        }
    }

    #[must_use]
    pub const fn output_bytes(self) -> usize {
        match self {
            Self::Blake3Full32 => 32,
            Self::Blake3Trunc16
            | Self::Blake3DeriveKey16
            | Self::Blake3RowPrefix16
            | Self::Aegis128LMac16 => 16,
        }
    }
}

/// Mirrors the private row envelope in storage/store/fingerprint.rs, using
/// its public streaming Digest primitive. `RelationId` is u32; the domain is
/// raw prefix bytes, NOT a derive-key context. No prepared-state cache:
/// production creates and feeds this prefix on every row fingerprint.
fn row_prefix_state() -> bumbledb::digest::Digest {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(b"bumbledb/1/row-fp");
    digest.update(&bumbledb::RelationId(0).0.to_be_bytes());
    digest
}

/// AEGIS MAC adapter — one place to fix if the pinned `aegis` crate exposes a
/// different constructor shape. Contract: zero 16-byte key, message as the
/// authenticated data of an empty encrypted message, 16-byte tag returned as
/// bytes. The dependency request (Cargo hub patch) pins `aegis = "=0.9.15"`;
/// verify this call against that exact source in F3 before trusting output.
fn aegis128l_mac16_oneshot(message: &[u8]) -> [u8; 16] {
    let key = [0u8; 16];
    let mut mac = aegis::aegis128l::Aegis128LMac::<16>::new(&key);
    mac.update(message);
    mac.finalize()
}

fn aegis128l_mac16_streaming(chunks: &mut dyn Iterator<Item = &[u8]>) -> [u8; 16] {
    let key = [0u8; 16];
    let mut mac = aegis::aegis128l::Aegis128LMac::<16>::new(&key);
    for chunk in chunks {
        mac.update(chunk);
    }
    mac.finalize()
}

/// One-shot digest as raw bytes.
#[must_use]
pub fn digest_oneshot(candidate: Candidate, message: &[u8]) -> Vec<u8> {
    match candidate {
        Candidate::Blake3Full32 => blake3::hash(message).as_bytes().to_vec(),
        Candidate::Blake3Trunc16 => blake3::hash(message).as_bytes()[..16].to_vec(),
        Candidate::Blake3DeriveKey16 => {
            let mut hasher = blake3::Hasher::new_derive_key(FINGERPRINT_PROBE_CONTEXT);
            hasher.update(message);
            hasher.finalize().as_bytes()[..16].to_vec()
        }
        Candidate::Blake3RowPrefix16 => {
            let mut digest = row_prefix_state();
            digest.update(message);
            digest.finalize()[..16].to_vec()
        }
        Candidate::Aegis128LMac16 => aegis128l_mac16_oneshot(message).to_vec(),
    }
}

/// Streaming digest over a split schedule (chunk lengths summing to
/// `message.len()`). Must equal [`digest_oneshot`] byte-for-byte.
///
/// # Panics
/// When the schedule does not cover the message exactly — a probe programmer
/// error, loud at authoring time.
#[must_use]
pub fn digest_streaming(candidate: Candidate, message: &[u8], schedule: &[usize]) -> Vec<u8> {
    assert_eq!(
        schedule.iter().sum::<usize>(),
        message.len(),
        "split schedule must cover the message exactly"
    );
    let mut chunks = Vec::with_capacity(schedule.len());
    let mut cursor = 0usize;
    for &take in schedule {
        chunks.push(&message[cursor..cursor + take]);
        cursor += take;
    }
    match candidate {
        Candidate::Blake3Full32 | Candidate::Blake3Trunc16 => {
            let mut hasher = blake3::Hasher::new();
            for chunk in &chunks {
                hasher.update(chunk);
            }
            let full = hasher.finalize();
            full.as_bytes()[..candidate.output_bytes()].to_vec()
        }
        Candidate::Blake3DeriveKey16 => {
            let mut hasher = blake3::Hasher::new_derive_key(FINGERPRINT_PROBE_CONTEXT);
            for chunk in &chunks {
                hasher.update(chunk);
            }
            hasher.finalize().as_bytes()[..16].to_vec()
        }
        Candidate::Blake3RowPrefix16 => {
            let mut digest = row_prefix_state();
            for chunk in &chunks {
                digest.update(chunk);
            }
            digest.finalize()[..16].to_vec()
        }
        Candidate::Aegis128LMac16 => aegis128l_mac16_streaming(&mut chunks.into_iter()).to_vec(),
    }
}

/// The equivalence phase: every candidate, every corpus input, every split
/// schedule; plus the truncation-is-prefix law. Runs before any timing so a
/// wrong implementation can never publish a throughput number.
///
/// # Errors
pub fn check_equivalence(corpus: &[ProbeInput]) -> Result<(), String> {
    for input in corpus {
        let message = input.slice();
        let full = digest_oneshot(Candidate::Blake3Full32, message);
        let trunc = digest_oneshot(Candidate::Blake3Trunc16, message);
        if trunc != full[..16] {
            return Err(format!(
                "{}: 16-byte truncation is not the 32-byte prefix",
                input.name
            ));
        }
        for candidate in CANDIDATES {
            let oneshot = digest_oneshot(candidate, message);
            if oneshot.len() != candidate.output_bytes() {
                return Err(format!(
                    "{}: {} emitted {} bytes, expected {}",
                    input.name,
                    candidate.name(),
                    oneshot.len(),
                    candidate.output_bytes()
                ));
            }
            for schedule in inputs::split_schedules(message.len()) {
                let streamed = digest_streaming(candidate, message, &schedule);
                if streamed != oneshot {
                    return Err(format!(
                        "{}: {} streaming digest over {:?}-chunk schedule diverges from one-shot",
                        input.name,
                        candidate.name(),
                        schedule.len()
                    ));
                }
            }
        }
        // Domain separation is real: the derive-key fingerprint must differ
        // from the plain truncated digest (equal would mean the context label
        // is silently ignored).
        let plain16 = digest_oneshot(Candidate::Blake3Trunc16, message);
        let derived16 = digest_oneshot(Candidate::Blake3DeriveKey16, message);
        if plain16 == derived16 {
            return Err(format!(
                "{}: derive-key digest equals the plain digest — domain separation lost",
                input.name
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TimingRow {
    pub candidate: &'static str,
    pub input: String,
    pub len: usize,
    pub align_offset: usize,
    pub stats: Stats,
    /// Message payload bytes across samples; excludes domain/scope framing.
    pub work_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ProbeReport {
    pub provenance: report::Provenance,
    pub seed: u64,
    pub equivalence: &'static str,
    pub kat: kat::KatOutcome,
    pub rows: Vec<TimingRow>,
    /// Fresh-state empty-message proxy; includes finalization and Vec allocation.
    pub init_rows: Vec<TimingRow>,
    /// Mixed short-fact stream rows (one row per candidate over the stream).
    pub mixture_rows: Vec<TimingRow>,
}

fn time_candidate(
    candidate: Candidate,
    name: String,
    len: usize,
    align_offset: usize,
    proto: Protocol,
    message: &[u8],
) -> Result<TimingRow, String> {
    let m = harness::measure_batched(proto, Modes::default(), batch_for(len), || {
        std::hint::black_box(digest_oneshot(candidate, std::hint::black_box(message)));
        Ok(len as u64)
    })?;
    Ok(TimingRow {
        candidate: candidate.name(),
        input: name,
        len,
        align_offset,
        stats: m.stats,
        work_bytes: m.work,
    })
}

/// Short inputs are batched so the sample duration clears
/// [`harness::QUANTUM_FLOOR_NS`]; bulk inputs use batch 1.
fn batch_for(len: usize) -> u32 {
    match len {
        0..=128 => 512,
        129..=4096 => 64,
        _ => 1,
    }
}

/// The full probe lane (CLI `hash-probe`). Report-class: artifacts +
/// stdout markdown; exit 1 on equivalence/KAT failure.
///
/// # Errors
pub fn run(args: &HashProbeArgs) -> Result<i32, String> {
    let corpus = inputs::corpus(args.seed);
    if let Err(message) = check_equivalence(&corpus) {
        eprintln!("hash-probe equivalence failure: {message}");
        return Ok(1);
    }
    let kat = match &args.kat {
        Some(path) => {
            let outcome = kat::verify_blake3_file(path)?;
            if let kat::KatOutcome::Failed(detail) = &outcome {
                eprintln!("hash-probe KAT failure: {detail}");
                return Ok(1);
            }
            outcome
        }
        None => kat::KatOutcome::NotRun("no --kat vector file supplied".to_owned()),
    };

    let proto = Protocol {
        warmups: 8,
        samples: args.samples.unwrap_or(64),
    };
    let mut rows = Vec::new();
    for input in &corpus {
        for candidate in CANDIDATES {
            rows.push(time_candidate(
                candidate,
                input.name.clone(),
                input.len,
                input.align_offset,
                proto,
                input.slice(),
            )?);
        }
    }
    let mut init_rows = Vec::new();
    for candidate in CANDIDATES {
        init_rows.push(time_candidate(
            candidate,
            "init-empty".to_owned(),
            0,
            0,
            proto,
            &[],
        )?);
    }
    let stream = inputs::mixture(args.seed, 4096);
    let mut mixture_rows = Vec::new();
    for candidate in CANDIDATES {
        let mut cursor = 0usize;
        let m = harness::measure_batched(proto, Modes::default(), 64, || {
            let input = &stream[cursor % stream.len()];
            cursor += 1;
            std::hint::black_box(digest_oneshot(
                candidate,
                std::hint::black_box(input.slice()),
            ));
            Ok(input.len as u64)
        })?;
        mixture_rows.push(TimingRow {
            candidate: candidate.name(),
            input: "short-fact-mixture".to_owned(),
            len: 0,
            align_offset: 0,
            stats: m.stats,
            work_bytes: m.work,
        });
    }

    let probe = ProbeReport {
        provenance: report::provenance(Path::new(".")),
        seed: args.seed,
        equivalence: "passed",
        kat,
        rows,
        init_rows,
        mixture_rows,
    };
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-hash-probe",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    std::fs::write(out_dir.join("hash-probe.json"), to_json(&probe))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = render(&probe);
    std::fs::write(out_dir.join("hash-probe.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}

fn push_row(out: &mut String, row: &TimingRow) {
    let _ = write!(
        out,
        "{{\"candidate\":\"{}\",\"input\":\"{}\",\"len\":{},\"align_offset\":{},\"p50_ns\":{},\"p99_ns\":{},\"min_ns\":{},\"max_ns\":{},\"mean_ns\":{},\"work_bytes\":{}}}",
        row.candidate,
        row.input,
        row.len,
        row.align_offset,
        row.stats.p50,
        row.stats.p99,
        row.stats.min,
        row.stats.max,
        row.stats.mean_ns,
        row.work_bytes,
    );
}

fn push_rows(out: &mut String, key: &str, rows: &[TimingRow]) {
    let _ = write!(out, ",\"{key}\":[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(out, row);
    }
    out.push(']');
}

#[must_use]
pub fn to_json(probe: &ProbeReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    report::push_provenance(&mut out, &probe.provenance);
    let _ = write!(
        out,
        ",\"seed\":{},\"equivalence\":\"{}\",\"kat\":",
        probe.seed, probe.equivalence
    );
    probe.kat.push_json(&mut out);
    out.push_str(
        ",\"kat_scope\":[\"blake3-full-32\"],\"reuse_timing\":\"not-run\",\"timing_scope\":\"fresh-state-finalize-vec\"",
    );
    push_rows(&mut out, "rows", &probe.rows);
    push_rows(&mut out, "init_rows", &probe.init_rows);
    push_rows(&mut out, "mixture_rows", &probe.mixture_rows);
    out.push('}');
    out
}

fn render(probe: &ProbeReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Hash candidate probe\n");
    let _ = writeln!(
        out,
        "Equivalence: {}. KAT: {}. Seed {}. Times are per-hash; work is message \
         payload bytes across samples, excluding framing. Output width and hashing time are separate \
         decisions — trunc-16 shares full-32 compression work by construction.\n",
        probe.equivalence,
        probe.kat.describe(),
        probe.seed
    );
    let _ = writeln!(
        out,
        "Production row candidate: blake3-row-prefix-rel0-16. The derive-key label \
         retains its historical construction. KAT status covers only plain \
         blake3-full-32; other construction KATs and reuse timing are not run. \
         All timings include fresh state, finalization and Vec allocation.\n"
    );
    let _ = writeln!(
        out,
        "| candidate | input | len | off | p50 ns | p99 ns | mean ns |"
    );
    let _ = writeln!(out, "|---|---|---:|---:|---:|---:|---:|");
    for row in probe
        .rows
        .iter()
        .chain(&probe.init_rows)
        .chain(&probe.mixture_rows)
    {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} |",
            row.candidate,
            row.input,
            row.len,
            row.align_offset,
            row.stats.p50,
            row.stats.p99,
            row.stats.mean_ns,
        );
    }
    out
}
