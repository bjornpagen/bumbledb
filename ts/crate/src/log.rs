//! The bumbledb-log grammar bridge. One implementation reads and writes
//! the protocol's bytes — `crates/bumbledb-log` — and this module only
//! carries payloads across: the sealed per-theory `LogCodec` handle
//! (batch encode/decode + braid derivation) and the document lanes
//! (manifest, checkpoint, sidecar, checkpoint-scratch), bytes in and
//! plain tagged payloads out, grammar only — no store verb, no fd, no
//! clock crosses. Every refusal crossing the boundary carries its
//! identity kind exactly as the log core spells it, minted through the
//! checked-in `log-identities.json` table, so an identity outside the
//! table is a loud bridge error, never a silent new string on the wire.

use std::collections::BTreeMap;

use bumbledb::Value;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, StatementKind, Weight,
};
use bumbledb_log::braids::{BraidId, braids};
use bumbledb_log::codec::{Batch, BatchHeader, Codec, DecodeError, Op};
use bumbledb_log::manifest::{Checkpoint, Head, Manifest};
use bumbledb_log::replica::{encode_ckpt_scratch, parse_ckpt_scratch};
use bumbledb_log::sidecar::{Chain, ChainEntry, Pending};
use napi::bindgen_prelude::{
    Array, BigInt, Buffer, Env, External, Object, ToNapiValue, Uint8Array,
};
use napi::sys;
use napi_derive::napi;

use crate::marshal::{self, ValueOut};
use crate::tags;

/// The mint-table row families, one per bumbledb-log refusal enum,
/// pinned to `log-identities.json` by the golden test below. A refusal
/// kind crosses only through [`mint`], so a kind these rosters do not
/// carry cannot cross silently.
const BATCH_DECODE_IDENTITIES: &[&str] = &[
    "Truncated",
    "BadMagic",
    "Version",
    "Flags",
    "FingerprintMismatch",
    "UnknownBraid",
    "UnknownOpKind",
    "UnknownRelation",
    "ClosedRelation",
    "OpRelationOutsideBraid",
    "TagMismatch",
    "BoolByte",
    "NonCanonicalF64",
    "InvalidUtf8",
    "EmptyInterval",
    "IntervalOverflow",
    "TrailingBytes",
];

/// The encode family rides the exhaustive `wire_tags!` table — a new
/// core variant fails compile in `tags::log_encode_refusal`, and the
/// table is an assertee of the core's own speller: the
/// `encode_tags_are_the_core_identities` test pins each row to
/// `EncodeError::identity`.
const BATCH_ENCODE_IDENTITIES: &[&str] = tags::log_encode_refusal::TAGS;

const MANIFEST_IDENTITIES: &[&str] = &["Malformed", "Version"];

const CHECKPOINT_IDENTITIES: &[&str] = &[
    "Malformed",
    "Version",
    "Overflow",
    "UnknownBraid",
    "BraidSet",
];

const SIDECAR_IDENTITIES: &[&str] = &["Malformed", "Version", "UnknownBraid", "Overflow"];

/// The one gate a refusal identity crosses through: membership in its
/// family row of the mint table, or a loud bridge error.
fn mint(family: &'static [&'static str], kind: &'static str) -> napi::Result<&'static str> {
    if family.contains(&kind) {
        Ok(kind)
    } else {
        Err(marshal::err(format!(
            "bumbledb-log marshal: refusal identity `{kind}` is outside the mint table"
        )))
    }
}

/// A grammar lane's domain outcome: the payload, or a refusal row
/// `{ ok: false, kind, message }` whose kind is a mint-table entry.
pub enum LogOutcome<T> {
    Value(T),
    Refused { kind: &'static str, message: String },
}

impl<T: ToNapiValue> ToNapiValue for LogOutcome<T> {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds a plain object and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        match val {
            Self::Value(value) => {
                obj.set("ok", true)?;
                obj.set("value", value)?;
            }
            Self::Refused { kind, message } => {
                obj.set("ok", false)?;
                obj.set("kind", kind)?;
                obj.set("message", message)?;
            }
        }
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// The sealed per-theory log codec: the descriptor parsed once
/// (vocabulary + braid map) beside the fingerprint the wire pins.
/// Immutable plain data — no lifecycle verb, no capability pointer; the
/// External drops with GC.
pub struct LogCodecHandle {
    codec: Codec,
}

fn digest_in(bytes: &Uint8Array, ctx: &str) -> napi::Result<[u8; 32]> {
    let raw: &[u8] = bytes;
    <[u8; 32]>::try_from(raw).map_err(|_| {
        marshal::err(format!(
            "bumbledb-log marshal: {ctx}: expected 32 bytes, got {}",
            raw.len()
        ))
    })
}

fn fingerprint_in(hex: &str) -> napi::Result<[u8; 32]> {
    let refuse = || {
        marshal::err(format!(
            "bumbledb-log marshal: fingerprint is not 64 hex characters: `{hex}`"
        ))
    };
    if hex.len() != 64 || !hex.is_ascii() {
        return Err(refuse());
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(hex.as_bytes().as_chunks::<2>().0) {
        let text = std::str::from_utf8(pair).map_err(|_| refuse())?;
        *slot = u8::from_str_radix(text, 16).map_err(|_| refuse())?;
    }
    Ok(out)
}

fn projection_in(obj: &Object, ctx: &str) -> napi::Result<Box<[FieldId]>> {
    let arr: Array = marshal::req(obj, "projection", ctx)?;
    let mut fields = Vec::with_capacity(arr.len() as usize);
    for index in 0..arr.len() {
        let id = marshal::ordinal(
            marshal::req_at::<f64>(&arr, index, "statement projection")?,
            "projection field",
        )?;
        fields.push(FieldId(marshal::u16_id(id, "projection field")?));
    }
    Ok(fields.into_boxed_slice())
}

fn side_in(obj: &Object) -> napi::Result<Side> {
    Ok(Side {
        relation: RelationId(marshal::ordinal(
            marshal::req::<f64>(obj, "relation", "statement side")?,
            "side relation",
        )?),
        projection: projection_in(obj, "statement side")?,
        // The braid derivation reads the side's relation and projection
        // only; σ selections never cross.
        selection: Vec::new().into_boxed_slice(),
    })
}

fn statement_in(obj: &Object) -> napi::Result<StatementDescriptor> {
    let kind: String = marshal::req(obj, "kind", "descriptor statement")?;
    let kind = tags::statement_kind::parse(&kind).ok_or_else(|| {
        marshal::err(format!("bumbledb marshal: unknown statement kind `{kind}`"))
    })?;
    Ok(match kind {
        StatementKind::Functionality => StatementDescriptor::Functionality {
            relation: RelationId(marshal::ordinal(
                marshal::req::<f64>(obj, "relation", "fd statement")?,
                "fd relation",
            )?),
            projection: projection_in(obj, "fd statement")?,
        },
        StatementKind::Containment => StatementDescriptor::Containment {
            source: side_in(&marshal::req::<Object>(
                obj,
                "source",
                "containment statement",
            )?)?,
            target: side_in(&marshal::req::<Object>(
                obj,
                "target",
                "containment statement",
            )?)?,
        },
        StatementKind::Capacity => StatementDescriptor::Capacity {
            target: side_in(&marshal::req::<Object>(
                obj,
                "target",
                "capacity statement",
            )?)?,
            // The braid derivation reads the relations and the target
            // projection only; the weight and window never cross.
            weight: Weight::Unit,
            lo: 0,
            hi: None,
            source: side_in(&marshal::req::<Object>(
                obj,
                "source",
                "capacity statement",
            )?)?,
        },
    })
}

/// Rebuilds the grammar core's descriptor from the `DescriptorWire`
/// shape (`internalDescriptor`'s output). The walker reads exactly what
/// the core consumes: per relation the declared field types and `fresh`
/// attrs plus closedness (extension PRESENCE — the axiom rows never
/// cross); per statement the relations and the projections the braid
/// derivation reads. The wire's statements are the MATERIALIZED list,
/// so the walker strips the fresh/closed-key prefix
/// `materialized_statements` mints — re-materialization on the rebuilt
/// descriptor reproduces the wire's statement ids exactly, which is
/// what pins the serial-at statement ids.
fn descriptor_in(obj: &Object) -> napi::Result<SchemaDescriptor> {
    let relations_arr: Array = marshal::req(obj, "relations", "log descriptor")?;
    let mut relations = Vec::with_capacity(relations_arr.len() as usize);
    let mut strip: u32 = 0;
    for index in 0..relations_arr.len() {
        let relation = marshal::req_at::<Object>(&relations_arr, index, "descriptor relations")?;
        let name: String = marshal::req(&relation, "name", "descriptor relation")?;
        let closed = relation.get::<Array>("extension")?.is_some();
        let fields_arr: Array = marshal::req(&relation, "fields", "descriptor relation")?;
        if closed {
            if fields_arr.len() == 0 {
                return Err(marshal::err(format!(
                    "bumbledb-log marshal: closed relation `{name}` has no synthetic id slot"
                )));
            }
            strip += 1;
        }
        // A closed relation's sealed slot 0 is the synthetic handle id;
        // the declared fields follow it.
        let first_declared = u32::from(closed);
        let mut fields = Vec::with_capacity((fields_arr.len() - first_declared) as usize);
        for field_index in first_declared..fields_arr.len() {
            let field = marshal::req_at::<Object>(&fields_arr, field_index, "relation fields")?;
            let value_type: Object = marshal::req(&field, "valueType", "descriptor field")?;
            let fresh: bool = marshal::req(&field, "fresh", "descriptor field")?;
            if fresh {
                strip += 1;
            }
            fields.push(FieldDescriptor {
                name: marshal::req::<String>(&field, "name", "descriptor field")?.into(),
                value_type: marshal::value_type_in(&value_type)?,
                generation: if fresh {
                    Generation::Fresh
                } else {
                    Generation::None
                },
            });
        }
        relations.push(RelationDescriptor {
            name: name.into(),
            fields,
            extension: closed.then(|| Vec::new().into_boxed_slice()),
        });
    }
    let statements_arr: Array = marshal::req(obj, "statements", "log descriptor")?;
    if statements_arr.len() < strip {
        return Err(marshal::err(format!(
            "bumbledb-log marshal: descriptor statements ({}) shorter than the materialized prefix ({strip})",
            statements_arr.len()
        )));
    }
    let mut statements = Vec::with_capacity((statements_arr.len() - strip) as usize);
    for index in strip..statements_arr.len() {
        let statement = marshal::req_at::<Object>(&statements_arr, index, "descriptor statements")?;
        statements.push(statement_in(&statement)?);
    }
    Ok(SchemaDescriptor {
        relations,
        statements,
    })
}

/// Seals the theory's log codec from the engine's `DescriptorWire`.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_codec(descriptor: Object) -> napi::Result<External<LogCodecHandle>> {
    let fingerprint = fingerprint_in(&marshal::req::<String>(
        &descriptor,
        "fingerprint",
        "log descriptor",
    )?)?;
    let parsed = descriptor_in(&descriptor)?;
    Ok(External::new(LogCodecHandle {
        codec: Codec::new(&parsed, fingerprint),
    }))
}

/// The braid decomposition and the serial-at statement ids, riding the
/// same `DescriptorWire`. A pure derivation; the TS driver caches the
/// result per theory beside the codec handle.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_braids_of(descriptor: Object) -> napi::Result<BraidsWire> {
    let parsed = descriptor_in(&descriptor)?;
    Ok(BraidsWire::of(&braids(&parsed)))
}

pub struct BraidsWire {
    components: Vec<(u32, Vec<u32>)>,
    serial_at: Vec<u32>,
}

impl BraidsWire {
    fn of(braids: &bumbledb_log::braids::Braids) -> Self {
        Self {
            components: braids
                .components()
                .into_iter()
                .map(|(braid, relations)| {
                    (
                        braid.raw(),
                        relations.into_iter().map(|relation| relation.0).collect(),
                    )
                })
                .collect(),
            serial_at: braids
                .serial_at()
                .iter()
                .map(|statement| u32::from(statement.0))
                .collect(),
        }
    }
}

impl ToNapiValue for BraidsWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut root = Object::new(&env_handle)?;
        let mut components = Vec::with_capacity(val.components.len());
        for (braid, relations) in val.components {
            let mut component = Object::new(&env_handle)?;
            component.set("braid", braid)?;
            component.set("relations", relations)?;
            components.push(component);
        }
        root.set("components", components)?;
        root.set("serialAt", val.serial_at)?;
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `root` was created against it.
        unsafe { Object::to_napi_value(env, root) }
    }
}

fn op_in(obj: &Object) -> napi::Result<Op> {
    let kind: String = marshal::req(obj, "kind", "batch op")?;
    let kind = tags::log_op::parse(&kind)
        .ok_or_else(|| marshal::err(format!("bumbledb marshal: unknown op kind `{kind}`")))?;
    let relation = RelationId(marshal::ordinal(
        marshal::req::<f64>(obj, "relation", "batch op")?,
        "op relation",
    )?);
    let wire_rows: Array = marshal::req(obj, "rows", "batch op")?;
    let mut rows: Vec<Box<[Value]>> = Vec::with_capacity(wire_rows.len() as usize);
    for row_index in 0..wire_rows.len() {
        let wire_row = marshal::req_at::<Array>(&wire_rows, row_index, "op rows")?;
        let mut row: Vec<Value> = Vec::with_capacity(wire_row.len() as usize);
        for cell_index in 0..wire_row.len() {
            let cell = marshal::req_at::<Object>(&wire_row, cell_index, "op row")?;
            row.push(marshal::tagged_value(&cell)?);
        }
        rows.push(row.into_boxed_slice());
    }
    Ok(Op {
        kind,
        relation,
        rows,
    })
}

/// Encodes a batch through the sealed codec: header + tagged-value ops
/// in (the query-literal lane's inbound spelling), wire bytes out. The
/// handle is the fingerprint authority — the header wire never carries
/// one — and the header braid is minted through the codec's own map, so
/// the bridge mints `UnknownBraid` as the twin of the core's encode
/// refusal; every other refusal is the core's own identity row.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_encode_batch(
    handle: &External<LogCodecHandle>,
    header: Object,
    ops: Array,
) -> napi::Result<LogOutcome<Buffer>> {
    let braid_raw = marshal::ordinal(
        marshal::req::<f64>(&header, "braid", "batch header")?,
        "header braid",
    )?;
    let Some(braid) = handle.codec.braids().parse(braid_raw) else {
        return Ok(LogOutcome::Refused {
            kind: mint(BATCH_ENCODE_IDENTITIES, "UnknownBraid")?,
            message: format!("bumbledb-log encode refusal: UnknownBraid {{ braid: {braid_raw} }}"),
        });
    };
    let parsed_header = BatchHeader {
        fingerprint: *handle.codec.fingerprint(),
        braid,
        braid_gen: marshal::u64_in(
            &marshal::req::<BigInt>(&header, "braidGen", "batch header")?,
            "header braidGen",
        )?,
        prev: digest_in(
            &marshal::req::<Uint8Array>(&header, "prev", "batch header")?,
            "header prev",
        )?,
        writer: marshal::u64_in(
            &marshal::req::<BigInt>(&header, "writer", "batch header")?,
            "header writer",
        )?,
        timestamp: marshal::u64_in(
            &marshal::req::<BigInt>(&header, "timestamp", "batch header")?,
            "header timestamp",
        )?,
    };
    let mut parsed_ops: Vec<Op> = Vec::with_capacity(ops.len() as usize);
    for index in 0..ops.len() {
        let op = marshal::req_at::<Object>(&ops, index, "batch ops")?;
        parsed_ops.push(op_in(&op)?);
    }
    match handle.codec.encode(&parsed_header, &parsed_ops) {
        Ok(bytes) => Ok(LogOutcome::Value(Buffer::from(bytes))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: mint(
                BATCH_ENCODE_IDENTITIES,
                tags::log_encode_refusal::tag(&refusal),
            )?,
            message: format!("bumbledb-log encode refusal: {refusal:?}"),
        }),
    }
}

/// Decodes a batch: the core's full sequential parse; rows cross exactly
/// as the engine's `ValueOut` walk. The header wire omits the
/// fingerprint — decode already refused any batch whose fingerprint is
/// not the handle's own, so carrying it back is dead data.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_decode_batch(
    handle: &External<LogCodecHandle>,
    bytes: Uint8Array,
) -> napi::Result<LogOutcome<BatchWire>> {
    match handle.codec.decode(&bytes) {
        Ok(batch) => Ok(LogOutcome::Value(BatchWire(batch))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: mint(BATCH_DECODE_IDENTITIES, refusal.identity())?,
            message: decode_refusal_message(&refusal),
        }),
    }
}

fn decode_refusal_message(refusal: &DecodeError) -> String {
    match refusal {
        DecodeError::NonCanonicalF64 {
            relation,
            row,
            field,
            bits,
        } => format!(
            "bumbledb-log decode refusal: NonCanonicalF64 {{ relation: {}, row: {row}, field: {field}, bits: 0x{bits:016x} }}",
            relation.0
        ),
        _ => format!("bumbledb-log decode refusal: {refusal:?}"),
    }
}

pub struct BatchWire(Batch);

impl ToNapiValue for BatchWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let batch = val.0;
        let mut root = Object::new(&env_handle)?;
        let mut header = Object::new(&env_handle)?;
        header.set("braid", batch.header.braid.raw())?;
        header.set("braidGen", batch.header.braid_gen)?;
        header.set("prev", Buffer::from(batch.header.prev.to_vec()))?;
        header.set("writer", batch.header.writer)?;
        header.set("timestamp", batch.header.timestamp)?;
        root.set("header", header)?;
        let mut ops = Vec::with_capacity(batch.ops.len());
        for op in batch.ops {
            let mut op_obj = Object::new(&env_handle)?;
            op_obj.set("kind", tags::log_op::tag(&op.kind))?;
            op_obj.set("relation", op.relation.0)?;
            let rows: Vec<Vec<ValueOut>> = op
                .rows
                .into_iter()
                .map(|row| {
                    row.into_vec()
                        .into_iter()
                        .map(ValueOut::from_value)
                        .collect()
                })
                .collect();
            op_obj.set("rows", rows)?;
            ops.push(op_obj);
        }
        root.set("ops", ops)?;
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `root` was created against it.
        unsafe { Object::to_napi_value(env, root) }
    }
}

/// Parses manifest bytes: version, fingerprint, optional checkpoint
/// digest. Grammar only — the CAS verbs live in the TS driver.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_parse_manifest(bytes: Uint8Array) -> napi::Result<LogOutcome<ManifestDocWire>> {
    match Manifest::parse(&bytes) {
        Ok(manifest) => Ok(LogOutcome::Value(ManifestDocWire(manifest))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: mint(MANIFEST_IDENTITIES, refusal.identity())?,
            message: format!("bumbledb-log manifest refusal: {refusal:?}"),
        }),
    }
}

/// Renders the manifest's one encoding — byte-exact for CAS bodies.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_render_manifest(doc: Object) -> napi::Result<Buffer> {
    let fingerprint = digest_in(
        &marshal::req::<Uint8Array>(&doc, "fingerprint", "manifest document")?,
        "manifest fingerprint",
    )?;
    let checkpoint = doc
        .get::<Uint8Array>("checkpoint")?
        .map(|digest| digest_in(&digest, "manifest checkpoint"))
        .transpose()?;
    Ok(Buffer::from(
        Manifest {
            fingerprint,
            checkpoint,
        }
        .render(),
    ))
}

pub struct ManifestDocWire(Manifest);

impl ToNapiValue for ManifestDocWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds a plain object and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        obj.set("fingerprint", Buffer::from(val.0.fingerprint.to_vec()))?;
        if let Some(checkpoint) = val.0.checkpoint {
            obj.set("checkpoint", Buffer::from(checkpoint.to_vec()))?;
        }
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `obj` was created against it.
        unsafe { Object::to_napi_value(env, obj) }
    }
}

fn braid_in(codec: &Codec, obj: &Object, ctx: &str) -> napi::Result<BraidId> {
    let raw = marshal::ordinal(marshal::req::<f64>(obj, "braid", ctx)?, "braid id")?;
    codec.braids().parse(raw).ok_or_else(|| {
        marshal::err(format!(
            "bumbledb-log marshal: {ctx}: braid {raw} is not in the theory's decomposition"
        ))
    })
}

/// Parses a `ckpt/{digest}` document against the theory's own braid
/// decomposition. Derived facts (digest, vector sum) stay derived — the
/// TS driver owns the vector algebra.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_parse_checkpoint(
    handle: &External<LogCodecHandle>,
    bytes: Uint8Array,
) -> napi::Result<LogOutcome<CheckpointWire>> {
    match Checkpoint::parse(&bytes, handle.codec.braids()) {
        Ok(doc) => Ok(LogOutcome::Value(CheckpointWire(doc))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: mint(CHECKPOINT_IDENTITIES, refusal.identity())?,
            message: format!("bumbledb-log checkpoint refusal: {refusal:?}"),
        }),
    }
}

/// Renders a checkpoint document. Braid ids mint through the codec's own
/// map; an id the decomposition does not mint, or a duplicate, is a
/// shape refusal — render itself is total.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_render_checkpoint(
    handle: &External<LogCodecHandle>,
    doc: Object,
) -> napi::Result<Buffer> {
    let braids_arr: Array = marshal::req(&doc, "braids", "checkpoint document")?;
    let mut heads: BTreeMap<BraidId, Head> = BTreeMap::new();
    for index in 0..braids_arr.len() {
        let entry = marshal::req_at::<Object>(&braids_arr, index, "checkpoint braids")?;
        let braid = braid_in(&handle.codec, &entry, "checkpoint head")?;
        let head = Head {
            g: marshal::u64_in(
                &marshal::req::<BigInt>(&entry, "g", "checkpoint head")?,
                "checkpoint g",
            )?,
            hash: digest_in(
                &marshal::req::<Uint8Array>(&entry, "hash", "checkpoint head")?,
                "checkpoint hash",
            )?,
            ts: marshal::u64_in(
                &marshal::req::<BigInt>(&entry, "ts", "checkpoint head")?,
                "checkpoint ts",
            )?,
        };
        if heads.insert(braid, head).is_some() {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: duplicate braid {} in checkpoint document",
                braid.raw()
            )));
        }
    }
    let parsed = Checkpoint {
        braids: heads,
        catalog: digest_in(
            &marshal::req::<Uint8Array>(&doc, "catalog", "checkpoint document")?,
            "checkpoint catalog",
        )?,
        writer: marshal::u64_in(
            &marshal::req::<BigInt>(&doc, "writer", "checkpoint document")?,
            "checkpoint writer",
        )?,
        prev: doc
            .get::<Uint8Array>("prev")?
            .map(|digest| digest_in(&digest, "checkpoint prev"))
            .transpose()?,
    };
    Ok(Buffer::from(parsed.render()))
}

pub struct CheckpointWire(Checkpoint);

impl ToNapiValue for CheckpointWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let doc = val.0;
        let mut root = Object::new(&env_handle)?;
        let mut heads = Vec::with_capacity(doc.braids.len());
        for (braid, head) in doc.braids {
            let mut entry = Object::new(&env_handle)?;
            entry.set("braid", braid.raw())?;
            entry.set("g", head.g)?;
            entry.set("hash", Buffer::from(head.hash.to_vec()))?;
            entry.set("ts", head.ts)?;
            heads.push(entry);
        }
        root.set("braids", heads)?;
        root.set("catalog", Buffer::from(doc.catalog.to_vec()))?;
        root.set("writer", doc.writer)?;
        if let Some(prev) = doc.prev {
            root.set("prev", Buffer::from(prev.to_vec()))?;
        }
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `root` was created against it.
        unsafe { Object::to_napi_value(env, root) }
    }
}

/// Parses chain-sidecar bytes against the theory's braid decomposition.
/// The fs half — atomic write, the total `Absent`/`Fault`/`Corrupt` read
/// — stays in the TS driver; only the grammar crosses.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_parse_sidecar(
    handle: &External<LogCodecHandle>,
    bytes: Uint8Array,
) -> napi::Result<LogOutcome<ChainWire>> {
    match Chain::parse(&bytes, handle.codec.braids()) {
        Ok(chain) => Ok(LogOutcome::Value(ChainWire(chain))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: mint(SIDECAR_IDENTITIES, refusal.identity())?,
            message: format!("bumbledb-log sidecar refusal: {refusal:?}"),
        }),
    }
}

/// Renders a chain sidecar. An absent `pending` key is the `Settled`
/// arm — the fused-option spelling the schema wire already uses.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_render_sidecar(
    handle: &External<LogCodecHandle>,
    chain: Object,
) -> napi::Result<Buffer> {
    let entries_arr: Array = marshal::req(&chain, "entries", "sidecar document")?;
    let mut entries: BTreeMap<BraidId, ChainEntry> = BTreeMap::new();
    for index in 0..entries_arr.len() {
        let entry = marshal::req_at::<Object>(&entries_arr, index, "sidecar entries")?;
        let braid = braid_in(&handle.codec, &entry, "sidecar entry")?;
        let parsed = ChainEntry {
            g: marshal::u64_in(
                &marshal::req::<BigInt>(&entry, "g", "sidecar entry")?,
                "sidecar g",
            )?,
            prev: digest_in(
                &marshal::req::<Uint8Array>(&entry, "prev", "sidecar entry")?,
                "sidecar prev",
            )?,
            ts: marshal::u64_in(
                &marshal::req::<BigInt>(&entry, "ts", "sidecar entry")?,
                "sidecar ts",
            )?,
        };
        if entries.insert(braid, parsed).is_some() {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: duplicate braid {} in sidecar document",
                braid.raw()
            )));
        }
    }
    let parsed = match chain.get::<Object>("pending")? {
        None => Chain::Settled { entries },
        Some(pending) => {
            let bytes: Uint8Array = marshal::req(&pending, "bytes", "pending batch")?;
            if u32::try_from(bytes.len()).is_err() {
                return Err(marshal::err(format!(
                    "bumbledb-log marshal: pending batch of {} bytes exceeds the u32 length prefix",
                    bytes.len()
                )));
            }
            Chain::Pending {
                entries,
                batch: Pending {
                    braid: braid_in(&handle.codec, &pending, "pending batch")?,
                    slot: marshal::u64_in(
                        &marshal::req::<BigInt>(&pending, "slot", "pending batch")?,
                        "pending slot",
                    )?,
                    bytes: bytes.to_vec(),
                },
            }
        }
    };
    Ok(Buffer::from(parsed.render()))
}

pub struct ChainWire(Chain);

impl ToNapiValue for ChainWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let (entries, pending) = match val.0 {
            Chain::Settled { entries } => (entries, None),
            Chain::Pending { entries, batch } => (entries, Some(batch)),
        };
        let mut root = Object::new(&env_handle)?;
        let mut wire_entries = Vec::with_capacity(entries.len());
        for (braid, entry) in entries {
            let mut obj = Object::new(&env_handle)?;
            obj.set("braid", braid.raw())?;
            obj.set("g", entry.g)?;
            obj.set("prev", Buffer::from(entry.prev.to_vec()))?;
            obj.set("ts", entry.ts)?;
            wire_entries.push(obj);
        }
        root.set("entries", wire_entries)?;
        if let Some(batch) = pending {
            let mut obj = Object::new(&env_handle)?;
            obj.set("braid", batch.braid.raw())?;
            obj.set("slot", batch.slot)?;
            obj.set("bytes", Buffer::from(batch.bytes))?;
            root.set("pending", obj)?;
        }
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `root` was created against it.
        unsafe { Object::to_napi_value(env, root) }
    }
}

/// The digest a scratch-lease body names, or null — the refusal is
/// undifferentiated by law (both drivers yield a bare none; naming
/// variants would over-pin).
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn log_parse_ckpt_scratch(bytes: Uint8Array) -> Option<Buffer> {
    parse_ckpt_scratch(&bytes).map(|digest| Buffer::from(digest.to_vec()))
}

/// Renders the scratch-lease body: version byte, then the 32-byte
/// digest.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_render_ckpt_scratch(digest: Uint8Array) -> napi::Result<Buffer> {
    let digest = digest_in(&digest, "scratch digest")?;
    Ok(Buffer::from(encode_ckpt_scratch(&digest).to_vec()))
}

#[cfg(test)]
mod mint_table {
    use super::{
        BATCH_DECODE_IDENTITIES, BATCH_ENCODE_IDENTITIES, CHECKPOINT_IDENTITIES,
        MANIFEST_IDENTITIES, SIDECAR_IDENTITIES, mint,
    };
    use bumbledb::schema::{RelationDescriptor, RelationId, SchemaDescriptor, ValueType};
    use bumbledb_log::braids::{BraidId, braids};
    use bumbledb_log::codec::{DecodeError, EncodeError, ValueShape};
    use bumbledb_log::manifest::{CheckpointError, ManifestError};
    use bumbledb_log::sidecar::SidecarError;
    use serde_json::Value as Json;

    use crate::tags;

    fn one_braid() -> BraidId {
        let descriptor = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "sample".into(),
                fields: vec![],
                extension: None,
            }],
            statements: vec![],
        };
        braids(&descriptor)
            .parse(0)
            .expect("relation 0 is its own braid")
    }

    #[test]
    fn identities_json_matches_the_tables() {
        let committed: Json = serde_json::from_str(include_str!("../log-identities.json"))
            .expect("ts/crate/log-identities.json parses");
        for (key, table) in [
            ("batchDecode", BATCH_DECODE_IDENTITIES),
            ("batchEncode", BATCH_ENCODE_IDENTITIES),
            ("manifest", MANIFEST_IDENTITIES),
            ("checkpoint", CHECKPOINT_IDENTITIES),
            ("sidecar", SIDECAR_IDENTITIES),
        ] {
            let expected: Vec<Json> = table.iter().map(|kind| Json::from(*kind)).collect();
            assert_eq!(
                committed.get(key).and_then(Json::as_array),
                Some(&expected),
                "log-identities.json `{key}` drifted from the in-crate table — \
                 update the stub to match the enums (never the reverse without \
                 a core change)"
            );
        }
    }

    #[test]
    fn every_decode_identity_is_a_table_row_in_order() {
        let braid = one_braid();
        let witnesses = [
            DecodeError::Truncated { offset: 0 },
            DecodeError::BadMagic { got: [0; 4] },
            DecodeError::Version { got: 0 },
            DecodeError::Flags { got: 1 },
            DecodeError::FingerprintMismatch { got: [0; 32] },
            DecodeError::UnknownBraid { got: 1 },
            DecodeError::UnknownOpKind { op: 0, got: 3 },
            DecodeError::UnknownRelation {
                op: 0,
                relation: RelationId(9),
            },
            DecodeError::ClosedRelation {
                op: 0,
                relation: RelationId(0),
            },
            DecodeError::OpRelationOutsideBraid {
                op: 0,
                relation: RelationId(0),
                braid,
            },
            DecodeError::TagMismatch {
                relation: RelationId(0),
                row: 0,
                field: 0,
                expected: ValueType::Bool,
                got: 9,
            },
            DecodeError::BoolByte {
                relation: RelationId(0),
                row: 0,
                field: 0,
                got: 2,
            },
            DecodeError::NonCanonicalF64 {
                relation: RelationId(0),
                row: 0,
                field: 0,
                bits: 0x8000_0000_0000_0000,
            },
            DecodeError::InvalidUtf8 {
                relation: RelationId(0),
                row: 0,
                field: 0,
            },
            DecodeError::EmptyInterval {
                relation: RelationId(0),
                row: 0,
                field: 0,
            },
            DecodeError::IntervalOverflow {
                relation: RelationId(0),
                row: 0,
                field: 0,
            },
            DecodeError::TrailingBytes { at: 0 },
        ];
        let spelled: Vec<&'static str> = witnesses.iter().map(DecodeError::identity).collect();
        assert_eq!(spelled, BATCH_DECODE_IDENTITIES);
    }

    #[test]
    fn noncanonical_float_diagnostic_preserves_all_bits_as_text() {
        let error = DecodeError::NonCanonicalF64 {
            relation: RelationId(7),
            row: 3,
            field: 2,
            bits: 0xfff0_0000_0000_0001,
        };
        assert_eq!(
            super::decode_refusal_message(&error),
            "bumbledb-log decode refusal: NonCanonicalF64 { relation: 7, row: 3, field: 2, bits: 0xfff0000000000001 }"
        );
    }

    #[test]
    fn encode_tags_are_the_core_identities() {
        let braid = one_braid();
        let witnesses = [
            EncodeError::FingerprintMismatch,
            EncodeError::UnknownBraid { braid: 1 },
            EncodeError::UnknownRelation {
                op: 0,
                relation: RelationId(9),
            },
            EncodeError::ClosedRelation {
                op: 0,
                relation: RelationId(0),
            },
            EncodeError::OpRelationOutsideBraid {
                op: 0,
                relation: RelationId(0),
                braid,
            },
            EncodeError::Arity {
                op: 0,
                relation: RelationId(0),
                row: 0,
            },
            EncodeError::Value {
                op: 0,
                relation: RelationId(0),
                row: 0,
                field: 0,
                cause: ValueShape::Kind {
                    expected: ValueType::Bool,
                },
            },
            EncodeError::TooManyOps,
            EncodeError::TooManyRows { op: 0 },
        ];
        for refusal in &witnesses {
            assert_eq!(
                tags::log_encode_refusal::tag(refusal),
                refusal.identity(),
                "the bridge tag row is the core's own identity"
            );
        }
        let spelled: Vec<&'static str> = witnesses.iter().map(EncodeError::identity).collect();
        assert_eq!(
            spelled, BATCH_ENCODE_IDENTITIES,
            "the witness list covers every encode variant in table order"
        );
    }

    #[test]
    fn every_document_identity_is_a_table_row_in_order() {
        let manifest = [
            ManifestError::Malformed { at: 0 },
            ManifestError::Version { got: 0 },
        ];
        let spelled: Vec<&'static str> = manifest.iter().map(ManifestError::identity).collect();
        assert_eq!(spelled, MANIFEST_IDENTITIES);

        let checkpoint = [
            CheckpointError::Malformed { at: 0 },
            CheckpointError::Version { got: 0 },
            CheckpointError::Overflow,
            CheckpointError::UnknownBraid { got: 1 },
            CheckpointError::BraidSet,
        ];
        let spelled: Vec<&'static str> = checkpoint.iter().map(CheckpointError::identity).collect();
        assert_eq!(spelled, CHECKPOINT_IDENTITIES);

        let sidecar = [
            SidecarError::Malformed { at: 0 },
            SidecarError::Version { got: 0 },
            SidecarError::UnknownBraid { got: 1 },
            SidecarError::Overflow,
        ];
        let spelled: Vec<&'static str> = sidecar.iter().map(SidecarError::identity).collect();
        assert_eq!(spelled, SIDECAR_IDENTITIES);
    }

    #[test]
    fn an_identity_outside_the_table_cannot_cross() {
        assert!(mint(MANIFEST_IDENTITIES, "Malformed").is_ok());
        let refused = mint(MANIFEST_IDENTITIES, "BraidSet").expect_err("foreign identity");
        assert!(
            refused.reason.contains("outside the mint table"),
            "{}",
            refused.reason
        );
    }
}
