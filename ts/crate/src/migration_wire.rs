//! The two read-only migration-codec responses (C11): bounded JSON envelope
//! plumbing over P09's native `schema_file`/`migration::{plan, manifest}`
//! lanes. Everything SEMANTIC — schema validation, canonical `SchemaId`,
//! snapshot rendering, plan parsing/validation/digesting, manifest
//! verification/appending, plan-set digests — happens in those native
//! modules; this file only reads the request envelope, re-renders embedded
//! plan/manifest subtrees to text for the native parsers, and spells the
//! response envelope. It is transport, never a second canonical codec: no
//! digest, frame or scalar judgment lives here.

use bumbledb::work::WorkContext;
use bumbledb_log::migration::manifest::{
    Manifest, ManifestError, append_entry, parse_manifest, plan_set_digest, render_manifest,
    verify_manifest,
};
use bumbledb_log::migration::plan::{Plan, PlanError, parse_plan, render_plan};
use bumbledb_log::schema_file;

use crate::runtime::RuntimeError;

/// The migration frame cap for envelope-carried plan/manifest data —
/// provisional until the F3 format freeze (C12) selects the deployment
/// envelope; a change here is a re-emit, never a reinterpretation.
pub(crate) const MIGRATION_CAP: usize = 16 << 20;

// ---------------------------------------------------------------------------
// A minimal strict JSON envelope reader/renderer. Objects, arrays, strings,
// booleans, null and NON-NEGATIVE INTEGER numbers only (the same numeric
// discipline as the native migration reader: floats and bigints are never
// bare numbers). Bounded depth; refusals are typed envelope errors.
// ---------------------------------------------------------------------------

const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Envelope {
    Null,
    Bool(bool),
    U64(u64),
    Text(String),
    Array(Vec<Envelope>),
    Object(Vec<(String, Envelope)>),
}

impl Envelope {
    pub(crate) fn get(&self, key: &str) -> Option<&Envelope> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(*value),
            _ => None,
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

type EnvelopeResult<T> = Result<T, &'static str>;

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn bump(&mut self) -> EnvelopeResult<u8> {
        let byte = self.peek().ok_or("truncated JSON")?;
        self.at += 1;
        Ok(byte)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> EnvelopeResult<()> {
        if self.bump()? == byte {
            Ok(())
        } else {
            Err("unexpected JSON token")
        }
    }

    fn literal(&mut self, text: &str, value: Envelope) -> EnvelopeResult<Envelope> {
        for expected in text.bytes() {
            if self.bump()? != expected {
                return Err("unexpected JSON literal");
            }
        }
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> EnvelopeResult<Envelope> {
        if depth > MAX_DEPTH {
            return Err("JSON deeper than the envelope bound");
        }
        self.skip_ws();
        match self.peek().ok_or("truncated JSON")? {
            b'n' => self.literal("null", Envelope::Null),
            b't' => self.literal("true", Envelope::Bool(true)),
            b'f' => self.literal("false", Envelope::Bool(false)),
            b'"' => Ok(Envelope::Text(self.string()?)),
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == Some(b']') {
                    self.at += 1;
                    return Ok(Envelope::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.skip_ws();
                    match self.bump()? {
                        b',' => {}
                        b']' => break,
                        _ => return Err("malformed JSON array"),
                    }
                }
                Ok(Envelope::Array(items))
            }
            b'{' => {
                self.at += 1;
                let mut entries: Vec<(String, Envelope)> = Vec::new();
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.at += 1;
                    return Ok(Envelope::Object(entries));
                }
                loop {
                    self.skip_ws();
                    let key = self.string()?;
                    if entries.iter().any(|(name, _)| *name == key) {
                        return Err("duplicate JSON object key");
                    }
                    self.skip_ws();
                    self.expect(b':')?;
                    let value = self.value(depth + 1)?;
                    entries.push((key, value));
                    self.skip_ws();
                    match self.bump()? {
                        b',' => {}
                        b'}' => break,
                        _ => return Err("malformed JSON object"),
                    }
                }
                Ok(Envelope::Object(entries))
            }
            b'0'..=b'9' => {
                let start = self.at;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.at += 1;
                }
                if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
                    return Err("floats are never bare JSON numbers on this envelope");
                }
                let text = std::str::from_utf8(&self.bytes[start..self.at])
                    .map_err(|_| "malformed JSON number")?;
                if text.len() > 1 && text.starts_with('0') {
                    return Err("leading zeros refuse");
                }
                text.parse::<u64>()
                    .map(Envelope::U64)
                    .map_err(|_| "JSON number exceeds u64")
            }
            b'-' => Err("negative numbers are never bare on this envelope"),
            _ => Err("unexpected JSON token"),
        }
    }

    fn string(&mut self) -> EnvelopeResult<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump()? {
                b'"' => return Ok(out),
                b'\\' => match self.bump()? {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let mut code: u32 = 0;
                        for _ in 0..4 {
                            let digit = (self.bump()? as char)
                                .to_digit(16)
                                .ok_or("malformed \\u escape")?;
                            code = code * 16 + digit;
                        }
                        // Surrogate pairs: a high surrogate must be followed
                        // by an escaped low surrogate; lone surrogates refuse.
                        if (0xd800..0xdc00).contains(&code) {
                            self.expect(b'\\')?;
                            self.expect(b'u')?;
                            let mut low: u32 = 0;
                            for _ in 0..4 {
                                let digit = (self.bump()? as char)
                                    .to_digit(16)
                                    .ok_or("malformed \\u escape")?;
                                low = low * 16 + digit;
                            }
                            if !(0xdc00..0xe000).contains(&low) {
                                return Err("lone surrogate");
                            }
                            code = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                        } else if (0xdc00..0xe000).contains(&code) {
                            return Err("lone surrogate");
                        }
                        out.push(char::from_u32(code).ok_or("invalid \\u escape")?);
                    }
                    _ => return Err("unknown escape"),
                },
                byte if byte < 0x20 => return Err("raw control byte in string"),
                byte => {
                    // Re-assemble UTF-8: the envelope arrived as checked
                    // UTF-8, so continuation bytes follow their lead.
                    let width = match byte {
                        0x00..=0x7f => 0,
                        0xc0..=0xdf => 1,
                        0xe0..=0xef => 2,
                        0xf0..=0xf7 => 3,
                        _ => return Err("malformed UTF-8"),
                    };
                    let start = self.at - 1;
                    for _ in 0..width {
                        self.bump()?;
                    }
                    let text = std::str::from_utf8(&self.bytes[start..self.at])
                        .map_err(|_| "malformed UTF-8")?;
                    out.push_str(text);
                }
            }
        }
    }
}

pub(crate) fn read_envelope(bytes: &[u8]) -> EnvelopeResult<Envelope> {
    let mut reader = Reader { bytes, at: 0 };
    let value = reader.value(0)?;
    reader.skip_ws();
    if reader.at != bytes.len() {
        return Err("trailing bytes after JSON");
    }
    Ok(value)
}

pub(crate) fn push_json_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn render_envelope(out: &mut String, value: &Envelope) {
    match value {
        Envelope::Null => out.push_str("null"),
        Envelope::Bool(true) => out.push_str("true"),
        Envelope::Bool(false) => out.push_str("false"),
        Envelope::U64(number) => out.push_str(&number.to_string()),
        Envelope::Text(text) => push_json_string(out, text),
        Envelope::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                render_envelope(out, item);
            }
            out.push(']');
        }
        Envelope::Object(entries) => {
            out.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json_string(out, key);
                out.push(':');
                render_envelope(out, item);
            }
            out.push('}');
        }
    }
}

fn subtree_text(value: &Envelope) -> String {
    let mut out = String::new();
    render_envelope(&mut out, value);
    out
}

fn hex32(bytes: &[u8; 32]) -> String {
    crate::hex_fingerprint(bytes)
}

fn hex_to_fingerprint(text: &str) -> Option<bumbledb::SchemaFingerprint> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let hex = std::str::from_utf8(pair).ok()?;
        *slot = u8::from_str_radix(hex, 16).ok()?;
    }
    // Lowercase-only: an uppercase spelling is a different (refused) token.
    if text.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return None;
    }
    Some(bumbledb::SchemaFingerprint(out))
}

// ---------------------------------------------------------------------------
// Response envelopes.
// ---------------------------------------------------------------------------

fn refused(code: &str, detail: &str) -> Vec<u8> {
    let mut out = String::from("{\"refused\":{\"code\":");
    push_json_string(&mut out, code);
    out.push_str(",\"detail\":");
    push_json_string(&mut out, detail);
    out.push_str("}}");
    out.into_bytes()
}

fn envelope_error(detail: &'static str) -> RuntimeError {
    RuntimeError::Engine {
        kind: "migrationEnvelope",
        message: detail.to_string(),
    }
}

/// `runtimeMigrationSchema`: validate + fingerprint + render one schema
/// spec through the ONE native grammar (`schema_file`), yielding
/// `{ schemaId, snapshot }` or a typed `{ refused }` row.
pub(crate) fn schema_response(
    parsed: Result<(bumbledb::SchemaDescriptor, crate::FieldAttrsTable), crate::OpenOutcome>,
    context: &WorkContext,
) -> Result<Vec<u8>, RuntimeError> {
    context.checkpoint()?;
    let descriptor = match parsed {
        Ok((descriptor, _attrs)) => descriptor,
        Err(
            crate::OpenOutcome::SchemaError(message) | crate::OpenOutcome::NewtypeMismatch(message),
        ) => {
            return Ok(refused("Misuse", &message));
        }
    };
    context.step(1 + descriptor.relations.len() as u64)?;
    let schema_id = match schema_file::schema_id(&descriptor) {
        Ok(fingerprint) => fingerprint,
        Err(error) => return Ok(refused("Misuse", &error.to_string())),
    };
    let snapshot = schema_file::render(&descriptor);
    context.input(snapshot.len() as u64)?;
    let mut out = String::from("{\"schemaId\":");
    push_json_string(&mut out, &hex32(&schema_id.0));
    out.push_str(",\"snapshot\":");
    push_json_string(&mut out, &snapshot);
    out.push('}');
    Ok(out.into_bytes())
}

fn manifest_refusal(error: &ManifestError) -> Vec<u8> {
    let code = match error {
        ManifestError::Json(_) | ManifestError::Shape(_) => "MigrationUnsupported",
        _ => "MigrationDrift",
    };
    refused(code, &format!("{error:?}"))
}

fn plan_refusal(error: &PlanError) -> Vec<u8> {
    refused("MigrationUnsupported", &format!("{error:?}"))
}

/// `runtimeMigrationRead` (`kind: "chain"`): parse + verify the manifest,
/// bind every recorded plan's canonical digest, optionally validate/append
/// one new plan (returning its canonical rendered text, the new manifest
/// text and the new entry), and optionally compute a pending plan-set
/// digest. Nothing is trusted from text; nothing here opens, initializes,
/// freezes or migrates a database.
#[allow(clippy::too_many_lines)]
pub(crate) fn chain_response(
    request: &[u8],
    context: &WorkContext,
) -> Result<Vec<u8>, RuntimeError> {
    context.checkpoint()?;
    let tree = read_envelope(request).map_err(envelope_error)?;
    if tree.get("kind").and_then(Envelope::as_str) != Some("chain") {
        return Err(envelope_error("unknown migration request kind"));
    }
    let cap = MIGRATION_CAP;
    // 1. The manifest: recorded chain text, or the declared empty base.
    let mut manifest = match tree.get("manifest") {
        Some(value) if !value.is_null() => {
            let text = subtree_text(value);
            context.input(text.len() as u64)?;
            match parse_manifest(&text, cap) {
                Ok(manifest) => manifest,
                Err(error) => return Ok(manifest_refusal(&error)),
            }
        }
        _ => {
            let base = tree
                .get("baseSchemaId")
                .and_then(Envelope::as_str)
                .ok_or_else(|| envelope_error("a fresh chain requires baseSchemaId"))?;
            let base = hex_to_fingerprint(base)
                .ok_or_else(|| envelope_error("baseSchemaId is not 64 lowercase hex"))?;
            Manifest {
                base_schema: base,
                entries: Vec::new(),
            }
        }
    };
    let head_prefix = match verify_manifest(&manifest, cap) {
        Ok(prefix) => prefix,
        Err(error) => return Ok(manifest_refusal(&error)),
    };
    // 2. Bind every recorded plan to its entry (digest recomputation —
    //    never text trust).
    let mut plans: Vec<Plan> = Vec::new();
    if let Some(Envelope::Array(items)) = tree.get("plans") {
        for item in items {
            context.step(1)?;
            let text = subtree_text(item);
            context.input(text.len() as u64)?;
            match parse_plan(&text) {
                Ok(plan) => plans.push(plan),
                Err(error) => return Ok(plan_refusal(&error)),
            }
        }
    }
    {
        let refs: Vec<&Plan> = plans.iter().collect();
        if let Err(error) = bumbledb_log::migration::manifest::bind_plans(&manifest, 0, &refs, cap)
        {
            return Ok(manifest_refusal(&error));
        }
    }
    if plans.len() != manifest.entries.len() {
        return Ok(refused(
            "MigrationDrift",
            "recorded plans and manifest entries disagree in count",
        ));
    }
    // 3. Optional append: validate the new plan and extend the chain.
    let mut head_prefix = head_prefix;
    let appended = match tree.get("append") {
        Some(value) if !value.is_null() => {
            let text = subtree_text(value);
            context.input(text.len() as u64)?;
            let plan = match parse_plan(&text) {
                Ok(plan) => plan,
                Err(error) => return Ok(plan_refusal(&error)),
            };
            let entry = match append_entry(&mut manifest, &plan, cap) {
                Ok(entry) => entry,
                Err(error) => return Ok(manifest_refusal(&error)),
            };
            head_prefix = entry.prefix_digest;
            let plan_text = render_plan(&plan);
            let manifest_text = match render_manifest(&manifest, cap) {
                Ok(text) => text,
                Err(error) => {
                    return Ok(refused("MigrationUnsupported", &format!("{error:?}")));
                }
            };
            Some((entry, plan_text, manifest_text))
        }
        _ => None,
    };
    // 4. Optional pending plan-set digest.
    let plan_set = match tree.get("planSet") {
        Some(value) if !value.is_null() => {
            let first = value
                .get("first")
                .and_then(Envelope::as_u64)
                .ok_or_else(|| envelope_error("planSet.first must be a u64"))?;
            let count = value
                .get("count")
                .and_then(Envelope::as_u64)
                .ok_or_else(|| envelope_error("planSet.count must be a u64"))?;
            let first = usize::try_from(first).map_err(|_| envelope_error("planSet.first"))?;
            let count = usize::try_from(count).map_err(|_| envelope_error("planSet.count"))?;
            match plan_set_digest(&manifest, first, count, cap) {
                Ok(digest) => Some(digest),
                Err(error) => return Ok(manifest_refusal(&error)),
            }
        }
        _ => None,
    };
    // 5. The response envelope.
    let mut out = String::from("{\"headPrefixDigest\":");
    push_json_string(&mut out, &hex32(&head_prefix));
    out.push_str(",\"planSetDigest\":");
    match plan_set {
        Some(digest) => push_json_string(&mut out, &hex32(&digest)),
        None => out.push_str("null"),
    }
    out.push_str(",\"appended\":");
    match appended {
        None => out.push_str("null"),
        Some((entry, plan_text, manifest_text)) => {
            out.push_str("{\"entry\":{\"sequence\":");
            push_json_string(&mut out, &entry.sequence.to_string());
            out.push_str(",\"id\":");
            push_json_string(&mut out, entry.label.as_str());
            out.push_str(",\"fromSchemaId\":");
            push_json_string(&mut out, &hex32(&entry.from_schema.0));
            out.push_str(",\"toSchemaId\":");
            push_json_string(&mut out, &hex32(&entry.to_schema.0));
            out.push_str(",\"planDigest\":");
            push_json_string(&mut out, &hex32(&entry.plan_digest));
            out.push_str(",\"prefixDigest\":");
            push_json_string(&mut out, &hex32(&entry.prefix_digest));
            out.push_str("},\"planText\":");
            push_json_string(&mut out, &plan_text);
            out.push_str(",\"manifestText\":");
            push_json_string(&mut out, &manifest_text);
            out.push('}');
        }
    }
    out.push('}');
    context.input(out.len() as u64)?;
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb_log::migration::manifest::base_prefix_digest;

    #[test]
    fn envelope_reader_is_strict_and_total_over_the_request_grammar() {
        // The exact shapes the generator sends: objects, arrays, strings,
        // u64 numbers, null. Everything else refuses typed.
        let tree = read_envelope(
            br#"{"kind":"chain","manifest":null,"baseSchemaId":"ab","plans":[],"append":null,"planSet":{"first":0,"count":1}}"#,
        )
        .expect("the request grammar parses");
        assert_eq!(tree.get("kind").and_then(Envelope::as_str), Some("chain"));
        assert!(tree.get("manifest").expect("key").is_null());
        assert_eq!(
            tree.get("planSet")
                .and_then(|set| set.get("count"))
                .and_then(Envelope::as_u64),
            Some(1)
        );
        assert!(read_envelope(b"{\"a\":1.5}").is_err(), "floats refuse");
        assert!(read_envelope(b"{\"a\":-1}").is_err(), "negatives refuse");
        assert!(
            read_envelope(b"{\"a\":01}").is_err(),
            "leading zeros refuse"
        );
        assert!(
            read_envelope(b"{\"a\":1,\"a\":2}").is_err(),
            "dup keys refuse"
        );
        assert!(read_envelope(b"{} trailing").is_err(), "trailing refuses");
    }

    #[test]
    fn string_escapes_round_trip_through_the_renderer() {
        let text = "a\"b\\c\nd\u{1}e\u{1F600}";
        let mut rendered = String::new();
        push_json_string(&mut rendered, text);
        let parsed = read_envelope(rendered.as_bytes()).expect("rendered string parses");
        assert_eq!(parsed.as_str(), Some(text));
    }

    #[test]
    fn fresh_chain_reports_the_base_prefix_digest() {
        // A fresh chain (no manifest) must answer with the base prefix of
        // the declared empty-base schema — the same digest the native
        // manifest module derives.
        let base = bumbledb::SchemaFingerprint([7; 32]);
        let expected = base_prefix_digest(&base, MIGRATION_CAP).expect("base prefix");
        let request = format!(
            r#"{{"kind":"chain","manifest":null,"baseSchemaId":"{}","plans":[],"append":null,"planSet":null}}"#,
            hex32(&base.0)
        );
        let work = bumbledb::work::ExecutionPolicy {
            input_bytes: 1 << 20,
            working_bytes: 1 << 20,
            scratch_bytes: 1 << 20,
            result_bytes: 1 << 20,
            rows: 1 << 20,
            work_units: 1 << 20,
            timeout: std::time::Duration::from_secs(5),
        }
        .start()
        .expect("work context");
        let response = chain_response(request.as_bytes(), &work).expect("chain response");
        let tree = read_envelope(&response).expect("response parses");
        assert_eq!(
            tree.get("headPrefixDigest").and_then(Envelope::as_str),
            Some(hex32(&expected).as_str())
        );
        assert!(tree.get("planSetDigest").expect("key").is_null());
        assert!(tree.get("appended").expect("key").is_null());
    }
}
