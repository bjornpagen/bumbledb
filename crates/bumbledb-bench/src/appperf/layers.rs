//! Native / bridge / Effect / whole-app cost separation (chapters 35/40 §7;
//! RUN-*/API-* shared evidence; the chapter 40 "matched direct-native versus
//! Effect versus full application" diagnostic).
//!
//! The Rust side of the split is measured in-process by [`super::runner`].
//! The Node-side layers are emitted by the runtime/SDK harnesses (P06/P07/P13
//! implement the emitters per chapter 40's routing) as JSON in the exact
//! shape parsed here — this file owns the format, the parser and the
//! summariser, so both sides of the seam agree on one schema. The diagnostic
//! is not a second public SDK: emitters live in test/bench code only.
//!
//! Sample schema (one JSON object per measured operation):
//!
//! ```json
//! {
//!   "layer": "native|bridge|effect|app",
//!   "op": "warm-read",
//!   "ns": 12345,
//!   "queue_ns": 100,
//!   "conv_ns": 200,
//!   "event_loop_delay_ns": 0,
//!   "bytes_copied": 4096,
//!   "gc_count": 0,
//!   "external_bytes": 0
//! }
//! ```
//!
//! `ns` is the end-to-end critical path for the op at that layer. Optional
//! fields are omitted when uninstrumented — never emitted as zero. Layer cost
//! attribution subtracts *distributions* (p50 native vs p50 bridge), never
//! per-sample pairs, because samples at different layers are not the same
//! execution.

use std::fmt::Write as _;

use crate::json::{self, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Direct Rust call, no Node.
    Native,
    /// Node addon call without Effect (private bridge harness).
    Bridge,
    /// Public Effect API on a warmed runtime.
    Effect,
    /// The complete application path (request handler, tenant binding, log).
    App,
}

pub const LAYERS: [Layer; 4] = [Layer::Native, Layer::Bridge, Layer::Effect, Layer::App];

impl Layer {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Bridge => "bridge",
            Self::Effect => "effect",
            Self::App => "app",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "native" => Some(Self::Native),
            "bridge" => Some(Self::Bridge),
            "effect" => Some(Self::Effect),
            "app" => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSample {
    pub layer: Layer,
    pub op: String,
    pub ns: u64,
    pub queue_ns: Option<u64>,
    pub conv_ns: Option<u64>,
    pub event_loop_delay_ns: Option<u64>,
    pub bytes_copied: Option<u64>,
    pub gc_count: Option<u64>,
    pub external_bytes: Option<u64>,
}

fn opt_u64(value: Option<&Value>, field: &str) -> Result<Option<u64>, String> {
    match value {
        None => Ok(None),
        Some(v) => {
            let number = v
                .as_f64()
                .ok_or_else(|| format!("`{field}` must be a number"))?;
            if number.is_nan() || number < 0.0 {
                return Err(format!("`{field}` must be nonnegative, got {number}"));
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "nonnegative checked; sample counters fit u64 by construction"
            )]
            Ok(Some(number as u64))
        }
    }
}

/// Parse one emitted sample object.
///
/// # Errors
pub fn parse_sample(value: &Value) -> Result<LayerSample, String> {
    let layer_raw = value
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| "sample missing `layer`".to_owned())?;
    let layer = Layer::parse(layer_raw)
        .ok_or_else(|| format!("unknown layer `{layer_raw}` (native|bridge|effect|app)"))?;
    let op = value
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| "sample missing `op`".to_owned())?
        .to_owned();
    let ns = opt_u64(value.get("ns"), "ns")?.ok_or_else(|| "sample missing `ns`".to_owned())?;
    Ok(LayerSample {
        layer,
        op,
        ns,
        queue_ns: opt_u64(value.get("queue_ns"), "queue_ns")?,
        conv_ns: opt_u64(value.get("conv_ns"), "conv_ns")?,
        event_loop_delay_ns: opt_u64(value.get("event_loop_delay_ns"), "event_loop_delay_ns")?,
        bytes_copied: opt_u64(value.get("bytes_copied"), "bytes_copied")?,
        gc_count: opt_u64(value.get("gc_count"), "gc_count")?,
        external_bytes: opt_u64(value.get("external_bytes"), "external_bytes")?,
    })
}

/// Parse a whole emitted file: `{"samples":[...]}`.
///
/// # Errors
pub fn parse_file(text: &str) -> Result<Vec<LayerSample>, String> {
    let parsed = json::parse(text).map_err(|e| format!("layer sample JSON: {e}"))?;
    let samples = parsed
        .get("samples")
        .and_then(Value::as_arr)
        .ok_or_else(|| "no `samples` array".to_owned())?;
    samples.iter().map(parse_sample).collect()
}

/// Serialize samples in the pinned shape (the Rust-side emitters use this so
/// both languages produce byte-compatible files for the merger).
#[must_use]
pub fn to_json(samples: &[LayerSample]) -> String {
    let mut out = String::from("{\"samples\":[");
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{{\"layer\":\"{}\",\"op\":", sample.layer.label());
        json::push_str_lit(&mut out, &sample.op);
        let _ = write!(out, ",\"ns\":{}", sample.ns);
        for (field, value) in [
            ("queue_ns", sample.queue_ns),
            ("conv_ns", sample.conv_ns),
            ("event_loop_delay_ns", sample.event_loop_delay_ns),
            ("bytes_copied", sample.bytes_copied),
            ("gc_count", sample.gc_count),
            ("external_bytes", sample.external_bytes),
        ] {
            if let Some(value) = value {
                let _ = write!(out, ",\"{field}\":{value}");
            }
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

/// Per-(op, layer) latency summary. Attribution reads *between* rows:
/// `bridge.p50 - native.p50` is the bridge toll's central tendency; the
/// summariser never pairs individual samples across layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSummary {
    pub op: String,
    pub layer: Layer,
    pub samples: u64,
    pub p50_ns: u64,
    pub p99_ns: u64,
    pub max_event_loop_delay_ns: Option<u64>,
    pub total_bytes_copied: Option<u64>,
}

/// Summarise samples grouped by (op, layer). An op present at a higher layer
/// but missing at a lower one is reported as a coverage hole by
/// [`coverage_holes`], because a whole-app number without its native
/// baseline attributes nothing.
#[must_use]
pub fn summarize(samples: &[LayerSample]) -> Vec<LayerSummary> {
    /// One in-flight group: op name, layer, samples, max event-loop delay,
    /// max RSS delta.
    type Group = (String, Layer, Vec<u64>, Option<u64>, Option<u64>);
    let mut groups: Vec<Group> = Vec::new();
    for sample in samples {
        let found = groups
            .iter()
            .position(|(op, layer, ..)| *op == sample.op && *layer == sample.layer);
        let index = if let Some(index) = found {
            index
        } else {
            groups.push((sample.op.clone(), sample.layer, Vec::new(), None, None));
            groups.len() - 1
        };
        let group = &mut groups[index];
        group.2.push(sample.ns);
        if let Some(delay) = sample.event_loop_delay_ns {
            group.3 = Some(group.3.unwrap_or(0).max(delay));
        }
        if let Some(bytes) = sample.bytes_copied {
            group.4 = Some(group.4.unwrap_or(0) + bytes);
        }
    }
    groups
        .into_iter()
        .map(|(op, layer, mut ns, delay, bytes)| {
            ns.sort_unstable();
            let idx = |q: f64| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss,
                    reason = "quantile index arithmetic over small sample counts"
                )]
                let index = ((ns.len() as f64 - 1.0) * q).round() as usize;
                ns[index.min(ns.len() - 1)]
            };
            LayerSummary {
                op,
                layer,
                samples: ns.len() as u64,
                p50_ns: idx(0.50),
                p99_ns: idx(0.99),
                max_event_loop_delay_ns: delay,
                total_bytes_copied: bytes,
            }
        })
        .collect()
}

/// Ops that lack a complete native→bridge→effect layer chain (`app` is
/// optional per op; the other three are the decomposition contract).
#[must_use]
pub fn coverage_holes(summaries: &[LayerSummary]) -> Vec<String> {
    let mut ops: Vec<&str> = summaries.iter().map(|s| s.op.as_str()).collect();
    ops.sort_unstable();
    ops.dedup();
    let mut holes = Vec::new();
    for op in ops {
        for layer in [Layer::Native, Layer::Bridge, Layer::Effect] {
            if !summaries.iter().any(|s| s.op == op && s.layer == layer) {
                holes.push(format!("{op}: missing {} layer", layer.label()));
            }
        }
    }
    holes
}
