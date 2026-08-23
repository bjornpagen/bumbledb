//! Decoder robustness as ordinary tests: a seeded deterministic
//! mutation harness over the golden batches. Every mutation must come
//! back as a value — an accepted batch or a typed refusal — and every
//! refusal must carry its cross-implementation identity; a panic
//! anywhere fails the run. Exhaustive truncation runs beside the
//! random byte storm so every sequential-parse boundary is crossed.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb_log::codec::Codec;
use serde_json::Value as Json;

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound.max(1)).expect("bound fits u64"))
            .expect("index fits usize")
    }
}

fn goldens() -> Vec<(String, Codec, Vec<u8>, bool)> {
    let schemas = support::load_schemas();
    let dir = support::corpus_dir().join("batch");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("corpus present") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let sidecar: Json =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("sidecar")).expect("json");
        let schema = sidecar["schema"].as_str().expect("schema").to_owned();
        let fingerprint: [u8; 32] = support::unhex(sidecar["fingerprint"].as_str().expect("hex"))
            .try_into()
            .expect("32 bytes");
        let codec = Codec::new(&schemas[&schema], fingerprint);
        let bytes = std::fs::read(path.with_extension("bin")).expect("bin");
        let ok = sidecar["expect"].as_str() == Some("ok");
        out.push((schema, codec, bytes, ok));
    }
    assert!(!out.is_empty(), "the corpus feeds the harness");
    out
}

#[test]
fn every_truncation_of_an_accepted_batch_refuses() {
    for (name, codec, bytes, ok) in goldens() {
        if !ok {
            continue;
        }
        for len in 0..bytes.len() {
            let outcome = codec.decode(&bytes[..len]);
            let refusal = outcome.expect_err("a proper prefix always refuses");
            assert!(!refusal.identity().is_empty(), "{name}: typed identity");
        }
    }
}

#[test]
fn a_seeded_byte_storm_never_panics_and_always_types_its_refusals() {
    let mut prng = XorShift(0x5eed_cafe_f00d_0001);
    for (name, codec, bytes, _) in goldens() {
        for _ in 0..2_000 {
            let mut mutated = bytes.clone();
            match prng.next() % 4 {
                // Flip up to three bytes.
                0 | 1 => {
                    for _ in 0..=prng.below(3) {
                        if mutated.is_empty() {
                            break;
                        }
                        let at = prng.below(mutated.len());
                        mutated[at] ^= u8::try_from(prng.next() % 255 + 1).expect("byte");
                    }
                }
                // Truncate at a random point.
                2 => {
                    let len = prng.below(mutated.len() + 1);
                    mutated.truncate(len);
                }
                // Append random garbage.
                _ => {
                    for _ in 0..=prng.below(9) {
                        mutated.push(u8::try_from(prng.next() % 256).expect("byte"));
                    }
                }
            }
            match codec.decode(&mutated) {
                Ok(_) => {
                    // A mutation that lands on another valid batch is
                    // fine; the chain discipline at apply is the next
                    // tripwire.
                }
                Err(refusal) => {
                    assert!(!refusal.identity().is_empty(), "{name}: typed identity");
                }
            }
        }
    }
}
