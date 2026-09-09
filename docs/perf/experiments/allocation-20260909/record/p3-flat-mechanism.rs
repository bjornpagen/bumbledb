use std::hint::black_box;
use std::io::Read;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb-bench/src/boost.rs"]
mod boost;
#[allow(dead_code)]
#[path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb-bench/src/clockproxy.rs"]
mod clock;
#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/p3-flat-kernels-v3.rs"]
mod kernels;

type Kernel = fn(&[u64], &[u64], &[u32], u64, u64, &mut Vec<u32>);
struct Group {
    starts: Vec<u64>,
    ends: Vec<u64>,
    positions: Vec<u32>,
}
struct Sample {
    group: usize,
    lo: u64,
    hi: u64,
    hits: usize,
}
struct Input {
    groups: Vec<Group>,
    samples: Vec<Sample>,
}

fn word32(file: &mut impl Read) -> u32 {
    let mut bytes = [0; 4];
    file.read_exact(&mut bytes).unwrap();
    u32::from_le_bytes(bytes)
}
fn word64(file: &mut impl Read) -> u64 {
    let mut bytes = [0; 8];
    file.read_exact(&mut bytes).unwrap();
    u64::from_le_bytes(bytes)
}
fn load(path: &std::path::Path) -> Input {
    let mut file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
    let mut magic = [0; 8];
    file.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"BFLAT001");
    let group_count = word32(&mut file) as usize;
    assert_eq!(group_count, 1999);
    let mut groups = Vec::with_capacity(group_count);
    let mut by_dir = std::collections::BTreeMap::new();
    for index in 0..group_count {
        let dir = word32(&mut file);
        let len = word32(&mut file) as usize;
        assert!(len <= 128);
        let mut group = Group {
            starts: Vec::with_capacity(len),
            ends: Vec::with_capacity(len),
            positions: Vec::with_capacity(len),
        };
        for _ in 0..len {
            group.starts.push(word64(&mut file));
            group.ends.push(word64(&mut file));
            group.positions.push(word32(&mut file));
        }
        assert!(group.starts.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(by_dir.insert(dir, index).is_none());
        groups.push(group);
    }
    let count = word32(&mut file) as usize;
    assert_eq!(count, 144953);
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let dir = word32(&mut file);
        let group = by_dir[&dir];
        let lo = word64(&mut file);
        let hi = word64(&mut file);
        let hits = word32(&mut file) as usize;
        assert!(hits <= groups[group].starts.len());
        samples.push(Sample {
            group,
            lo,
            hi,
            hits,
        });
    }
    let mut last = [0; 1];
    assert_eq!(file.read(&mut last).unwrap(), 0);
    Input { groups, samples }
}

fn verify(input: &Input) {
    let mut expected = Vec::new();
    let mut out = Vec::new();
    for sample in &input.samples {
        let group = &input.groups[sample.group];
        expected.clear();
        expected.extend(
            group
                .starts
                .iter()
                .zip(&group.ends)
                .zip(&group.positions)
                .filter_map(|((&start, &end), &position)| {
                    (start < sample.hi && end > sample.lo).then_some(position)
                }),
        );
        assert_eq!(expected.len(), sample.hits);
        for kernel in [kernels::branchy as Kernel, kernels::block8, kernels::mask8] {
            kernel(
                &group.starts,
                &group.ends,
                &group.positions,
                sample.lo,
                sample.hi,
                &mut out,
            );
            assert_eq!(out, expected);
        }
    }
}

fn synthetic(len: usize, every: usize, lo: u64, hi: u64) -> Input {
    let starts: Vec<_> = (0..len).map(|i| (i / 2) as u64 * 3).collect();
    let ends: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            if every != 0 && i % every == 0 {
                u64::MAX
            } else {
                start + 1
            }
        })
        .collect();
    let positions: Vec<_> = (0..len).map(|i| ((len - i) * 17 + 3) as u32).collect();
    let hits = starts
        .iter()
        .zip(&ends)
        .filter(|(start, end)| **start < hi && **end > lo)
        .count();
    Input {
        groups: vec![Group {
            starts,
            ends,
            positions,
        }],
        samples: vec![Sample {
            group: 0,
            lo,
            hi,
            hits,
        }],
    }
}

fn draw(input: &Input, kernel: Kernel, out: &mut Vec<u32>) -> u64 {
    let mut sum = 0u64;
    for sample in &input.samples {
        let group = &input.groups[sample.group];
        kernel(
            &group.starts,
            &group.ends,
            &group.positions,
            sample.lo,
            sample.hi,
            out,
        );
        sum += black_box(out.len()) as u64;
    }
    black_box(out);
    sum
}

fn measure(name: &str, input: &Input, draws: u64) {
    verify(input);
    let expected: u64 = input.samples.iter().map(|sample| sample.hits as u64).sum();
    let order = [
        ("A0", kernels::branchy as Kernel),
        ("B0", kernels::block8),
        ("C0", kernels::mask8),
        ("C1", kernels::mask8),
        ("B1", kernels::block8),
        ("A1", kernels::branchy),
    ];
    for (label, kernel) in order {
        let kernel = black_box(kernel);
        let mut out = Vec::with_capacity(4096);
        for _ in 0..2 {
            assert_eq!(draw(input, kernel, &mut out), expected);
        }
        clock::warm_up(Duration::from_millis(20));
        let pre = clock::effective_ghz();
        let start = Instant::now();
        let mut sum = 0;
        for _ in 0..draws {
            sum += draw(input, kernel, &mut out);
        }
        let ns = start.elapsed().as_nanos();
        let post = clock::effective_ghz();
        assert_eq!(sum, expected * draws);
        assert_eq!(
            out.capacity(),
            4096,
            "equal warm capacity; no timing allocations"
        );
        println!(
            "MECH {name} {label} ns={ns} draws={draws} calls={} pre={pre:.6} post={post:.6} flagged={} checksum={sum}",
            draws * input.samples.len() as u64,
            pre.min(post) < clock::CONTAMINATION_GHZ
        );
    }
}

fn main() {
    boost::engage_from_env().unwrap();
    let path = std::env::args_os().nth(1).unwrap();
    let actual = load(std::path::Path::new(&path));
    // Independent equality cases include both sides of the flat cutoff;
    // the 129-row kernel check is not a proposed production threshold change.
    for len in [0, 1, 16, 46, 75, 106, 127, 128, 129] {
        for every in [0, 1, 2, 16] {
            for (lo, hi) in [
                (0, 0),
                (0, u64::MAX),
                (8, 8),
                (12, 8),
                (384, u64::MAX),
                (u64::MAX, u64::MAX),
            ] {
                verify(&synthetic(len, every, lo, hi));
            }
        }
    }
    println!("CORRECTNESS actual schedules and 216 synthetic cases per three kernels");
    measure("saved-flat-order", &actual, 8);
    for (name, input) in [
        ("empty", synthetic(0, 0, 0, 0)),
        ("singleton", synthetic(1, 1, 0, u64::MAX)),
        ("early-none", synthetic(128, 1, 0, 0)),
        ("early-two", synthetic(128, 1, 0, 1)),
        ("no-hits", synthetic(128, 0, 384, u64::MAX)),
        ("all-hits", synthetic(128, 1, 384, u64::MAX)),
        ("alternating", synthetic(128, 2, 384, u64::MAX)),
        ("sparse", synthetic(128, 16, 384, u64::MAX)),
        ("reversed", synthetic(128, 16, 384, 17)),
    ] {
        measure(name, &input, 131072);
    }
}
