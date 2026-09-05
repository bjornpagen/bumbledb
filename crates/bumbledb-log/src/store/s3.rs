//! `S3Store`: the conditional-store verbs over one S3 bucket/prefix. `SigV4`
//! and the conditional headers ride the `object_store` crate; this module
//! maps vendor outcomes onto the three-way conditional grammar and nothing
//! else. Certainty comes ONLY from the typed outcome of the conditional
//! operation: a typed success is publication, a typed precondition failure is
//! a proved loss, and EVERYTHING else after dispatch — 409 conflicts,
//! timeouts, connection resets, 5xx, auth refusals, lost response bodies —
//! is `Indeterminate` (potentially applied). There is no substring matching
//! of error text; certainty-by-substring is not a qualified service
//! contract. Pre-dispatch refusals (key grammar, foreign async context)
//! remain `Err` because they are provably raised before any request is sent.
//! Conditional writes are never retried by the transport. Credentials are
//! consulted per request, off the worker threads. `ETags` stay opaque
//! version tokens.
//!
//! Production qualification is for a specific AWS S3 configuration (region,
//! bucket class, strong read-after-write, conditional replacement, IAM
//! separation) exercised by the real-credential F3 lane; emulator green is
//! not S3 qualification (C07).

use std::io;
use std::sync::Arc;

use object_store::aws::{AmazonS3, AmazonS3Builder, AwsCredential};
use object_store::path::Path;
use object_store::{
    CredentialProvider, Error as ObjError, GetOptions, ObjectStore as _, PutMode, PutOptions,
    RetryConfig, UpdateVersion,
};
use tokio::runtime::{Builder, Handle, Runtime};
use std::sync::OnceLock;

use super::key_ok;
use super::receive::{
    ObservedError, ReceiveAccumulator, ReceiveFault, ReceiveLimits, ReceivedBody, ReceivedHead,
    ReceivingStore, TransportContext, TransportObservation,
};
use crate::writer::verbs::{
    ConditionalOutcome, ConditionalStore, HeadVersion, ListPage, PutOutcome,
};
use object_store::list::{PaginatedListOptions, PaginatedListStore};

/// One shared bounded I/O runtime for all S3 stores with the same transport
/// configuration. Tenant authority stays separate; only the executor is shared
/// (LOG-023).
static SHARED_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

fn shared_runtime() -> Result<Arc<Runtime>, S3Error> {
    if Handle::try_current().is_ok() {
        return Err(S3Error::new(
            "open",
            String::new(),
            io::Error::other("S3Store is constructed outside an async context"),
        ));
    }
    SHARED_RUNTIME
        .get_or_try_init(|| {
            Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .map(Arc::new)
                .map_err(|source| S3Error::new("open", String::new(), source))
        })
        .map(Arc::clone)
}

/// Static keys, or a caller-owned refresh the store invokes before each
/// signed request (env, IMDS, SSO, secrets manager, host rotation).
pub enum S3Credentials {
    Static {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
    Refresh(Arc<dyn Fn() -> io::Result<StaticKeys> + Send + Sync>),
}

/// The three values a refresh must produce. `session_token` is `None` for
/// long-lived keys.
#[derive(Clone)]
pub struct StaticKeys {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// One constructor: endpoint, region, bucket, credentials. `endpoint` is
/// `None` for AWS's regional virtual-host. `region: "auto"` without an
/// endpoint is refused — R2's `auto` rides the endpoint arm.
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub credentials: S3Credentials,
}

/// The S3 backend's infrastructure failure: transport, auth, stream bodies.
/// Observations are transport facts; they are never a publication verdict.
#[derive(Debug)]
pub struct S3Error {
    pub op: &'static str,
    pub key: String,
    pub source: io::Error,
    pub observation: TransportObservation,
}

impl ObservedError for S3Error {
    fn observation(&self) -> TransportObservation {
        self.observation
    }
}

impl S3Error {
    fn new(op: &'static str, key: impl Into<String>, source: io::Error) -> Self {
        let observation = observe_io(&source);
        Self {
            op,
            key: key.into(),
            source,
            observation,
        }
    }

    fn observed(
        op: &'static str,
        key: impl Into<String>,
        source: io::Error,
        observation: TransportObservation,
    ) -> Self {
        Self {
            op,
            key: key.into(),
            source,
            observation,
        }
    }
}

fn observe_io(error: &io::Error) -> TransportObservation {
    match error.kind() {
        io::ErrorKind::NotFound => TransportObservation::Missing,
        io::ErrorKind::PermissionDenied => TransportObservation::Denied,
        _ => TransportObservation::Indeterminate,
    }
}

fn observe_obj(error: &ObjError) -> TransportObservation {
    match error {
        ObjError::NotFound { .. } => TransportObservation::Missing,
        ObjError::PermissionDenied { .. } | ObjError::Unauthenticated { .. } => {
            TransportObservation::Denied
        }
        ObjError::Precondition { .. } | ObjError::NotModified { .. } => {
            TransportObservation::Precondition
        }
        ObjError::AlreadyExists { .. } => TransportObservation::Conflict,
        // Generic 5xx/reset/timeout/bucket/region payloads stay Indeterminate:
        // substring matching is not a service contract, and L08 must not
        // treat this as a publication verdict.
        _ => TransportObservation::Indeterminate,
    }
}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s3 store {} on `{}`: {}", self.op, self.key, self.source)
    }
}

impl std::error::Error for S3Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// The verbs against one bucket. Construct and call outside an async
/// context — every verb drives a dedicated multi-thread runtime and returns
/// `Err` rather than `block_on` from a foreign context.
pub struct S3Store {
    inner: AmazonS3,
    handle: Handle,
    _runtime: Arc<Runtime>,
}

impl S3Store {
    /// # Errors
    /// Refuses construction inside an async context, `region: "auto"`
    /// without an endpoint, and client-build failure. No network is touched.
    pub fn new(config: &S3Config) -> Result<Self, S3Error> {
        if Handle::try_current().is_ok() {
            return Err(S3Error::new(
                "open",
                config.bucket.clone(),
                io::Error::other("S3Store is constructed outside an async context"),
            ));
        }
        if config.region == "auto" && config.endpoint.is_none() {
            return Err(S3Error::observed(
                "open",
                config.region.clone(),
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "region auto requires an endpoint",
                ),
                TransportObservation::Region,
            ));
        }
        let runtime = shared_runtime()?;
        let handle = runtime.handle().clone();
        let inner = build_client(config)?;
        Ok(Self {
            inner,
            handle,
            _runtime: runtime,
        })
    }

    fn path_of(op: &'static str, key: &str) -> Result<Path, S3Error> {
        if !key_ok(key) {
            return Err(S3Error::new(
                op,
                key,
                io::Error::new(io::ErrorKind::InvalidInput, "invalid store key"),
            ));
        }
        Path::parse(key).map_err(|source| S3Error::new(op, key, io::Error::other(source)))
    }

    /// The verbs are the sync surface; refuse rather than `block_on` from a
    /// foreign async context.
    fn block<T>(
        &self,
        op: &'static str,
        key: &str,
        fut: impl std::future::Future<Output = T>,
    ) -> Result<T, S3Error> {
        if Handle::try_current().is_ok() {
            return Err(S3Error::new(
                op,
                key,
                io::Error::other("store verbs are synchronous"),
            ));
        }
        Ok(self.handle.block_on(fut))
    }
}

fn build_client(config: &S3Config) -> Result<AmazonS3, S3Error> {
    let mut builder = AmazonS3Builder::new()
        .with_region(&config.region)
        .with_bucket_name(&config.bucket)
        .with_conditional_put(object_store::aws::S3ConditionalPut::ETagMatch)
        // Conditional writes are never blindly retried by the transport: a
        // retry after a timeout could double-report a unique transition.
        .with_retry(RetryConfig {
            max_retries: 0,
            ..RetryConfig::default()
        });
    match &config.endpoint {
        Some(endpoint) => {
            builder = builder
                .with_endpoint(endpoint)
                .with_virtual_hosted_style_request(false)
                .with_allow_http(endpoint.starts_with("http://"));
        }
        None => {
            builder = builder.with_virtual_hosted_style_request(true);
        }
    }
    builder = builder.with_credentials(Arc::new(request_credentials(&config.credentials)));
    builder
        .build()
        .map_err(|source| S3Error::new("open", config.bucket.clone(), io::Error::other(source)))
}

/// One provider for both credential arms. `get_credential` consults the arm
/// on every sign; the Refresh callback runs on `spawn_blocking` so blocking
/// I/O never occupies a tokio worker.
fn request_credentials(credentials: &S3Credentials) -> RefreshProvider {
    match credentials {
        S3Credentials::Static {
            access_key_id,
            secret_access_key,
            session_token,
        } => {
            let keys = StaticKeys {
                access_key_id: access_key_id.clone(),
                secret_access_key: secret_access_key.clone(),
                session_token: session_token.clone(),
            };
            RefreshProvider {
                refresh: Arc::new(move || Ok(keys.clone())),
            }
        }
        S3Credentials::Refresh(refresh) => RefreshProvider {
            refresh: Arc::clone(refresh),
        },
    }
}

struct RefreshProvider {
    refresh: Arc<dyn Fn() -> io::Result<StaticKeys> + Send + Sync>,
}

impl std::fmt::Debug for RefreshProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("S3CredentialRefresh")
    }
}

impl CredentialProvider for RefreshProvider {
    type Credential = AwsCredential;

    fn get_credential<'a, 'async_trait>(
        &'a self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = object_store::Result<Arc<AwsCredential>>> + Send + 'a>,
    >
    where
        'a: 'async_trait,
        Self: 'async_trait,
    {
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            let keys = tokio::task::spawn_blocking(move || refresh())
                .await
                .map_err(|source| ObjError::Generic {
                    store: "S3",
                    source: Box::new(source),
                })?
                .map_err(|source| ObjError::Generic {
                    store: "S3",
                    source: Box::new(source),
                })?;
            Ok(Arc::new(AwsCredential {
                key_id: keys.access_key_id,
                secret_key: keys.secret_access_key,
                token: keys.session_token,
            }))
        })
    }
}

fn infra(op: &'static str, key: &str, source: ObjError) -> S3Error {
    let observation = observe_obj(&source);
    S3Error::observed(op, key, io::Error::other(source), observation)
}

/// The typed post-dispatch verdict for a conditional head CREATE. Only the
/// typed variant of the conditional operation may claim certainty, and the
/// crate remaps BOTH a proved 412/304 ("a head exists") AND an unproved 409
/// (a concurrent conditional write raced this one) onto `AlreadyExists`
/// (`object_store::client::retry` maps `CONFLICT` there, and the AWS create
/// arm remaps `Precondition`/`NotModified` there) — so the variant cannot
/// prove a loss. Every dispatched error is held `Indeterminate`; the
/// publication machine resolves creation certainty by reading the head back
/// and comparing evidence, never by status guessing.
fn create_verdict(_error: &ObjError) -> ConditionalOutcome {
    ConditionalOutcome::Indeterminate
}

/// The typed post-dispatch verdict for a conditional head REPLACE.
/// `Precondition` is the typed 412 — a proved loss (the crate also remaps
/// real S3's 404-under-`If-Match` onto it; a bare `NotFound` from another
/// backend is the same proof: the conditioned version cannot match a missing
/// head). Everything else after dispatch — the 409 `AlreadyExists` remap,
/// 5xx, timeouts, resets, auth refusals — is `Indeterminate`: the CAS may
/// have been applied.
fn replace_verdict(error: &ObjError) -> ConditionalOutcome {
    match error {
        ObjError::Precondition { .. } | ObjError::NotFound { .. } => {
            ConditionalOutcome::PreconditionFailed
        }
        _ => ConditionalOutcome::Indeterminate,
    }
}

/// A 2xx that arrived without its transition token cannot prove the exact
/// version chain a conditional successor must condition on; hold it as
/// `Indeterminate` and let the machine resolve by evidence.
fn conditional_success(raw: Option<String>) -> ConditionalOutcome {
    match raw {
        Some(etag) => ConditionalOutcome::Published {
            version: HeadVersion(Box::from(etag.into_bytes())),
        },
        None => ConditionalOutcome::Indeterminate,
    }
}

fn version_of(op: &'static str, key: &str, raw: Option<String>) -> Result<HeadVersion, S3Error> {
    raw.map(|etag| HeadVersion(Box::from(etag.into_bytes())))
        .ok_or_else(|| {
            S3Error::new(
                op,
                key,
                io::Error::new(io::ErrorKind::InvalidData, "vendor omitted ETag"),
            )
        })
}

fn etag_text(version: &HeadVersion) -> String {
    String::from_utf8_lossy(&version.0).into_owned()
}

async fn stream_get_capped(
    op: &'static str,
    key: &str,
    ctx: TransportContext<'_>,
    result: object_store::GetResult,
) -> Result<ReceivedBody, S3Error> {
    // Content-Length / meta.size is not a receiving bound (C6).
    use futures::StreamExt as _;
    let mut stream = result.into_stream();
    let mut acc = ReceiveAccumulator::new(ctx);
    while let Some(chunk) = stream.next().await {
        acc.checkpoint().map_err(|fault| s3_fault(op, key, fault))?;
        let chunk = chunk.map_err(|source| infra(op, key, source))?;
        if let Err(fault) = acc.push(&chunk) {
            return Err(s3_fault(op, key, fault));
        }
    }
    acc.finish().map_err(|fault| s3_fault(op, key, fault))
}

fn s3_fault(op: &'static str, key: &str, fault: ReceiveFault) -> S3Error {
    let observation = fault.observation();
    S3Error::observed(op, key, fault.into_io(key), observation)
}

impl ReceivingStore for S3Store {
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, S3Error> {
        let path = Self::path_of("receive_object", key)?;
        self.block("receive_object", key, async {
            ctx.checkpoint()
                .map_err(|source| {
                    S3Error::new(
                        "receive_object",
                        key,
                        io::Error::new(io::ErrorKind::TimedOut, format!("{source:?}")),
                    )
                })?;
            match self.inner.get_opts(&path, GetOptions::new()).await {
                Ok(result) => stream_get_capped("receive_object", key, ctx, result).await,
                Err(ObjError::NotFound { .. }) => Err(S3Error::observed(
                    "receive_object",
                    key,
                    io::Error::new(io::ErrorKind::NotFound, "object missing"),
                    TransportObservation::Missing,
                )),
                Err(source) => Err(infra("receive_object", key, source)),
            }
        })?
    }

    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, S3Error> {
        let path = Self::path_of("receive_head", head_key)?;
        self.block("receive_head", head_key, async {
            ctx.checkpoint()
                .map_err(|source| {
                    S3Error::new(
                        "receive_head",
                        head_key,
                        io::Error::new(io::ErrorKind::TimedOut, format!("{source:?}")),
                    )
                })?;
            match self.inner.get_opts(&path, GetOptions::new()).await {
                Ok(result) => {
                    let etag = result.meta.e_tag.clone();
                    let bytes = stream_get_capped("receive_head", head_key, ctx, result).await?;
                    let version = version_of("receive_head", head_key, etag)?;
                    Ok(ReceivedHead::Present {
                        version,
                        body: bytes,
                    })
                }
                Err(ObjError::NotFound { .. }) => Ok(ReceivedHead::Absent),
                Err(source) => Err(infra("receive_head", head_key, source)),
            }
        })?
    }
}

impl ConditionalStore for S3Store {
    type Error = S3Error;

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, S3Error> {
        let path = Self::path_of("create_head", head_key)?;
        let payload = body.to_vec();
        self.block("create_head", head_key, async {
            let opts = PutOptions {
                mode: PutMode::Create,
                ..PutOptions::default()
            };
            match self.inner.put_opts(&path, payload.into(), opts).await {
                Ok(result) => Ok(conditional_success(result.e_tag)),
                Err(source) => Ok(create_verdict(&source)),
            }
        })?
    }

    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, S3Error> {
        let path = Self::path_of("replace_head", head_key)?;
        let payload = body.to_vec();
        let etag = etag_text(expected);
        self.block("replace_head", head_key, async {
            let opts = PutOptions {
                mode: PutMode::Update(UpdateVersion {
                    e_tag: Some(etag),
                    version: None,
                }),
                ..PutOptions::default()
            };
            match self.inner.put_opts(&path, payload.into(), opts).await {
                Ok(result) => Ok(conditional_success(result.e_tag)),
                Err(source) => Ok(replace_verdict(&source)),
            }
        })?
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, S3Error> {
        let path = Self::path_of("put_object", key)?;
        let payload = body.to_vec();
        self.block("put_object", key, async {
            match self.inner.put(&path, payload.into()).await {
                Ok(_) => Ok(PutOutcome::Stored),
                // Immutable objects resolve certainty by content read-back
                // (`put_verified`): any dispatched failure is unproved, never
                // a claimed non-store.
                Err(_) => Ok(PutOutcome::Indeterminate),
            }
        })?
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, S3Error> {
        const PAGE: usize = 1_000;
        let resume = after.map(|token| String::from_utf8_lossy(token).into_owned());
        // Durable progress is the last fully processed canonical key, never
        // an opaque provider page token (C6).
        let keys: Vec<String> = self.block("list_objects", prefix, async {
            let listed = self
                .inner
                .list_paginated(
                    Some(prefix),
                    PaginatedListOptions {
                        offset: resume.clone(),
                        max_keys: Some(PAGE),
                        page_token: None,
                        ..PaginatedListOptions::default()
                    },
                )
                .await
                .map_err(|source| infra("list_objects", prefix, source))?;
            let mut keys: Vec<String> = listed
                .result
                .objects
                .into_iter()
                .map(|object| object.location.to_string())
                .filter(|key| key.starts_with(prefix))
                .filter(|key| resume.as_deref().is_none_or(|resume| key.as_str() > resume))
                .collect();
            keys.sort();
            keys.truncate(PAGE);
            Ok::<_, S3Error>(keys)
        })??;
        let next = if keys.len() == PAGE {
            keys.last().map(|last| Box::from(last.as_bytes()))
        } else {
            None
        };
        Ok(ListPage { keys, next })
    }

    fn delete_object(&self, key: &str) -> Result<(), S3Error> {
        let path = Self::path_of("delete_object", key)?;
        self.block("delete_object", key, async {
            match self.inner.delete(&path).await {
                Ok(()) | Err(ObjError::NotFound { .. }) => Ok(()),
                Err(source) => Err(infra("delete_object", key, source)),
            }
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_keys() -> S3Credentials {
        S3Credentials::Static {
            access_key_id: "AKIAEXAMPLE".into(),
            secret_access_key: "secret".into(),
            session_token: None,
        }
    }

    fn config() -> S3Config {
        S3Config {
            endpoint: Some("http://127.0.0.1:1".into()),
            region: "us-east-1".into(),
            bucket: "bucket".into(),
            credentials: static_keys(),
        }
    }

    #[test]
    fn constructor_builds_without_touching_the_network() {
        let store = S3Store::new(&config()).expect("build");
        drop(store);
    }

    #[test]
    fn constructor_refuses_region_auto_without_an_endpoint() {
        let bad = S3Config {
            endpoint: None,
            region: "auto".into(),
            ..config()
        };
        assert!(S3Store::new(&bad).is_err());
    }

    #[test]
    fn constructor_and_verbs_refuse_inside_an_async_context() {
        let store = S3Store::new(&config()).expect("build outside");
        let runtime = Builder::new_current_thread().build().unwrap();
        runtime.block_on(async {
            assert!(S3Store::new(&config()).is_err());
            assert!(store
                .receive_head("t/HEAD", TransportContext::limited(64))
                .is_err());
            assert!(store
                .receive_object("t/objects/1/chunk/aa", TransportContext::limited(64))
                .is_err());
        });
    }

    #[test]
    fn hostile_keys_refuse_before_any_request() {
        let store = S3Store::new(&config()).expect("build");
        for key in ["~tmp/x", "a/../b", "a//b", "x.lock"] {
            assert!(
                store
                    .receive_object(key, TransportContext::limited(64))
                    .is_err(),
                "{key}"
            );
        }
    }

    #[test]
    fn replace_certainty_is_typed_and_never_substring_matched() {
        // The typed 412 (and the crate's 404-under-If-Match remap) is the ONE
        // proved loss.
        let precondition = ObjError::Precondition {
            path: "p".into(),
            source: Box::new(io::Error::other("412")),
        };
        assert_eq!(
            replace_verdict(&precondition),
            ConditionalOutcome::PreconditionFailed
        );
        let missing = ObjError::NotFound {
            path: "p".into(),
            source: Box::new(io::Error::other("404")),
        };
        assert_eq!(
            replace_verdict(&missing),
            ConditionalOutcome::PreconditionFailed
        );
        // EVERYTHING else after dispatch is potentially published: a
        // 5xx-shaped transport failure, a lost/reset connection, a timeout,
        // an auth refusal, and the crate's 409 -> AlreadyExists remap. None
        // of these may claim the CAS did not apply — and none of the
        // classification consults error text.
        let five_hundred = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::other(
                "Server returned non-2xx status code: 500 Internal Error",
            )),
        };
        assert_eq!(
            replace_verdict(&five_hundred),
            ConditionalOutcome::Indeterminate
        );
        let reset = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "error sending request",
            )),
        };
        assert_eq!(replace_verdict(&reset), ConditionalOutcome::Indeterminate);
        let timeout = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::new(io::ErrorKind::TimedOut, "timed out")),
        };
        assert_eq!(replace_verdict(&timeout), ConditionalOutcome::Indeterminate);
        let denied = ObjError::PermissionDenied {
            path: "p".into(),
            source: Box::new(io::Error::other("403 Forbidden")),
        };
        assert_eq!(replace_verdict(&denied), ConditionalOutcome::Indeterminate);
        let conflict = ObjError::AlreadyExists {
            path: "p".into(),
            source: Box::new(io::Error::other("Client error with status 409 Conflict")),
        };
        assert_eq!(replace_verdict(&conflict), ConditionalOutcome::Indeterminate);
    }

    #[test]
    fn create_certainty_resolves_by_evidence_not_status_guessing() {
        // The crate remaps BOTH the proved 412 and the unproved 409 onto
        // `AlreadyExists` for a conditional create, so the variant cannot
        // prove a loss: every dispatched create error is held Indeterminate
        // and the machine resolves by reading the head back.
        let exists = ObjError::AlreadyExists {
            path: "p".into(),
            source: Box::new(io::Error::other("412 or 409 — the variant cannot say")),
        };
        assert_eq!(create_verdict(&exists), ConditionalOutcome::Indeterminate);
        let five_hundred = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::other("503 Slow Down")),
        };
        assert_eq!(
            create_verdict(&five_hundred),
            ConditionalOutcome::Indeterminate
        );
    }

    #[test]
    fn a_conditional_success_without_its_transition_token_is_indeterminate() {
        assert!(matches!(
            conditional_success(Some("etag".into())),
            ConditionalOutcome::Published { .. }
        ));
        assert_eq!(conditional_success(None), ConditionalOutcome::Indeterminate);
    }

    #[test]
    fn typed_transport_errors_are_observations_never_publication() {
        let missing = ObjError::NotFound {
            path: "p".into(),
            source: Box::new(io::Error::other("404")),
        };
        assert_eq!(observe_obj(&missing), TransportObservation::Missing);
        let denied = ObjError::PermissionDenied {
            path: "p".into(),
            source: Box::new(io::Error::other("403")),
        };
        assert_eq!(observe_obj(&denied), TransportObservation::Denied);
        let generic = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::other("NoSuchBucket")),
        };
        assert_eq!(
            observe_obj(&generic),
            TransportObservation::Indeterminate,
            "generic payloads are not guessed into bucket/region/publication"
        );
        assert_eq!(replace_verdict(&denied), ConditionalOutcome::Indeterminate);
    }

    #[test]
    fn credential_refresh_runs_on_the_shared_runtime_before_a_signed_request() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let calls = Arc::new(AtomicU64::new(0));
        let refresh = {
            let calls = Arc::clone(&calls);
            Arc::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(StaticKeys {
                    access_key_id: "AKIAEXAMPLE".into(),
                    secret_access_key: "secret".into(),
                    session_token: None,
                })
            })
        };
        let store = S3Store::new(&S3Config {
            endpoint: Some("http://127.0.0.1:1".into()),
            region: "us-east-1".into(),
            bucket: "bucket".into(),
            credentials: S3Credentials::Refresh(refresh),
        })
        .expect("build");
        let _ = store.receive_object(
            "t/objects/1/chunk/aa",
            TransportContext {
                work: None,
                receive: ReceiveLimits::capped(16),
            },
        );
        assert!(
            calls.load(Ordering::SeqCst) >= 1,
            "refresh is consulted per signed request, not cached as a per-tenant executor"
        );
    }
}
