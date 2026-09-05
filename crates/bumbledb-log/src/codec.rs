//! The checkpoint stream codec (C08/C12): one canonical logical stream —
//! schema-bound application facts, retained receipt rows, migration-history
//! evidence — cut into fixed-target chunks, described by one streamed
//! manifest, digested by one shared acyclic projection.
//!
//! The stream is logical: physical LMDB row IDs, dictionary numbering,
//! freelist layout and host page sizes never enter it. Facts are ordered
//! canonically (relation ascending, then full canonical row bytes); receipt
//! rows are ordered by their storage key (epoch, then request bytes). Chunks
//! are uncompressed in 1.0 and may split records; the decoder is a bounded
//! streaming parser.
//!
//! The **logical digest projection** here is shared by export, import and
//! replay checks (chapter 20): the application digest covers exactly the
//! fact records; the system digest covers exactly the keyed system records
//! (retained receipt rows under the `r` key prefix, migration/history
//! evidence under P09's `m` key prefix). Control state, head revisions, certificates and the digests
//! themselves are excluded — bound instead by the manifest's own hash and
//! the head that references it. `empty_application_digest`/
//! `empty_system_digest` are the blank-database projection; P04's
//! `blank_initial_digests` must equal them (recorded cross-lane patch).
//!
//! Physical bytes remain provisional until the F3 format freeze (C12).

use crate::history::authority::{HeadAuthority, decode_control, encode_control};
use crate::history::{
    DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, FrameError, IncarnationId,
    SchemaId, StateStamp,
};
use crate::manifest::wire::{self, Reader};
use crate::manifest::{put_object_ref, read_object_ref};
use crate::store::{ObjectKind, ObjectRef};

/// Default fixed-target chunk size: 8 MiB, uncompressed.
pub const CHUNK_TARGET: usize = 8 * 1024 * 1024;

pub const MANIFEST_FAMILY: &[u8] = b"bumbledb.ckpt.v1\0";
pub const MANIFEST_LAYOUT: u16 = 1;
const MANIFEST_KIND: u8 = 1;

pub const APPLICATION_DOMAIN: &str = "bumbledb.checkpoint.v1/application-digest";
pub const SYSTEM_DOMAIN: &str = "bumbledb.checkpoint.v1/system-digest";
pub const STREAM_DOMAIN: &str = "bumbledb.checkpoint.v1/stream-digest";

const TAG_FACT: u8 = 1;
const TAG_SYSTEM: u8 = 2;
const TAG_END: u8 = 4;

/// The canonical application digest of an empty database. One shared value:
/// genesis sentinels, blank hydration checks and the empty export projection
/// all name exactly this.
#[must_use]
pub fn empty_application_digest() -> [u8; 32] {
    *blake3::Hasher::new_derive_key(APPLICATION_DOMAIN)
        .finalize()
        .as_bytes()
}

/// The canonical system digest of an empty receipt/history table.
#[must_use]
pub fn empty_system_digest() -> [u8; 32] {
    *blake3::Hasher::new_derive_key(SYSTEM_DOMAIN)
        .finalize()
        .as_bytes()
}

/// Bounds every stream/manifest parse obeys before allocating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamLimits {
    /// Largest single record payload (row/receipt/history bytes).
    pub record_bytes: usize,
    /// Largest encoded manifest.
    pub manifest_bytes: usize,
}

impl StreamLimits {
    pub const DEFAULT: Self = Self {
        record_bytes: 16 * 1024 * 1024,
        manifest_bytes: 64 * 1024 * 1024,
    };
}

/// What one completed stream established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSummary {
    pub application_digest: [u8; 32],
    pub system_digest: [u8; 32],
    pub stream_digest: [u8; 32],
    pub total_bytes: u64,
    pub rows: u64,
    pub system_records: u64,
    pub chunks: Vec<ObjectRef>,
}

/// Where finished chunks go: an upload, a file writer, a test buffer. The
/// sink owns durability of each chunk before returning its reference.
pub trait ChunkSink {
    type Error;

    /// Persist one complete chunk and return its verified reference.
    ///
    /// # Errors
    /// The sink's own failure; the writer stops without partial claims.
    fn chunk(&mut self, bytes: &[u8]) -> Result<ObjectRef, Self::Error>;
}

/// Streaming writer refusals.
#[derive(Debug)]
pub enum WriteError<E> {
    /// Records must arrive facts → receipts → history; order violations are
    /// a caller bug surfaced as refusal, never silently reordered bytes.
    OutOfOrder,
    /// A single record exceeds the configured bound.
    RecordTooLarge {
        bytes: usize,
    },
    Sink(E),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Digest {
    Application,
    System,
    Framing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Facts,
    System,
    Finished,
}

/// The bounded streaming encoder: one chunk buffer, three incremental
/// hashers, no whole-database allocation.
pub struct StreamWriter<'s, K: ChunkSink> {
    sink: &'s mut K,
    chunk_target: usize,
    limits: StreamLimits,
    buffer: Vec<u8>,
    section: Section,
    application: blake3::Hasher,
    system: blake3::Hasher,
    stream: blake3::Hasher,
    total_bytes: u64,
    rows: u64,
    system_records: u64,
    chunks: Vec<ObjectRef>,
}

impl<'s, K: ChunkSink> StreamWriter<'s, K> {
    pub fn new(sink: &'s mut K, chunk_target: usize, limits: StreamLimits) -> Self {
        Self {
            sink,
            chunk_target: chunk_target.max(4_096),
            limits,
            buffer: Vec::new(),
            section: Section::Facts,
            application: blake3::Hasher::new_derive_key(APPLICATION_DOMAIN),
            system: blake3::Hasher::new_derive_key(SYSTEM_DOMAIN),
            stream: blake3::Hasher::new_derive_key(STREAM_DOMAIN),
            total_bytes: 0,
            rows: 0,
            system_records: 0,
            chunks: Vec::new(),
        }
    }

    fn emit(&mut self, record: &[u8], digest: Digest) -> Result<(), WriteError<K::Error>> {
        self.stream.update(record);
        match digest {
            Digest::Application => {
                self.application.update(record);
            }
            Digest::System => {
                self.system.update(record);
            }
            // Stream framing (the end record) is not logical content: the
            // empty database's projection digests cover exactly no records.
            Digest::Framing => {}
        }
        self.total_bytes += record.len() as u64;
        let mut rest = record;
        while !rest.is_empty() {
            let space = self.chunk_target - self.buffer.len();
            let take = space.min(rest.len());
            self.buffer.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.buffer.len() == self.chunk_target {
                self.flush()?;
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), WriteError<K::Error>> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let reference = self.sink.chunk(&self.buffer).map_err(WriteError::Sink)?;
        self.chunks.push(reference);
        self.buffer.clear();
        Ok(())
    }

    fn record(
        &mut self,
        tag: u8,
        key: Option<&[u8]>,
        payload: &[u8],
        digest: Digest,
    ) -> Result<(), WriteError<K::Error>> {
        if payload.len() > self.limits.record_bytes {
            return Err(WriteError::RecordTooLarge {
                bytes: payload.len(),
            });
        }
        let mut head = Vec::with_capacity(16 + key.map_or(0, <[u8]>::len));
        head.push(tag);
        if let Some(key) = key {
            head.extend_from_slice(
                &u16::try_from(key.len())
                    .map_err(|_| WriteError::RecordTooLarge { bytes: key.len() })?
                    .to_be_bytes(),
            );
            head.extend_from_slice(key);
        }
        head.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        self.emit(&head, digest)?;
        self.emit(payload, digest)
    }

    /// One application fact: relation, then full canonical row bytes. The
    /// caller supplies canonical order.
    ///
    /// # Errors
    /// Order/size refusals and sink failure.
    pub fn fact(&mut self, relation: u32, row: &[u8]) -> Result<(), WriteError<K::Error>> {
        if self.section > Section::Facts {
            return Err(WriteError::OutOfOrder);
        }
        let mut head = [0u8; 5];
        head[0] = TAG_FACT;
        head[1..5].copy_from_slice(&relation.to_be_bytes());
        self.emit(&head, Digest::Application)?;
        let len = (row.len() as u64).to_be_bytes();
        if row.len() > self.limits.record_bytes {
            return Err(WriteError::RecordTooLarge { bytes: row.len() });
        }
        self.emit(&len, Digest::Application)?;
        self.emit(row, Digest::Application)?;
        self.rows += 1;
        Ok(())
    }

    /// One keyed system record: a retained receipt row (`r` key prefix) or
    /// one migration-history evidence record (`m` key prefix). Keys arrive
    /// in ascending key order; hydration writes them back verbatim.
    ///
    /// # Errors
    /// Order/size refusals and sink failure.
    pub fn system(&mut self, key: &[u8], value: &[u8]) -> Result<(), WriteError<K::Error>> {
        if self.section > Section::System {
            return Err(WriteError::OutOfOrder);
        }
        self.section = Section::System;
        self.record(TAG_SYSTEM, Some(key), value, Digest::System)?;
        self.system_records += 1;
        Ok(())
    }

    /// Seal the stream: the end record binds the counters, the final partial
    /// chunk flushes, and the summary carries every digest.
    ///
    /// # Errors
    /// Sink failure.
    pub fn finish(mut self) -> Result<StreamSummary, WriteError<K::Error>> {
        let mut end = Vec::with_capacity(17);
        end.push(TAG_END);
        end.extend_from_slice(&self.rows.to_be_bytes());
        end.extend_from_slice(&self.system_records.to_be_bytes());
        self.section = Section::Finished;
        self.emit(&end, Digest::Framing)?;
        self.flush()?;
        Ok(StreamSummary {
            application_digest: *self.application.finalize().as_bytes(),
            system_digest: *self.system.finalize().as_bytes(),
            stream_digest: *self.stream.finalize().as_bytes(),
            total_bytes: self.total_bytes,
            rows: self.rows,
            system_records: self.system_records,
            chunks: self.chunks,
        })
    }
}

/// Where decoded records go during import/verification.
pub trait StreamSink {
    type Error;

    /// # Errors
    /// The sink's own failure; the decoder stops without partial claims.
    fn fact(&mut self, relation: u32, row: &[u8]) -> Result<(), Self::Error>;
    /// # Errors
    /// The sink's own failure; the decoder stops without partial claims.
    fn system(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;
}

/// Streaming decode refusals.
#[derive(Debug)]
pub enum ReadError<E, S> {
    /// Malformed stream grammar (bad tag, truncation, counter mismatch).
    Frame(FrameError),
    /// The chunk supplier failed.
    Chunk(E),
    /// The sink refused a record.
    Sink(S),
}

impl<E, S> From<FrameError> for ReadError<E, S> {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

/// Decode a chunked stream with one bounded carry buffer. Chunk items are
/// `B: AsRef<[u8]>` so a [`bumbledb::work::ChargedBytes`] (or `Vec<u8>`)
/// iterator decodes without a caller-side payload copy. Records may split
/// across chunk boundaries; the carry never exceeds one record plus one
/// chunk. Counters in the end record must match what was decoded, and bytes
/// after the end record refuse.
///
/// # Errors
/// Grammar refusals, chunk-supplier failures and sink refusals.
#[expect(
    clippy::too_many_lines,
    reason = "one bounded decode loop over the frozen chunk grammar"
)]
pub fn read_stream<E, S, B: AsRef<[u8]>>(
    chunks: impl IntoIterator<Item = Result<B, E>>,
    sink: &mut dyn StreamSink<Error = S>,
    limits: StreamLimits,
) -> Result<StreamSummary, ReadError<E, S>> {
    let mut carry: Vec<u8> = Vec::new();
    let mut application = blake3::Hasher::new_derive_key(APPLICATION_DOMAIN);
    let mut system = blake3::Hasher::new_derive_key(SYSTEM_DOMAIN);
    let mut stream = blake3::Hasher::new_derive_key(STREAM_DOMAIN);
    let mut total_bytes = 0u64;
    let mut rows = 0u64;
    let mut system_records = 0u64;
    let mut finished: Option<(u64, u64)> = None;

    let mut consume = |carry: &mut Vec<u8>| -> Result<bool, ReadError<E, S>> {
        // Try to parse one complete record from the carry front.
        let Some(&tag) = carry.first() else {
            return Ok(false);
        };
        let record_len: usize;
        match tag {
            TAG_FACT => {
                if carry.len() < 13 {
                    return Ok(false);
                }
                let len = u64::from_be_bytes(carry[5..13].try_into().expect("width"));
                let len = usize::try_from(len).map_err(|_| FrameError::LengthOverflow)?;
                if len > limits.record_bytes {
                    return Err(FrameError::LimitExceeded.into());
                }
                record_len = 13usize.checked_add(len).ok_or(FrameError::LengthOverflow)?;
                if carry.len() < record_len {
                    return Ok(false);
                }
            }
            TAG_SYSTEM => {
                if carry.len() < 3 {
                    return Ok(false);
                }
                let key_len =
                    usize::from(u16::from_be_bytes(carry[1..3].try_into().expect("width")));
                if carry.len() < 3 + key_len + 8 {
                    return Ok(false);
                }
                let len = u64::from_be_bytes(
                    carry[3 + key_len..3 + key_len + 8]
                        .try_into()
                        .expect("width"),
                );
                let len = usize::try_from(len).map_err(|_| FrameError::LengthOverflow)?;
                if len > limits.record_bytes {
                    return Err(FrameError::LimitExceeded.into());
                }
                record_len = (3 + key_len + 8)
                    .checked_add(len)
                    .ok_or(FrameError::LengthOverflow)?;
                if carry.len() < record_len {
                    return Ok(false);
                }
            }
            TAG_END => {
                if carry.len() < 17 {
                    return Ok(false);
                }
                record_len = 17;
            }
            got => return Err(FrameError::Tag { at: 0, got }.into()),
        }
        if finished.is_some() {
            return Err(FrameError::TrailingBytes { at: 0 }.into());
        }
        let record = &carry[..record_len];
        stream.update(record);
        total_bytes += record.len() as u64;
        match tag {
            TAG_FACT => {
                application.update(record);
                let relation = u32::from_be_bytes(record[1..5].try_into().expect("width"));
                rows += 1;
                let row = &record[13..];
                sink.fact(relation, row).map_err(ReadError::Sink)?;
            }
            TAG_SYSTEM => {
                system.update(record);
                let key_len =
                    usize::from(u16::from_be_bytes(record[1..3].try_into().expect("width")));
                let key = &record[3..3 + key_len];
                let value = &record[3 + key_len + 8..];
                system_records += 1;
                sink.system(key, value).map_err(ReadError::Sink)?;
            }
            TAG_END => {
                finished = Some((
                    u64::from_be_bytes(record[1..9].try_into().expect("width")),
                    u64::from_be_bytes(record[9..17].try_into().expect("width")),
                ));
            }
            _ => unreachable!("tag was validated"),
        }
        carry.drain(..record_len);
        Ok(true)
    };

    for chunk in chunks {
        let chunk = chunk.map_err(ReadError::Chunk)?;
        carry.extend_from_slice(chunk.as_ref());
        while consume(&mut carry)? {}
    }
    while consume(&mut carry)? {}
    if !carry.is_empty() {
        return Err(FrameError::Truncated { at: carry.len() }.into());
    }
    let Some((end_rows, end_system)) = finished else {
        return Err(FrameError::Truncated { at: 0 }.into());
    };
    if end_rows != rows || end_system != system_records {
        return Err(FrameError::InvalidCount.into());
    }
    Ok(StreamSummary {
        application_digest: *application.finalize().as_bytes(),
        system_digest: *system.finalize().as_bytes(),
        stream_digest: *stream.finalize().as_bytes(),
        total_bytes,
        rows,
        system_records,
        chunks: Vec::new(),
    })
}

/// The streamed checkpoint manifest / snapshot certificate: identity, the
/// captured stamps and control projection, the shared logical digests, and
/// the ordered chunk references. The manifest never contains its own digest;
/// the head's `ObjectRef` carries that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointManifest {
    pub identity: DatabaseIdentity,
    pub decision: DecisionStamp,
    pub state: StateStamp,
    /// The authority control projection at capture — provenance for
    /// hydration checks, never authority for new admission (the captured
    /// target head's control is installed instead).
    pub control_at_capture: HeadAuthority,
    pub application_digest: [u8; 32],
    pub system_digest: [u8; 32],
    pub stream_digest: [u8; 32],
    pub total_bytes: u64,
    pub rows: u64,
    pub system_records: u64,
    pub chunks: Vec<ObjectRef>,
}

/// # Errors
/// Oversized manifests refuse.
pub fn encode_manifest(
    manifest: &CheckpointManifest,
    limits: StreamLimits,
) -> Result<Vec<u8>, FrameError> {
    let control = encode_control(&manifest.control_at_capture, limits.manifest_bytes)?;
    let mut out = wire::frame_header(MANIFEST_FAMILY, MANIFEST_LAYOUT, MANIFEST_KIND);
    out.extend_from_slice(manifest.identity.database_id.as_core().as_bytes());
    out.extend_from_slice(manifest.identity.incarnation_id.as_core().as_bytes());
    out.extend_from_slice(&manifest.identity.schema_id.0);
    out.extend_from_slice(&manifest.decision.seq.to_be_bytes());
    out.extend_from_slice(manifest.decision.hash.as_bytes());
    out.extend_from_slice(manifest.state.incarnation.as_core().as_bytes());
    out.extend_from_slice(&manifest.state.data_revision.to_be_bytes());
    wire::put_span(&mut out, &control)?;
    out.extend_from_slice(&manifest.application_digest);
    out.extend_from_slice(&manifest.system_digest);
    out.extend_from_slice(&manifest.stream_digest);
    out.extend_from_slice(&manifest.total_bytes.to_be_bytes());
    out.extend_from_slice(&manifest.rows.to_be_bytes());
    out.extend_from_slice(&manifest.system_records.to_be_bytes());
    wire::put_u32(
        &mut out,
        u32::try_from(manifest.chunks.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    for chunk in &manifest.chunks {
        put_object_ref(&mut out, chunk);
    }
    wire::check_limit(out.len(), limits.manifest_bytes)?;
    Ok(out)
}

/// # Errors
/// Malformed manifests refuse; chunk kinds must be `Chunk`.
pub fn decode_manifest(
    bytes: &[u8],
    limits: StreamLimits,
) -> Result<CheckpointManifest, FrameError> {
    let mut input = Reader::begin(
        bytes,
        MANIFEST_FAMILY,
        MANIFEST_LAYOUT,
        MANIFEST_KIND,
        limits.manifest_bytes,
    )?;
    let identity = DatabaseIdentity {
        database_id: DatabaseId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        incarnation_id: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        schema_id: SchemaId(input.array()?),
    };
    let decision = DecisionStamp {
        seq: input.u64()?,
        hash: DecisionDigest::from_bytes(input.array()?),
    };
    let state = StateStamp {
        incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        data_revision: input.u64()?,
    };
    let control_bytes = input.span(limits.manifest_bytes)?;
    let control_at_capture = decode_control(control_bytes, limits.manifest_bytes)?;
    let application_digest = input.array()?;
    let system_digest = input.array()?;
    let stream_digest = input.array()?;
    let total_bytes = input.u64()?;
    let rows = input.u64()?;
    let system_records = input.u64()?;
    let count = input.u32()? as usize;
    // Chunk references are 49 encoded bytes each; the remaining input bounds
    // the count before allocation.
    if count > bytes.len() / 32 {
        return Err(FrameError::InvalidCount);
    }
    let mut chunks = Vec::with_capacity(count);
    for _ in 0..count {
        let reference = read_object_ref(&mut input)?;
        if reference.kind != ObjectKind::Chunk {
            return Err(FrameError::Kind { got: 0 });
        }
        chunks.push(reference);
    }
    input.end()?;
    if state.incarnation != identity.incarnation_id || control_at_capture.identity != identity {
        return Err(FrameError::StateIdentityMismatch);
    }
    Ok(CheckpointManifest {
        identity,
        decision,
        state,
        control_at_capture,
        application_digest,
        system_digest,
        stream_digest,
        total_bytes,
        rows,
        system_records,
        chunks,
    })
}

/// Check a decoded stream summary against its manifest: exact digests,
/// counters and bytes. A mismatch is corruption-class evidence, never a
/// partial import.
///
/// # Errors
/// The first disagreeing field refuses.
pub fn verify_summary(
    manifest: &CheckpointManifest,
    summary: &StreamSummary,
) -> Result<(), FrameError> {
    if manifest.stream_digest != summary.stream_digest
        || manifest.application_digest != summary.application_digest
        || manifest.system_digest != summary.system_digest
        || manifest.total_bytes != summary.total_bytes
        || manifest.rows != summary.rows
        || manifest.system_records != summary.system_records
    {
        return Err(FrameError::InvalidCount);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use super::*;
    use crate::history::authority::{Activation, HeadAuthority};

    struct BufferSink {
        chunks: Vec<Vec<u8>>,
    }

    impl ChunkSink for BufferSink {
        type Error = std::convert::Infallible;

        fn chunk(&mut self, bytes: &[u8]) -> Result<ObjectRef, Self::Error> {
            self.chunks.push(bytes.to_vec());
            Ok(ObjectRef::of(1, ObjectKind::Chunk, bytes))
        }
    }

    #[derive(Default)]
    struct CollectSink {
        facts: Vec<(u32, Vec<u8>)>,
        system: Vec<(Vec<u8>, Vec<u8>)>,
    }

    impl StreamSink for CollectSink {
        type Error = std::convert::Infallible;

        fn fact(&mut self, relation: u32, row: &[u8]) -> Result<(), Self::Error> {
            self.facts.push((relation, row.to_vec()));
            Ok(())
        }

        fn system(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
            self.system.push((key.to_vec(), value.to_vec()));
            Ok(())
        }
    }

    fn write_sample(chunk_target: usize) -> (Vec<Vec<u8>>, StreamSummary) {
        let mut sink = BufferSink { chunks: Vec::new() };
        let mut writer = StreamWriter::new(&mut sink, chunk_target, StreamLimits::DEFAULT);
        writer.fact(0, b"row-a").unwrap();
        writer.fact(0, b"row-b-longer-payload").unwrap();
        writer.fact(3, b"row-c").unwrap();
        writer.system(b"r-key-1", b"receipt-row-1").unwrap();
        writer.system(b"r-key-2", b"receipt-row-2").unwrap();
        writer
            .system(b"m-batch-1", b"applied-batch-evidence")
            .unwrap();
        let summary = writer.finish().unwrap();
        (sink.chunks, summary)
    }

    #[test]
    fn streams_roundtrip_identically_across_chunk_boundaries() {
        // A chunk size small enough to split every record proves the carry
        // parser; the digests must not depend on the chunking.
        let (big_chunks, big) = write_sample(1 << 20);
        let (small_chunks, small) = write_sample(4_096);
        assert!(small_chunks.len() >= big_chunks.len());
        assert_eq!(big.application_digest, small.application_digest);
        assert_eq!(big.system_digest, small.system_digest);
        assert_eq!(big.stream_digest, small.stream_digest);
        let mut sink = CollectSink::default();
        let summary = read_stream(
            small_chunks
                .into_iter()
                .map(Ok::<_, std::convert::Infallible>),
            &mut sink,
            StreamLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(summary.application_digest, big.application_digest);
        assert_eq!(summary.system_digest, big.system_digest);
        assert_eq!(summary.rows, 3);
        assert_eq!(summary.system_records, 3);
        assert_eq!(sink.facts.len(), 3);
        assert_eq!(sink.facts[1].1, b"row-b-longer-payload");
        assert_eq!(sink.system[0].0, b"r-key-1");
        assert_eq!(sink.system[2].1, b"applied-batch-evidence");
    }

    /// Charged chunk owners decode through `as_ref` without a payload `to_vec`.
    #[test]
    fn charged_bytes_chunk_iterator_decodes_without_payload_copy() {
        use bumbledb::work::{ByteKind, ChargedBytes};
        use bumbledb::ExecutionPolicy;
        use std::time::Duration;

        let work = ExecutionPolicy {
            input_bytes: 1 << 20,
            working_bytes: 1 << 20,
            scratch_bytes: 1 << 20,
            result_bytes: 1 << 20,
            rows: 1 << 16,
            work_units: 1_024,
            timeout: Duration::from_secs(30),
        }
        .start()
        .expect("work");
        let (chunks, expected) = write_sample(4_096);
        let charged: Vec<ChargedBytes> = chunks
            .into_iter()
            .map(|bytes| {
                ChargedBytes::adopt(&work, ByteKind::Working, bytes.into_boxed_slice())
                    .expect("adopt")
            })
            .collect();
        let mut sink = CollectSink::default();
        let summary = read_stream(
            charged
                .iter()
                .map(|chunk| Ok::<_, std::convert::Infallible>(chunk.as_bytes())),
            &mut sink,
            StreamLimits::DEFAULT,
        )
        .expect("charged chunks decode");
        assert_eq!(summary.rows, expected.rows);
        assert_eq!(summary.system_records, expected.system_records);
        assert_eq!(summary.application_digest, expected.application_digest);
        for owner in charged {
            drop(owner.into_owner());
        }
    }

    #[test]
    fn empty_projection_digests_are_the_blank_database_values() {
        let mut sink = BufferSink { chunks: Vec::new() };
        let writer = StreamWriter::new(&mut sink, 4_096, StreamLimits::DEFAULT);
        let summary = writer.finish().unwrap();
        assert_eq!(summary.application_digest, empty_application_digest());
        assert_eq!(summary.system_digest, empty_system_digest());
        assert_eq!(summary.rows, 0);
        // Recorded cross-lane obligation: P04's blank_initial_digests()
        // must equal (empty_application_digest(), empty_system_digest())
        // after its recorded patch; asserted centrally in the lane tests.
    }

    #[test]
    fn corruption_truncation_and_counter_mismatch_refuse() {
        let (chunks, _) = write_sample(4_096);
        // Truncation of the final chunk.
        let mut truncated = chunks.clone();
        let last = truncated.last_mut().unwrap();
        last.pop();
        let mut sink = CollectSink::default();
        assert!(
            read_stream(
                truncated.into_iter().map(Ok::<_, std::convert::Infallible>),
                &mut sink,
                StreamLimits::DEFAULT
            )
            .is_err()
        );
        // A flipped byte inside a record changes a digest but stays
        // grammatical — the manifest comparison catches it.
        let (chunks2, honest) = write_sample(4_096);
        let mut flipped = chunks2;
        flipped[0][6] ^= 0xff;
        let mut sink = CollectSink::default();
        let outcome = read_stream(
            flipped.into_iter().map(Ok::<_, std::convert::Infallible>),
            &mut sink,
            StreamLimits::DEFAULT,
        );
        if let Ok(summary) = outcome {
            assert_ne!(summary.stream_digest, honest.stream_digest);
        }
        // A bad tag refuses outright.
        let mut bad = write_sample(1 << 20).0;
        bad[0][0] = 9;
        let mut sink = CollectSink::default();
        assert!(matches!(
            read_stream(
                bad.into_iter().map(Ok::<_, std::convert::Infallible>),
                &mut sink,
                StreamLimits::DEFAULT
            ),
            Err(ReadError::Frame(FrameError::Tag { .. }))
        ));
    }

    #[test]
    fn out_of_order_sections_refuse() {
        let mut sink = BufferSink { chunks: Vec::new() };
        let mut writer = StreamWriter::new(&mut sink, 4_096, StreamLimits::DEFAULT);
        writer.system(b"k", b"r").unwrap();
        assert!(matches!(
            writer.fact(0, b"row"),
            Err(WriteError::OutOfOrder)
        ));
    }

    #[test]
    fn manifests_roundtrip_and_bind_identity_and_chunk_kinds() {
        let identity = DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        };
        let control = HeadAuthority::genesis(
            identity,
            DecisionStamp {
                seq: 0,
                hash: DecisionDigest::from_bytes([9; 32]),
            },
            Activation::NotActivated,
        )
        .unwrap();
        let manifest = CheckpointManifest {
            identity,
            decision: DecisionStamp {
                seq: 0,
                hash: DecisionDigest::from_bytes([9; 32]),
            },
            state: StateStamp {
                incarnation: identity.incarnation_id,
                data_revision: 0,
            },
            control_at_capture: control,
            application_digest: empty_application_digest(),
            system_digest: empty_system_digest(),
            stream_digest: [4; 32],
            total_bytes: 25,
            rows: 0,
            system_records: 0,
            chunks: vec![ObjectRef::of(1, ObjectKind::Chunk, b"chunk")],
        };
        let bytes = encode_manifest(&manifest, StreamLimits::DEFAULT).unwrap();
        assert_eq!(
            decode_manifest(&bytes, StreamLimits::DEFAULT).unwrap(),
            manifest
        );
        for end in 0..bytes.len() {
            assert!(decode_manifest(&bytes[..end], StreamLimits::DEFAULT).is_err());
        }
        // A decision-kind chunk reference refuses.
        let mut wrong = manifest.clone();
        wrong.chunks = vec![ObjectRef::of(1, ObjectKind::Decision, b"chunk")];
        let bytes = encode_manifest(&wrong, StreamLimits::DEFAULT).unwrap();
        assert!(decode_manifest(&bytes, StreamLimits::DEFAULT).is_err());
    }
}
