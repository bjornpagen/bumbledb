//! The `micro` report: every batch kernel at every SIMD level this CPU runs,
//! timed against its scalar twin in the same process, plus the `float_stats`
//! read families. Ratios within one run are the result, so host drift between
//! runs does not matter. Outputs are checked bit-identical before any timing;
//! nothing here asserts a speed.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bumbledb::AllenMask;
use bumbledb::kernels::{self, SimdLevel, reference};

use crate::harness::{self, Protocol, Stats, report};
use crate::json;
use crate::worlds::corpus_gen::Rng;
use crate::worlds::float_stats::{self, FloatStatsRow};

/// Which SIMD levels to time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Levels {
    /// Every level this CPU supports.
    All,
    /// Exactly these instruction-set names.
    Named(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicroArgs {
    pub levels: Levels,
    /// Elements per kernel input.
    pub elements: usize,
    /// Rows in the `float_stats` world.
    pub float_rows: u64,
    pub samples: u32,
    pub seed: u64,
    /// Scratch root for the `float_stats` store.
    pub dir: PathBuf,
    /// The JSON report path; the markdown lands beside it.
    pub out: Option<PathBuf>,
}

impl Default for MicroArgs {
    fn default() -> Self {
        Self {
            levels: Levels::All,
            elements: 1 << 16,
            float_rows: 1_000_000,
            samples: 64,
            seed: 1,
            dir: PathBuf::from("bench-data"),
            out: None,
        }
    }
}

/// One kernel run: at a SIMD level, or the scalar twin.
#[derive(Clone, Copy)]
enum Variant {
    Twin,
    Level(SimdLevel),
}

/// The deterministic inputs every kernel reads.
struct Inputs {
    words: Vec<u64>,
    needle: u64,
    bytes: Vec<u8>,
    starts: Vec<u64>,
    ends: Vec<u64>,
    b_starts: Vec<u64>,
    b_ends: Vec<u64>,
    points: Vec<u64>,
    strided: Vec<u64>,
    indices: Vec<u32>,
    codes: Vec<u8>,
    keep: Vec<u8>,
    items: Vec<u32>,
}

/// Field position of the strided folds: values sit at `i * STRIDE + OFFSET`.
const STRIDE: usize = 3;
const OFFSET: usize = 1;
const MASK: AllenMask = AllenMask::INTERSECTS;

impl Inputs {
    fn new(seed: u64, n: usize) -> Self {
        let mut rng = Rng::new(seed);
        let interval = |rng: &mut Rng| {
            let start = rng.range(1 << 20);
            (start, start + 1 + rng.range(64))
        };
        let (starts, ends): (Vec<u64>, Vec<u64>) = (0..n).map(|_| interval(&mut rng)).unzip();
        let (b_starts, b_ends): (Vec<u64>, Vec<u64>) = (0..n).map(|_| interval(&mut rng)).unzip();
        let words: Vec<u64> = (0..n).map(|_| rng.range(1024)).collect();
        let indices: Vec<u32> = (0..n)
            .filter(|_| rng.chance(1, 2))
            .map(|i| u32::try_from(i).expect("element counts fit u32"))
            .collect();
        let mut codes = vec![0; n];
        reference::allen_codes(&starts, &ends, &b_starts, &b_ends, &mut codes);
        Self {
            needle: words[n / 2],
            bytes: (0..n)
                .map(|_| u8::try_from(rng.range(256)).expect("a byte"))
                .collect(),
            points: (0..8).map(|_| rng.range(1 << 20)).collect(),
            strided: (0..n * STRIDE).map(|_| rng.u64()).collect(),
            keep: (0..n).map(|_| u8::from(rng.chance(1, 2))).collect(),
            items: (0..u32::try_from(n).expect("element counts fit u32")).collect(),
            words,
            starts,
            ends,
            b_starts,
            b_ends,
            indices,
            codes,
        }
    }
}

/// What a kernel run produced; twin and level runs must be equal.
#[derive(Debug, Default, PartialEq, Eq)]
struct Outputs {
    positions: Vec<u32>,
    bytes: Vec<u8>,
    sum: u128,
    min_max: (u64, u64),
}

impl Outputs {
    fn clear(&mut self) {
        self.positions.clear();
        self.bytes.clear();
        self.sum = 0;
        self.min_max = (0, 0);
    }
}

struct Kernel {
    name: &'static str,
    run: fn(Variant, &Inputs, &mut Outputs),
}

/// Positions whose Allen code the mask holds, through the twins.
fn twin_positions(codes: &[u8], out: &mut Vec<u32>) {
    let mut keep = vec![0; codes.len()];
    reference::allen_keep(codes, MASK.bits(), &mut keep);
    out.extend(
        keep.iter()
            .enumerate()
            .filter(|(_, keep)| **keep != 0)
            .map(|(i, _)| u32::try_from(i).expect("positions fit u32")),
    );
}

const KERNELS: &[Kernel] = &[
    Kernel {
        name: "filter_eq_u64",
        run: |variant, i, o| match variant {
            Variant::Twin => reference::filter_eq_u64(&i.words, i.needle, &mut o.positions),
            Variant::Level(l) => kernels::filter_eq_u64(l, &i.words, i.needle, &mut o.positions),
        },
    },
    Kernel {
        name: "filter_range_u64",
        run: |variant, i, o| match variant {
            Variant::Twin => reference::filter_range_u64(&i.words, 100, 600, &mut o.positions),
            Variant::Level(l) => kernels::filter_range_u64(l, &i.words, 100, 600, &mut o.positions),
        },
    },
    Kernel {
        name: "filter_eq_u8",
        run: |variant, i, o| match variant {
            Variant::Twin => reference::filter_eq_u8(&i.bytes, 7, &mut o.positions),
            Variant::Level(l) => kernels::filter_eq_u8(l, &i.bytes, 7, &mut o.positions),
        },
    },
    Kernel {
        name: "filter_point_in_u64",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                reference::filter_point_in_u64(&i.starts, &i.ends, i.points[0], &mut o.positions);
            }
            Variant::Level(l) => {
                kernels::filter_point_in_u64(l, &i.starts, &i.ends, i.points[0], &mut o.positions);
            }
        },
    },
    Kernel {
        name: "filter_any_point_in_u64",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                reference::filter_any_point_in_u64(&i.starts, &i.ends, &i.points, &mut o.positions);
            }
            Variant::Level(l) => {
                kernels::filter_any_point_in_u64(
                    l,
                    &i.starts,
                    &i.ends,
                    &i.points,
                    &mut o.positions,
                );
            }
        },
    },
    Kernel {
        name: "fold_sum_u64",
        run: |variant, i, o| {
            let count = i.words.len();
            o.sum = match variant {
                Variant::Twin => reference::fold_sum_u64(&i.strided, STRIDE, OFFSET, count),
                Variant::Level(l) => kernels::fold_sum_u64(l, &i.strided, STRIDE, OFFSET, count),
            };
        },
    },
    Kernel {
        name: "fold_min_max_u64",
        run: |variant, i, o| {
            let count = i.words.len();
            o.min_max = match variant {
                Variant::Twin => reference::fold_min_max_u64(&i.strided, STRIDE, OFFSET, count),
                Variant::Level(l) => {
                    kernels::fold_min_max_u64(l, &i.strided, STRIDE, OFFSET, count)
                }
            };
        },
    },
    Kernel {
        name: "fold_sum_u64_idx",
        run: |variant, i, o| {
            o.sum = match variant {
                Variant::Twin => {
                    reference::fold_sum_u64_idx(&i.strided, STRIDE, OFFSET, &i.indices)
                }
                Variant::Level(l) => {
                    kernels::fold_sum_u64_idx(l, &i.strided, STRIDE, OFFSET, &i.indices)
                }
            };
        },
    },
    Kernel {
        name: "fold_min_max_u64_idx",
        run: |variant, i, o| {
            o.min_max = match variant {
                Variant::Twin => {
                    reference::fold_min_max_u64_idx(&i.strided, STRIDE, OFFSET, &i.indices)
                }
                Variant::Level(l) => {
                    kernels::fold_min_max_u64_idx(l, &i.strided, STRIDE, OFFSET, &i.indices)
                }
            };
        },
    },
    Kernel {
        name: "allen_code_batch",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                o.bytes.resize(i.starts.len(), 0);
                reference::allen_codes(&i.starts, &i.ends, &i.b_starts, &i.b_ends, &mut o.bytes);
            }
            Variant::Level(l) => {
                kernels::allen_code_batch(
                    l,
                    &i.starts,
                    &i.ends,
                    &i.b_starts,
                    &i.b_ends,
                    &mut o.bytes,
                );
            }
        },
    },
    Kernel {
        name: "allen_code_batch_const",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                o.bytes.resize(i.starts.len(), 0);
                reference::allen_codes_const(
                    &i.starts,
                    &i.ends,
                    1 << 19,
                    (1 << 19) + 32,
                    &mut o.bytes,
                );
            }
            Variant::Level(l) => kernels::allen_code_batch_const(
                l,
                &i.starts,
                &i.ends,
                1 << 19,
                (1 << 19) + 32,
                &mut o.bytes,
            ),
        },
    },
    Kernel {
        name: "allen_filter_batch",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                o.bytes.resize(i.codes.len(), 0);
                reference::allen_keep(&i.codes, MASK.bits(), &mut o.bytes);
            }
            Variant::Level(l) => kernels::allen_filter_batch(l, &i.codes, MASK, &mut o.bytes),
        },
    },
    Kernel {
        name: "allen_filter_columns",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                let mut codes = vec![0; i.starts.len()];
                reference::allen_codes(&i.starts, &i.ends, &i.b_starts, &i.b_ends, &mut codes);
                twin_positions(&codes, &mut o.positions);
            }
            Variant::Level(l) => kernels::allen_filter_columns(
                l,
                &i.starts,
                &i.ends,
                &i.b_starts,
                &i.b_ends,
                MASK,
                &mut o.positions,
            ),
        },
    },
    Kernel {
        name: "allen_filter_columns_const",
        run: |variant, i, o| match variant {
            Variant::Twin => {
                let mut codes = vec![0; i.starts.len()];
                reference::allen_codes_const(
                    &i.starts,
                    &i.ends,
                    1 << 19,
                    (1 << 19) + 32,
                    &mut codes,
                );
                twin_positions(&codes, &mut o.positions);
            }
            Variant::Level(l) => kernels::allen_filter_columns_const(
                l,
                &i.starts,
                &i.ends,
                1 << 19,
                (1 << 19) + 32,
                MASK,
                &mut o.positions,
            ),
        },
    },
    Kernel {
        name: "compact_u32_by_mask",
        run: |variant, i, o| {
            o.positions.extend_from_slice(&i.items);
            match variant {
                Variant::Twin => reference::compact_u32_by_mask(&mut o.positions, &i.keep),
                Variant::Level(l) => kernels::compact_u32_by_mask(l, &mut o.positions, &i.keep),
            }
        },
    },
];

/// One kernel at one level against its twin.
#[derive(Debug, Clone)]
pub struct KernelRow {
    pub kernel: &'static str,
    pub level: &'static str,
    pub level_p50_ns: u64,
    pub twin_p50_ns: u64,
}

impl KernelRow {
    /// How many times faster the level runs than the scalar twin.
    #[must_use]
    pub fn speedup(&self) -> f64 {
        self.twin_p50_ns as f64 / self.level_p50_ns.max(1) as f64
    }
}

fn select(levels: &Levels) -> Result<Vec<SimdLevel>, String> {
    let available = SimdLevel::available();
    match levels {
        Levels::All => Ok(available),
        Levels::Named(names) => names
            .iter()
            .map(|name| {
                available
                    .iter()
                    .copied()
                    .find(|level| level.name() == name)
                    .ok_or_else(|| {
                        format!(
                            "level `{name}` is not available here (available: {})",
                            available
                                .iter()
                                .map(|level| level.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
            })
            .collect(),
    }
}

fn time(
    kernel: &Kernel,
    variant: Variant,
    inputs: &Inputs,
    proto: Protocol,
) -> Result<Stats, String> {
    let mut outputs = Outputs::default();
    harness::measure(proto, || {
        outputs.clear();
        (kernel.run)(variant, inputs, &mut outputs);
        Ok(std::hint::black_box(&outputs).positions.len() as u64)
    })
    .map(|measured| measured.stats)
}

/// Checks every kernel bit-identical to its twin at every selected level,
/// then times both.
/// # Errors
/// An unknown level name, or a kernel that disagrees with its twin.
pub fn kernel_rows(args: &MicroArgs) -> Result<Vec<KernelRow>, String> {
    let levels = select(&args.levels)?;
    let inputs = Inputs::new(args.seed, args.elements);
    let proto = Protocol {
        warmups: args.samples.div_ceil(8),
        samples: args.samples,
    };
    let mut rows = Vec::new();
    for kernel in KERNELS {
        let mut expected = Outputs::default();
        (kernel.run)(Variant::Twin, &inputs, &mut expected);
        for &level in &levels {
            let mut found = Outputs::default();
            (kernel.run)(Variant::Level(level), &inputs, &mut found);
            if found != expected {
                return Err(format!(
                    "{} at {} disagrees with its scalar twin",
                    kernel.name,
                    level.name()
                ));
            }
        }
        let twin = time(kernel, Variant::Twin, &inputs, proto)?;
        for &level in &levels {
            let at_level = time(kernel, Variant::Level(level), &inputs, proto)?;
            rows.push(KernelRow {
                kernel: kernel.name,
                level: level.name(),
                level_p50_ns: at_level.p50,
                twin_p50_ns: twin.p50,
            });
        }
    }
    Ok(rows)
}

fn to_json(args: &MicroArgs, kernels: &[KernelRow], floats: &[FloatStatsRow]) -> String {
    let mut out = String::from("{\"provenance\":");
    report::push_provenance(&mut out, &report::provenance(Path::new(".")));
    let _ = write!(
        out,
        ",\"seed\":{},\"elements\":{},\"float_rows\":{},\"samples\":{},\"kernels\":[",
        args.seed, args.elements, args.float_rows, args.samples
    );
    for (index, row) in kernels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"kernel\":");
        json::push_str_lit(&mut out, row.kernel);
        out.push_str(",\"level\":");
        json::push_str_lit(&mut out, row.level);
        let _ = write!(
            out,
            ",\"level_p50_ns\":{},\"twin_p50_ns\":{},\"speedup\":{:.3}}}",
            row.level_p50_ns,
            row.twin_p50_ns,
            row.speedup()
        );
    }
    out.push_str("],\"float_stats\":[");
    for (index, row) in floats.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"family\":");
        json::push_str_lit(&mut out, row.family);
        out.push_str(",\"about\":");
        json::push_str_lit(&mut out, row.about);
        let _ = write!(out, ",\"answers\":{},\"ours\":", row.answers);
        harness::lanes::push_stats(&mut out, &row.ours);
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn to_markdown(args: &MicroArgs, kernels: &[KernelRow], floats: &[FloatStatsRow]) -> String {
    let mut out = format!(
        "# micro — {} elements, {} samples, seed {}\n\n\
         | kernel | level | level p50 ns | twin p50 ns | speedup |\n|---|---|---:|---:|---:|\n",
        args.elements, args.samples, args.seed
    );
    for row in kernels {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {:.2}x |",
            row.kernel,
            row.level,
            row.level_p50_ns,
            row.twin_p50_ns,
            row.speedup()
        );
    }
    let _ = write!(
        out,
        "\n## float_stats — {} rows, checked against the naive evaluator\n\n\
         | family | answers | p50 ns | p99 ns | about |\n|---|---:|---:|---:|---|\n",
        args.float_rows
    );
    for row in floats {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            row.family, row.answers, row.ours.p50, row.ours.p99, row.about
        );
    }
    out
}

/// The `micro` command: writes `micro.json` (at `--out`) and `micro.md`.
/// # Errors
pub fn run(args: &MicroArgs) -> Result<i32, String> {
    let json_path = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out")
            .join(format!(
                "{}-micro",
                report::timestamp_iso8601().replace(':', "-")
            ))
            .join("micro.json")
    });
    if let Some(parent) = json_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("output {}: {e}", parent.display()))?;
    }
    let kernels = kernel_rows(args)?;
    let scratch = args.dir.join("micro-float-stats");
    let _ = std::fs::remove_dir_all(&scratch);
    let floats = float_stats::run(
        &scratch,
        args.seed,
        args.float_rows,
        Protocol {
            warmups: args.samples.div_ceil(8),
            samples: args.samples,
        },
    )?;
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::write(&json_path, to_json(args, &kernels, &floats))
        .map_err(|e| format!("artifact {}: {e}", json_path.display()))?;
    let markdown = to_markdown(args, &kernels, &floats);
    std::fs::write(json_path.with_extension("md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", json_path.display());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny(dir: &std::path::Path) -> MicroArgs {
        MicroArgs {
            elements: 1_000,
            float_rows: 300,
            samples: 1,
            dir: dir.to_path_buf(),
            out: Some(dir.join("out").join("micro.json")),
            ..MicroArgs::default()
        }
    }

    #[test]
    fn every_kernel_matches_its_twin_at_every_level() {
        let dir = crate::fixture::TempDir::new("micro-kernels");
        let rows = kernel_rows(&tiny(dir.path())).expect("kernels agree with their twins");
        assert_eq!(rows.len(), KERNELS.len() * SimdLevel::available().len());
    }

    #[test]
    fn an_unavailable_level_is_refused_by_name() {
        let dir = crate::fixture::TempDir::new("micro-levels");
        let args = MicroArgs {
            levels: Levels::Named(vec!["mmx".to_owned()]),
            ..tiny(dir.path())
        };
        let err = kernel_rows(&args).expect_err("mmx is not a level");
        assert!(err.contains("`mmx`"), "{err}");
    }

    #[test]
    fn the_report_names_every_kernel_level_and_float_family() {
        let dir = crate::fixture::TempDir::new("micro-report");
        let args = tiny(dir.path());
        assert_eq!(run(&args).expect("micro runs"), 0);
        let out = args.out.expect("an output path");
        let parsed = json::parse(&std::fs::read_to_string(&out).expect("micro.json"))
            .expect("micro.json parses");
        let kernels = parsed
            .get("kernels")
            .and_then(json::Value::as_arr)
            .expect("kernel rows");
        assert_eq!(kernels.len(), KERNELS.len() * SimdLevel::available().len());
        let floats = parsed
            .get("float_stats")
            .and_then(json::Value::as_arr)
            .expect("float rows");
        assert_eq!(floats.len(), float_stats::families().len());
        assert!(out.with_extension("md").exists());
    }
}
