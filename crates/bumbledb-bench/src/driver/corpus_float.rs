//! The `corpus-float` arm: renders P11's deterministic float fixture corpus
//! (`crate::corpus_gen::float_corpus`) to fixed line-hex files. A generator
//! like `gen` — it never measures, and its output bytes are pure functions
//! of the flags, so the emitted files are reviewable and diffable.
//!
//! The dispatched generators and the aggregate rendering are P11's
//! (`canon_corpus` / `order_corpus` / `arith_corpus` / `agg_corpus` /
//! `render_agg`); this arm owns only the flag plumbing and the canon/order/
//! arith line formats (16-hex-digit words, `->` separating inputs from
//! oracle expectations — the `render_agg` convention).
use std::fmt::Write as _;
use std::path::Path;

use crate::cli::CorpusFloatArgs;
use crate::corpus_gen::float_corpus::{
    ArithCase, CanonCase, OrderCase, agg_corpus, arith_corpus, canon_corpus, order_corpus,
    render_agg,
};

fn render_canon(cases: &[CanonCase]) -> String {
    let mut out = String::new();
    for case in cases {
        let _ = writeln!(out, "{:016x} -> {:016x}", case.raw, case.expected);
    }
    out
}

fn render_order(cases: &[OrderCase]) -> String {
    let mut out = String::new();
    for case in cases {
        let _ = writeln!(
            out,
            "{:016x} {:016x} -> {:016x} {:016x}",
            case.lhs, case.rhs, case.lhs_key, case.rhs_key
        );
    }
    out
}

fn render_arith(cases: &[ArithCase]) -> String {
    let mut out = String::new();
    for case in cases {
        let _ = writeln!(
            out,
            "{:016x} {:016x} -> add {:016x} sub {:016x} mul {:016x} div {:016x}",
            case.lhs, case.rhs, case.add, case.sub, case.mul, case.div
        );
    }
    out
}

fn write_file(dir: &Path, name: &str, contents: &str) -> Result<(), String> {
    let path = dir.join(name);
    std::fs::write(&path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

/// # Errors
/// Filesystem failures only — the generators are total.
pub fn cmd_corpus_float(args: &CorpusFloatArgs) -> Result<(), String> {
    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("out dir {}: {e}", args.out.display()))?;
    let canon = canon_corpus(args.seed, args.random);
    let order = order_corpus();
    let arith = arith_corpus(args.seed, args.random);
    let agg = agg_corpus(args.seed, args.groups, args.group_size);
    write_file(&args.out, "canon.txt", &render_canon(&canon))?;
    write_file(&args.out, "order.txt", &render_order(&order))?;
    write_file(&args.out, "arith.txt", &render_arith(&arith))?;
    write_file(&args.out, "agg.txt", &render_agg(&agg))?;
    println!(
        "corpus-float: seed {:#x}, {} canon / {} order / {} arith / {} agg cases -> {}",
        args.seed,
        canon.len(),
        order.len(),
        arith.len(),
        agg.len(),
        args.out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CorpusFloatArgs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-corpus-float-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The emitted files are deterministic byte-for-byte in the flags, every
    /// line carries the `->` oracle separator, and the named chapter 11
    /// aggregate goldens land in `agg.txt` (F-GOLDEN / F-CROSS fixture
    /// substrate).
    #[test]
    fn corpus_float_emits_deterministic_line_hex_files() {
        let dir = scratch("emit");
        let args = CorpusFloatArgs {
            seed: 0xB0B,
            random: 8,
            groups: 2,
            group_size: 4,
            out: dir.join("float"),
        };
        cmd_corpus_float(&args).expect("the generator writes");
        let first: Vec<String> = ["canon.txt", "order.txt", "arith.txt", "agg.txt"]
            .iter()
            .map(|name| std::fs::read_to_string(args.out.join(name)).expect("written"))
            .collect();
        for (name, text) in ["canon.txt", "order.txt", "arith.txt", "agg.txt"]
            .iter()
            .zip(&first)
        {
            assert!(!text.is_empty(), "{name} is non-empty");
            for line in text.lines() {
                assert!(line.contains(" -> "), "{name}: oracle separator in {line}");
            }
        }
        assert!(
            first[3].contains("7ff0000000000000"),
            "the MAX+MAX overflow golden reaches agg.txt"
        );
        // Regenerate into a second directory: identical bytes.
        let twin = CorpusFloatArgs {
            out: dir.join("float-twin"),
            ..args.clone()
        };
        cmd_corpus_float(&twin).expect("the twin writes");
        for (name, text) in ["canon.txt", "order.txt", "arith.txt", "agg.txt"]
            .iter()
            .zip(&first)
        {
            let again = std::fs::read_to_string(twin.out.join(name)).expect("written");
            assert_eq!(&again, text, "{name} is deterministic");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
