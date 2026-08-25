//! Text rendering of a parsed protocol document. Documents stay the
//! one binary grammar; inspect is a tool, not a wire format.

use std::fmt;
use std::fmt::Write as _;

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::{SchemaDescriptor, SchemaError, Value};

use crate::braids::braids;
use crate::codec::{Batch, Codec, DecodeError, OpKind};
use crate::manifest::{Checkpoint, CheckpointError, Manifest, ManifestError, hex32};
use crate::sidecar::{Chain, SidecarError};

/// Which protocol object a store key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Manifest,
    Checkpoint,
    Sidecar,
    Batch,
}

/// Why inspect refused the key or the bytes.
#[derive(Debug)]
pub enum InspectError {
    Kind,
    Missing,
    Key(String),
    Io(String),
    Manifest(ManifestError),
    Checkpoint(CheckpointError),
    Sidecar(SidecarError),
    Batch(DecodeError),
    Theory(SchemaError),
}

impl fmt::Display for InspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kind => write!(f, "inspect key is not a document"),
            Self::Missing => write!(f, "inspect key is missing"),
            Self::Key(key) => write!(f, "store key is not a slash path: {key}"),
            Self::Io(error) => write!(f, "inspect: {error}"),
            Self::Manifest(error) => write!(f, "inspect refused: manifest {}", error.identity()),
            Self::Checkpoint(error) => {
                write!(f, "inspect refused: checkpoint {}", error.identity())
            }
            Self::Sidecar(error) => write!(f, "inspect refused: sidecar {}", error.identity()),
            Self::Batch(error) => write!(f, "inspect refused: batch {}", error.identity()),
            Self::Theory(error) => write!(f, "inspect refused: theory {error}"),
        }
    }
}

/// The object kind the key spelling names. `.mdb` is a store snapshot,
/// not a protocol document.
#[must_use]
pub fn kind(key: &str) -> Result<Kind, InspectError> {
    let segs: Vec<&str> = key.split('/').filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        [.., "manifest"] => Ok(Kind::Manifest),
        [.., "chain"] => Ok(Kind::Sidecar),
        [.., "ckpt", name] if !name.ends_with(".mdb") && !name.is_empty() => Ok(Kind::Checkpoint),
        [.., "log", braid, slot] if !braid.is_empty() && !slot.is_empty() => Ok(Kind::Batch),
        _ => Err(InspectError::Kind),
    }
}

/// Decode through the crate parsers and render the value as text.
pub fn render(kind: Kind, bytes: &[u8], theory: &SchemaDescriptor) -> Result<String, InspectError> {
    let mut out = String::new();
    writeln!(out, "digest {}", hex32(blake3::hash(bytes).as_bytes())).expect("inspect");
    match kind {
        Kind::Manifest => write_manifest(
            &mut out,
            &Manifest::parse(bytes).map_err(InspectError::Manifest)?,
        ),
        Kind::Checkpoint => write_checkpoint(
            &mut out,
            &Checkpoint::parse(bytes, &braids(theory)).map_err(InspectError::Checkpoint)?,
        ),
        Kind::Sidecar => write_chain(
            &mut out,
            &Chain::parse(bytes, &braids(theory)).map_err(InspectError::Sidecar)?,
        ),
        Kind::Batch => write_batch(
            &mut out,
            &codec(theory)?.decode(bytes).map_err(InspectError::Batch)?,
        ),
    }
    Ok(out)
}

fn codec(theory: &SchemaDescriptor) -> Result<Codec, InspectError> {
    let schema = theory.clone().validate().map_err(InspectError::Theory)?;
    Ok(Codec::new(theory, schema_fingerprint(&schema).0))
}

fn write_manifest(out: &mut String, doc: &Manifest) {
    writeln!(out, "manifest").expect("inspect");
    writeln!(out, "fingerprint {}", hex32(&doc.fingerprint)).expect("inspect");
    match doc.checkpoint {
        Some(digest) => writeln!(out, "checkpoint {}", hex32(&digest)).expect("inspect"),
        None => writeln!(out, "checkpoint none").expect("inspect"),
    }
}

fn write_checkpoint(out: &mut String, doc: &Checkpoint) {
    writeln!(out, "checkpoint").expect("inspect");
    writeln!(out, "catalog {}", hex32(&doc.catalog)).expect("inspect");
    writeln!(out, "writer {}", doc.writer).expect("inspect");
    match doc.prev {
        Some(digest) => writeln!(out, "prev {}", hex32(&digest)).expect("inspect"),
        None => writeln!(out, "prev none").expect("inspect"),
    }
    for (braid, head) in &doc.braids {
        writeln!(
            out,
            "{braid} g {} hash {} ts {}",
            head.g,
            hex32(&head.hash),
            head.ts
        )
        .expect("inspect");
    }
}

fn write_chain(out: &mut String, doc: &Chain) {
    match doc {
        Chain::Settled { .. } => writeln!(out, "sidecar settled").expect("inspect"),
        Chain::Pending { batch, .. } => {
            writeln!(out, "sidecar pending").expect("inspect");
            writeln!(
                out,
                "pending {} {} {}",
                batch.braid,
                batch.slot,
                hex_bytes(&batch.bytes)
            )
            .expect("inspect");
        }
    }
    for (braid, entry) in doc.entries() {
        writeln!(
            out,
            "{braid} g {} prev {} ts {}",
            entry.g,
            hex32(&entry.prev),
            entry.ts
        )
        .expect("inspect");
    }
}

fn write_batch(out: &mut String, doc: &Batch) {
    let header = &doc.header;
    writeln!(out, "batch").expect("inspect");
    writeln!(out, "fingerprint {}", hex32(&header.fingerprint)).expect("inspect");
    writeln!(out, "braid {}", header.braid).expect("inspect");
    writeln!(out, "braid_gen {}", header.braid_gen).expect("inspect");
    writeln!(out, "prev {}", hex32(&header.prev)).expect("inspect");
    writeln!(out, "writer {}", header.writer).expect("inspect");
    writeln!(out, "timestamp {}", header.timestamp).expect("inspect");
    for op in &doc.ops {
        let verb = match op.kind {
            OpKind::Insert => "insert",
            OpKind::Delete => "delete",
        };
        writeln!(out, "{verb} {} {}", op.relation.0, op.rows.len()).expect("inspect");
        for row in &op.rows {
            out.push(' ');
            for (index, value) in row.iter().enumerate() {
                if index > 0 {
                    out.push('\t');
                }
                write_value(out, value);
            }
            out.push('\n');
        }
    }
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Bool(bit) => write!(out, "{bit}").expect("inspect"),
        Value::U64(n) => write!(out, "{n}").expect("inspect"),
        Value::I64(n) => write!(out, "{n}").expect("inspect"),
        Value::String(text) => write!(out, "{text:?}").expect("inspect"),
        Value::FixedBytes(raw) => out.push_str(&hex_bytes(raw)),
        Value::IntervalU64(interval) => {
            write!(out, "[{}, {})", interval.start(), interval.end()).expect("inspect");
        }
        Value::IntervalI64(interval) => {
            write!(out, "[{}, {})", interval.start(), interval.end()).expect("inspect");
        }
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("inspect");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::{Kind, kind, render};
    use crate::braids::braids;
    use crate::manifest::{Checkpoint, Head, Manifest, hex32};
    use crate::sidecar::{Chain, ChainEntry};
    use bumbledb::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
        StatementDescriptor, ValueType,
    };

    fn kitchen() -> SchemaDescriptor {
        let field = |name: &str, value_type: ValueType| FieldDescriptor {
            name: name.into(),
            value_type,
            generation: Generation::None,
        };
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "sample".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::from([FieldId(0)]),
            }],
        }
    }

    #[test]
    fn a_key_names_one_kind() {
        assert_eq!(kind("manifest").expect("kind"), Kind::Manifest);
        assert_eq!(kind("chain").expect("kind"), Kind::Sidecar);
        assert_eq!(kind("ckpt/aa").expect("kind"), Kind::Checkpoint);
        assert_eq!(kind("log/c00000000/1").expect("kind"), Kind::Batch);
        assert!(kind("ckpt/aa.mdb").is_err());
        assert!(kind("other").is_err());
    }

    #[test]
    fn inspect_walks_the_crate_parsers() {
        let theory = kitchen();
        let braid = *braids(&theory).components().keys().next().expect("braid");
        let manifest = Manifest {
            fingerprint: [0x11; 32],
            checkpoint: None,
        };
        let text = render(Kind::Manifest, &manifest.render(), &theory).expect("manifest");
        assert!(text.contains("manifest"));
        assert!(text.contains(&hex32(&manifest.fingerprint)));
        assert!(text.contains("checkpoint none"));

        let ckpt = Checkpoint {
            braids: [(
                braid,
                Head {
                    g: 1,
                    hash: [0x22; 32],
                    ts: 9,
                },
            )]
            .into(),
            catalog: [0x33; 32],
            writer: 7,
            prev: None,
        };
        let text = render(Kind::Checkpoint, &ckpt.render(), &theory).expect("checkpoint");
        assert!(text.contains("checkpoint"));
        assert!(text.contains(&format!("{braid} g 1")));
        assert_eq!(
            text.lines().next().expect("digest"),
            format!("digest {}", hex32(&ckpt.digest()))
        );

        let chain = Chain::Settled {
            entries: [(
                braid,
                ChainEntry {
                    g: 2,
                    prev: [0x44; 32],
                    ts: 3,
                },
            )]
            .into(),
        };
        let text = render(Kind::Sidecar, &chain.render(), &theory).expect("sidecar");
        assert!(text.contains("sidecar settled"));
        assert!(text.contains(&format!("{braid} g 2")));
    }
}
