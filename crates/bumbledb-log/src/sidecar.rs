//! The chain sidecar: `dir/chain`, the one local file carrying
//! protocol state. It is a floor cache, never a truth the store must
//! reconcile against — the wholeness identity in `crate::replica` is the
//! only check it ever faces, and a torn sidecar costs a re-pull, not a
//! recovery procedure. The document is a binary record of the batch
//! codec's primitives, written atomically (exclusive temp, fsync,
//! rename, fsync parent). The content address is `blake3` of those
//! bytes.
//!
//! Layout, little-endian:
//! `u8` version (3); `u32` entry count; each entry is `u32` braid,
//! `u64` g, `[u8; 32]` prev, `u64` ts; `u8` arm (`0` Settled, `1`
//! Pending). Pending then carries `u32` braid, `u64` gen, `u32` length,
//! and that many raw batch bytes.
//!
//! The chain is a sum — `Settled` or `Pending` — and `generation()` is
//! a total function of the value: the vector sum, plus one exactly when
//! the arm is `Pending`. `Pending` is writer-only; on a pure replica the
//! chain is permanently `Settled`. The read is a total sum too: `Absent`
//! is `NotFound` only, an infra fault is never absence, and a parse
//! refusal is `Corrupt`.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;

use crate::braids::{BraidId, Braids};
use crate::manifest::{Cursor, DOC_VERSION};
use crate::vector::Vector;

/// The sidecar's file name inside a replica directory.
pub const CHAIN_FILE: &str = "chain";

const ARM_SETTLED: u8 = 0;
const ARM_PENDING: u8 = 1;
const ENTRY_BYTES: usize = 4 + 8 + 32 + 8;

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
/// locally, so an interrupted commit resolves at open. This module
/// carries the bytes.
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
    /// The vector sum of the `g` column overflows `u64`.
    Overflow,
}

impl SidecarError {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "Malformed",
            Self::Version { .. } => "Version",
            Self::UnknownBraid { .. } => "UnknownBraid",
            Self::Overflow => "Overflow",
        }
    }
}

/// The sidecar read as a total sum. `Absent` is `NotFound` and nothing
/// else — an infra fault is never "no sidecar"; a parse refusal is
/// `Corrupt` and takes the disposable-law discard.
#[derive(Debug)]
pub enum SidecarRead {
    Absent,
    Fault(io::Error),
    Corrupt(SidecarError),
    Read(Chain),
}

impl SidecarRead {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Absent => "Absent",
            Self::Fault(_) => "Fault",
            Self::Corrupt(_) => "Corrupt",
            Self::Read(_) => "Read",
        }
    }
}

/// The parsed sidecar: a settled vector, or that vector plus the one
/// pending batch. Braids absent from the map sit at genesis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chain {
    Settled {
        entries: BTreeMap<BraidId, ChainEntry>,
    },
    Pending {
        entries: BTreeMap<BraidId, ChainEntry>,
        batch: Pending,
    },
}

impl Chain {
    /// Every braid of the decomposition at genesis — the bootstrap
    /// chain: zero vector, zero-hash heads, zero timestamps.
    #[must_use]
    pub fn genesis(braids: &Braids) -> Self {
        Self::Settled {
            entries: braids
                .components()
                .keys()
                .map(|braid| (*braid, ChainEntry::GENESIS))
                .collect(),
        }
    }

    /// The per-braid positions. Absent keys are genesis.
    #[must_use]
    pub fn entries(&self) -> &BTreeMap<BraidId, ChainEntry> {
        match self {
            Self::Settled { entries } | Self::Pending { entries, .. } => entries,
        }
    }

    pub fn entries_mut(&mut self) -> &mut BTreeMap<BraidId, ChainEntry> {
        match self {
            Self::Settled { entries } | Self::Pending { entries, .. } => entries,
        }
    }

    /// The braid's position; absent entries are genesis.
    #[must_use]
    pub fn position(&self, braid: BraidId) -> ChainEntry {
        self.entries()
            .get(&braid)
            .copied()
            .unwrap_or(ChainEntry::GENESIS)
    }

    /// The vector sum — the settled side of `generation`. Overflow
    /// saturates so generation stays total; the refusal lives in
    /// [`Vector::sum`].
    #[must_use]
    pub fn sum(&self) -> u64 {
        self.vector().sum().unwrap_or(u64::MAX)
    }

    /// The generation a store must show: the vector sum, plus one
    /// exactly when the chain is `Pending`. There is no addend a
    /// reader can forget.
    #[must_use]
    pub fn generation(&self) -> u64 {
        match self {
            Self::Settled { .. } => self.sum(),
            Self::Pending { .. } => self.sum().saturating_add(1),
        }
    }

    /// The vector: applied counts keyed by braid.
    #[must_use]
    pub fn vector(&self) -> Vector {
        self.entries()
            .iter()
            .map(|(braid, entry)| (*braid, entry.g))
            .collect()
    }

    /// The one rendering: version byte 3, counted entries in braid
    /// order, then the constructor arm. `Pending` carries its batch as
    /// raw bytes.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let entries = self.entries();
        let count = u32::try_from(entries.len()).expect("entry count fits u32");
        let mut out = Vec::new();
        out.push(DOC_VERSION);
        out.extend_from_slice(&count.to_le_bytes());
        for (braid, entry) in entries {
            out.extend_from_slice(&braid.raw().to_le_bytes());
            out.extend_from_slice(&entry.g.to_le_bytes());
            out.extend_from_slice(&entry.prev);
            out.extend_from_slice(&entry.ts.to_le_bytes());
        }
        match self {
            Self::Settled { .. } => out.push(ARM_SETTLED),
            Self::Pending { batch, .. } => {
                let len = u32::try_from(batch.bytes.len()).expect("pending fits u32");
                out.push(ARM_PENDING);
                out.extend_from_slice(&batch.braid.raw().to_le_bytes());
                out.extend_from_slice(&batch.slot.to_le_bytes());
                out.extend_from_slice(&len.to_le_bytes());
                out.extend_from_slice(&batch.bytes);
            }
        }
        out
    }

    /// Strict parse: braid ids are ones the decomposition mints,
    /// entries ascend in braid order (a duplicate has no place to
    /// stand), and an accepted document re-renders byte-identically.
    /// Braids the file omits sit at genesis. A leading byte other than
    /// 3 is a version refusal. The `g`/`ts`/`gen` columns are `u64le`;
    /// pending bytes are a length-delimited payload.
    pub fn parse(bytes: &[u8], braids: &Braids) -> Result<Self, SidecarError> {
        let mal = |at| SidecarError::Malformed { at };
        let mut cur = Cursor::new(bytes);
        let version = cur.u8().map_err(mal)?;
        if version != DOC_VERSION {
            return Err(SidecarError::Version {
                got: u64::from(version),
            });
        }
        let count = cur.u32().map_err(mal)?;
        if !bytes_back(count, cur.remaining(), ENTRY_BYTES) {
            return Err(mal(cur.at()));
        }
        let mut entries: BTreeMap<BraidId, ChainEntry> = BTreeMap::new();
        for _ in 0..count {
            let raw = cur.u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(SidecarError::UnknownBraid { got: raw });
            };
            let g = cur.u64().map_err(mal)?;
            let prev = cur.array32().map_err(mal)?;
            let ts = cur.u64().map_err(mal)?;
            if entries
                .last_key_value()
                .is_some_and(|(last, _)| *last >= braid)
            {
                return Err(mal(cur.at()));
            }
            entries.insert(braid, ChainEntry { g, prev, ts });
        }
        if entries
            .iter()
            .map(|(&braid, entry)| (braid, entry.g))
            .collect::<Vector>()
            .sum()
            .is_err()
        {
            return Err(SidecarError::Overflow);
        }
        let arm_at = cur.at();
        let chain = match cur.u8().map_err(mal)? {
            ARM_SETTLED => Self::Settled { entries },
            ARM_PENDING => {
                let raw = cur.u32().map_err(mal)?;
                let Some(braid) = braids.parse(raw) else {
                    return Err(SidecarError::UnknownBraid { got: raw });
                };
                let slot = cur.u64().map_err(mal)?;
                let len = cur.u32().map_err(mal)?;
                let n = usize::try_from(len).map_err(|_| mal(cur.at()))?;
                let body = cur.take(n).map_err(mal)?;
                Self::Pending {
                    entries,
                    batch: Pending {
                        braid,
                        slot,
                        bytes: body.to_vec(),
                    },
                }
            }
            _ => return Err(mal(arm_at)),
        };
        cur.end().map_err(mal)?;
        Ok(chain)
    }

    /// Atomic publication: exclusive temp beside the target, fsync,
    /// rename over `chain`, fsync the directory — so a crash leaves
    /// either the incumbent sidecar or the successor, never a torn
    /// record.
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

    /// Reads `dir/chain` as a total sum. `Absent` is `NotFound` only;
    /// every other io error is `Fault`; a parse refusal is `Corrupt`.
    #[must_use]
    pub fn read(dir: &Path, braids: &Braids) -> SidecarRead {
        match fs::read(dir.join(CHAIN_FILE)) {
            Ok(bytes) => match Self::parse(&bytes, braids) {
                Ok(chain) => SidecarRead::Read(chain),
                Err(error) => SidecarRead::Corrupt(error),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => SidecarRead::Absent,
            Err(err) => SidecarRead::Fault(err),
        }
    }
}

fn bytes_back(count: u32, remaining: usize, min_item: usize) -> bool {
    let declared = usize::try_from(count).expect("u32 fits usize");
    if declared == 0 {
        return true;
    }
    if min_item == 0 {
        return false;
    }
    remaining / min_item >= declared
}

#[cfg(test)]
mod tests {
    use super::{ARM_PENDING, ARM_SETTLED, CHAIN_FILE, Chain, Pending, SidecarError, SidecarRead};
    use crate::braids::braids;
    use crate::manifest::DOC_VERSION;
    use bumbledb::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
        StatementDescriptor, ValueType,
    };

    fn kitchen_braids() -> crate::braids::Braids {
        let field = |name: &str, value_type: ValueType| FieldDescriptor {
            name: name.into(),
            value_type,
            generation: Generation::None,
        };
        let descriptor = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "sample".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::from([FieldId(0)]),
            }],
        };
        braids(&descriptor)
    }

    #[test]
    fn genesis_renders_version_byte_and_settled_arm() {
        let chain = Chain::genesis(&kitchen_braids());
        let bytes = chain.render();
        assert_eq!(bytes[0], DOC_VERSION);
        assert_eq!(&bytes[1..5], 1u32.to_le_bytes());
        assert_eq!(&bytes[5..9], 0u32.to_le_bytes());
        assert_eq!(&bytes[9..17], 0u64.to_le_bytes());
        assert_eq!(&bytes[17..49], &[0u8; 32]);
        assert_eq!(&bytes[49..57], 0u64.to_le_bytes());
        assert_eq!(bytes[57], ARM_SETTLED);
        assert_eq!(bytes.len(), 58);
        assert!(matches!(chain, Chain::Settled { .. }));
        assert_eq!(chain.generation(), 0);
        assert_eq!(Chain::parse(&bytes, &kitchen_braids()), Ok(chain));
    }

    #[test]
    fn parse_refuses_a_leading_byte_other_than_3() {
        let braids = kitchen_braids();
        assert_eq!(
            Chain::parse(&[2], &braids),
            Err(SidecarError::Version { got: 2 })
        );
        assert_eq!(
            Chain::parse(br#"{"v":3}"#, &braids)
                .expect_err("text")
                .identity(),
            "Version"
        );
        let mut trailing = Chain::genesis(&braids).render();
        trailing.push(0);
        assert_eq!(
            Chain::parse(&trailing, &braids)
                .expect_err("trailing")
                .identity(),
            "Malformed"
        );
    }

    #[test]
    fn pending_bytes_are_a_length_delimited_payload() {
        let braids = kitchen_braids();
        let Chain::Settled { entries } = Chain::genesis(&braids) else {
            panic!("genesis is Settled");
        };
        let chain = Chain::Pending {
            entries,
            batch: Pending {
                braid: braids.parse(0).expect("kitchen braid"),
                slot: 1,
                bytes: vec![0x42, 0x44, 0x42, 0x4c],
            },
        };
        let bytes = chain.render();
        let arm_at = 57;
        assert_eq!(bytes[arm_at], ARM_PENDING);
        assert_eq!(&bytes[arm_at + 1..arm_at + 5], 0u32.to_le_bytes());
        assert_eq!(&bytes[arm_at + 5..arm_at + 13], 1u64.to_le_bytes());
        assert_eq!(&bytes[arm_at + 13..arm_at + 17], 4u32.to_le_bytes());
        assert_eq!(&bytes[arm_at + 17..], [0x42, 0x44, 0x42, 0x4c]);
        let parsed = Chain::parse(&bytes, &braids).expect("pending");
        let Chain::Pending { batch, .. } = parsed else {
            panic!("pending constructor");
        };
        assert_eq!(batch.bytes, [0x42, 0x44, 0x42, 0x4c]);
        assert_eq!(batch.slot, 1);
    }

    #[test]
    fn generation_is_the_sum_plus_one_exactly_when_pending() {
        let braids = kitchen_braids();
        let mut settled = Chain::genesis(&braids);
        settled.entries_mut().values_mut().next().expect("entry").g = 40;
        assert!(matches!(settled, Chain::Settled { .. }));
        assert_eq!(settled.sum(), 40);
        assert_eq!(settled.generation(), 40);
        assert_eq!(
            Chain::parse(&settled.render(), &braids).expect("settled"),
            settled
        );

        let Chain::Settled { entries } = settled else {
            panic!("settled");
        };
        let pending = Chain::Pending {
            entries,
            batch: Pending {
                braid: braids.parse(0).expect("kitchen braid"),
                slot: 41,
                bytes: vec![0x42, 0x44, 0x42, 0x4c],
            },
        };
        assert_eq!(pending.sum(), 40);
        assert_eq!(pending.generation(), 41);
        assert_eq!(
            Chain::parse(&pending.render(), &braids).expect("pending"),
            pending
        );
    }

    #[test]
    fn read_is_absent_corrupt_or_read() {
        let braids = kitchen_braids();
        let dir = std::env::temp_dir().join(format!(
            "bdb-log-sidecar-read-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch");
        assert_eq!(Chain::read(&dir, &braids).identity(), "Absent");
        std::fs::write(dir.join(CHAIN_FILE), [2]).expect("write version 2");
        let SidecarRead::Corrupt(SidecarError::Version { got: 2 }) = Chain::read(&dir, &braids)
        else {
            panic!("version 2 is Corrupt, not Absent");
        };
        let chain = Chain::genesis(&braids);
        chain.write_atomic(&dir).expect("write settled");
        match Chain::read(&dir, &braids) {
            SidecarRead::Read(got) => {
                assert_eq!(got, chain);
                assert_eq!(got.generation(), 0);
            }
            other => panic!("expected Read, got {}", other.identity()),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
