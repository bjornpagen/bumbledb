//! The chain sidecar: `dir/chain.json`, the one local file carrying
//! protocol state. It is a floor cache, never a truth the store must
//! reconcile against — the wholeness identity in `crate::replica` is the
//! only check it ever faces, and a torn sidecar costs a re-pull, not a
//! recovery procedure. Canonical one-line JSON, field order fixed,
//! written atomically (exclusive temp, fsync, rename, fsync parent).
//! `pending` is writer-only state; on a pure replica it is permanently
//! null.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;

use crate::braids::{BraidId, Braids};
use crate::manifest::{DOC_VERSION, Text, hex32};

/// The sidecar's file name inside a replica directory.
pub const CHAIN_FILE: &str = "chain.json";

/// One braid's chain position: the applied count, the blake3 of the
/// head log object (what the next batch's header must cite), and the
/// head timestamp (what the next batch's header must dominate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainEntry {
    pub g: u64,
    pub prev: [u8; 32],
    pub ts: u64,
}

impl ChainEntry {
    /// Braid genesis: count zero, the zero hash, timestamp zero.
    pub const GENESIS: Self = Self {
        g: 0,
        prev: [0u8; 32],
        ts: 0,
    };
}

/// The writer's one recovery slot: the batch it fsynced before applying
/// locally, so an interrupted commit resolves at open (60 owns the
/// resolution; this module only carries the bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub braid: BraidId,
    pub slot: u64,
    pub bytes: Vec<u8>,
}

/// Why sidecar bytes refused to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarError {
    Malformed {
        at: usize,
    },
    Version {
        got: u64,
    },
    /// A braid id the schema's own decomposition does not mint.
    UnknownBraid {
        got: u32,
    },
}

/// The parsed sidecar: per-braid chain positions and the pending slot.
/// Braids absent from the map sit at genesis; the canonical rendering
/// materializes every braid explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    pub entries: BTreeMap<BraidId, ChainEntry>,
    pub pending: Option<Pending>,
}

impl Chain {
    /// Every braid of the decomposition at genesis — the bootstrap
    /// chain: zero vector, zero-hash heads, zero timestamps.
    #[must_use]
    pub fn genesis(braids: &Braids) -> Self {
        Self {
            entries: braids
                .components()
                .keys()
                .map(|braid| (*braid, ChainEntry::GENESIS))
                .collect(),
            pending: None,
        }
    }

    /// The braid's position; absent entries are genesis.
    #[must_use]
    pub fn position(&self, braid: BraidId) -> ChainEntry {
        self.entries
            .get(&braid)
            .copied()
            .unwrap_or(ChainEntry::GENESIS)
    }

    /// The vector sum — the engine-generation side of the wholeness
    /// identity.
    #[must_use]
    pub fn sum(&self) -> u64 {
        self.entries.values().map(|entry| entry.g).sum()
    }

    /// The vector: applied counts keyed by braid.
    #[must_use]
    pub fn vector(&self) -> BTreeMap<BraidId, u64> {
        self.entries
            .iter()
            .map(|(braid, entry)| (*braid, entry.g))
            .collect()
    }

    /// The canonical single-line rendering.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut chain = String::new();
        for (index, (braid, entry)) in self.entries.iter().enumerate() {
            if index > 0 {
                chain.push(',');
            }
            write!(
                chain,
                "\"{braid}\":{{\"g\":{},\"prev\":\"{}\",\"ts\":{}}}",
                entry.g,
                hex32(&entry.prev),
                entry.ts
            )
            .expect("write to string");
        }
        let pending = match &self.pending {
            Some(pending) => {
                let mut body = format!(
                    "{{\"braid\":\"{}\",\"gen\":{},\"bytes\":\"",
                    pending.braid, pending.slot
                );
                for byte in &pending.bytes {
                    write!(body, "{byte:02x}").expect("write to string");
                }
                body.push_str("\"}");
                body
            }
            None => "null".to_string(),
        };
        format!("{{\"v\":{DOC_VERSION},\"chain\":{{{chain}}},\"pending\":{pending}}}").into_bytes()
    }

    /// Strict parse. Braid ids must be ones the decomposition mints;
    /// braids the file omits sit at genesis.
    pub fn parse(bytes: &[u8], braids: &Braids) -> Result<Self, SidecarError> {
        let mal = |at| SidecarError::Malformed { at };
        let mut text = Text::new(bytes);
        text.lit("{\"v\":").map_err(mal)?;
        let version = text.u64().map_err(mal)?;
        if version != DOC_VERSION {
            return Err(SidecarError::Version { got: version });
        }
        text.lit(",\"chain\":{").map_err(mal)?;
        let mut entries: BTreeMap<BraidId, ChainEntry> = BTreeMap::new();
        let mut first = true;
        while !text.peek("}") {
            if !first {
                text.lit(",").map_err(mal)?;
            }
            first = false;
            text.lit("\"c").map_err(mal)?;
            let raw = text.hex_u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(SidecarError::UnknownBraid { got: raw });
            };
            text.lit("\":{\"g\":").map_err(mal)?;
            let g = text.u64().map_err(mal)?;
            text.lit(",\"prev\":\"").map_err(mal)?;
            let prev = text.hex32().map_err(mal)?;
            text.lit("\",\"ts\":").map_err(mal)?;
            let ts = text.u64().map_err(mal)?;
            text.lit("}").map_err(mal)?;
            if entries.insert(braid, ChainEntry { g, prev, ts }).is_some() {
                return Err(mal(text.at()));
            }
        }
        text.lit("},\"pending\":").map_err(mal)?;
        let pending = if text.peek("null") {
            text.lit("null").map_err(mal)?;
            None
        } else {
            text.lit("{\"braid\":\"c").map_err(mal)?;
            let raw = text.hex_u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(SidecarError::UnknownBraid { got: raw });
            };
            text.lit("\",\"gen\":").map_err(mal)?;
            let slot = text.u64().map_err(mal)?;
            text.lit(",\"bytes\":\"").map_err(mal)?;
            let body = text.hex_bytes().map_err(mal)?;
            text.lit("\"}").map_err(mal)?;
            Some(Pending {
                braid,
                slot,
                bytes: body,
            })
        };
        text.lit("}").map_err(mal)?;
        text.end().map_err(mal)?;
        Ok(Self { entries, pending })
    }

    /// Atomic publication: exclusive temp beside the target, fsync,
    /// rename over `chain.json`, fsync the directory — so a crash leaves
    /// either the old sidecar or the new one, never a torn line.
    pub fn write_atomic(&self, dir: &Path) -> io::Result<()> {
        let target = dir.join(CHAIN_FILE);
        let temp = dir.join(format!(".{CHAIN_FILE}.tmp.{}", std::process::id()));
        let _ = fs::remove_file(&temp);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let written = file
            .write_all(&self.render())
            .and_then(|()| file.sync_all());
        drop(file);
        if let Err(err) = written {
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        if let Err(err) = fs::rename(&temp, &target) {
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        File::open(dir)?.sync_all()
    }

    /// Reads `dir/chain.json`. `Ok(None)` when the file does not exist;
    /// parse refusals surface so the caller can apply the disposable law.
    pub fn read(dir: &Path, braids: &Braids) -> io::Result<Option<Result<Self, SidecarError>>> {
        match fs::read(dir.join(CHAIN_FILE)) {
            Ok(bytes) => Ok(Some(Self::parse(&bytes, braids))),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}
