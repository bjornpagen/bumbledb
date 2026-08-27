//! `S3Store`: the five verbs over one S3-compatible target. `SigV4` and
//! the conditional headers ride the `object_store` crate; this module
//! maps their outcomes onto the trait's sums and nothing else. A 409
//! is `Ambiguous`, never a proved `Exists` or `Moved`. Conditional
//! writes are not retried by the transport. Credentials are consulted
//! per request, off the worker threads. Body-stream failures ride
//! `StoreError`. The fencing token on a [`Fenced`] write is object
//! metadata: create records it as the generation a later swap can
//! lose to, and `body.token <` that stored generation is `Moved`.

use std::borrow::Cow;
use std::io;
use std::sync::Arc;

use object_store::aws::{AmazonS3, AmazonS3Builder, AwsCredential};
use object_store::path::Path;
use object_store::{
    Attribute, Attributes, CredentialProvider, Error as ObjError, GetOptions, ObjectStore as _,
    ObjectStoreExt as _, PutMode, PutOptions, RetryConfig, UpdateVersion,
};
use tokio::runtime::{Builder, Handle, Runtime};

use super::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result, StoreError, StoreKey, Swap,
    parse_prefix, prove_create, prove_swap,
};

/// Static keys, or a caller-owned refresh the store invokes before
/// each signed request. The refresh is `dyn` because the caller owns
/// open-ended credential behavior (env, IMDS, SSO, secrets manager,
/// host rotation); a generic on the store and a bare function pointer
/// both refuse that. The boxed future is the foreign `CredentialProvider`
/// shape, not a second house abstraction.
pub enum S3Credentials {
    Static {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
    Refresh(Arc<dyn Fn() -> io::Result<StaticKeys> + Send + Sync>),
}

/// The three values a refresh must produce. `session_token` is `None`
/// for long-lived keys.
#[derive(Clone)]
pub struct StaticKeys {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// One constructor: endpoint, region, bucket, credentials, key prefix.
/// `endpoint` is `None` for AWS's regional virtual-host. `region: "auto"`
/// without an endpoint is refused — R2's `auto` rides the endpoint arm.
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub credentials: S3Credentials,
    pub prefix: String,
}

/// The five verbs against one bucket. Clone shares the client and the
/// runtime; two clones are two HTTP clients over one prefix. Construct
/// and call outside an async context — every verb drives a dedicated
/// multi-thread runtime and returns `Err` rather than `block_on` from
/// an async context.
#[derive(Clone)]
pub struct S3Store {
    inner: AmazonS3,
    prefix: String,
    handle: Handle,
    _runtime: Arc<Runtime>,
}

impl S3Store {
    /// # Errors
    pub fn new(config: &S3Config) -> Result<Self> {
        if Handle::try_current().is_ok() {
            return Err(StoreError {
                op: "open",
                key: config.bucket.clone(),
                source: io::Error::other("S3Store is constructed outside an async context"),
            });
        }
        if config.region == "auto" && config.endpoint.is_none() {
            return Err(StoreError {
                op: "open",
                key: config.region.clone(),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "region auto requires an endpoint",
                ),
            });
        }
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|source| StoreError {
                op: "open",
                key: config.bucket.clone(),
                source,
            })?;
        let handle = runtime.handle().clone();
        let prefix = parse_prefix(&config.prefix).map_err(|err| StoreError {
            op: "open",
            key: config.prefix.clone(),
            source: io::Error::new(io::ErrorKind::InvalidInput, err),
        })?;
        let inner = build_client(config)?;
        Ok(Self {
            inner,
            prefix,
            handle,
            _runtime: Arc::new(runtime),
        })
    }

    fn object_path(&self, key: &StoreKey) -> Result<Path> {
        let raw = join_prefix(&self.prefix, key.as_str());
        Path::parse(&raw).map_err(|source| StoreError {
            op: "path",
            key: raw,
            source: io::Error::other(source),
        })
    }

    /// The verbs are the sync surface. Construction and every verb
    /// return `Err` when this thread is inside an async context, so
    /// the dedicated runtime never `block_on`s from a foreign context.
    fn block<T>(&self, fut: impl std::future::Future<Output = T>) -> Result<T> {
        if Handle::try_current().is_ok() {
            return Err(StoreError {
                op: "block",
                key: self.prefix.clone(),
                source: io::Error::other("store verbs are synchronous"),
            });
        }
        Ok(self.handle.block_on(fut))
    }

    /// Delete every object under this store's prefix. The smoke lane's
    /// bucket cleanup.
    ///
    /// # Errors
    pub fn sweep_prefix(&self) -> Result<u64> {
        let raw = if self.prefix.is_empty() {
            return Err(StoreError {
                op: "sweep",
                key: String::new(),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "refuse to sweep an empty prefix",
                ),
            });
        } else {
            self.prefix.clone()
        };
        let path = Path::parse(&raw).map_err(|source| StoreError {
            op: "sweep",
            key: raw,
            source: io::Error::other(source),
        })?;
        self.block(async { sweep_listed(&self.inner, &path).await })?
    }
}

async fn sweep_listed(inner: &AmazonS3, prefix: &Path) -> Result<u64> {
    let listed = inner
        .list_with_delimiter(Some(prefix))
        .await
        .map_err(|source| StoreError {
            op: "sweep",
            key: prefix.as_ref().to_string(),
            source: io::Error::other(source),
        })?;
    let mut n = 0u64;
    for object in listed.objects {
        match inner.delete(&object.location).await {
            Ok(()) | Err(ObjError::NotFound { .. }) => n += 1,
            Err(source) => {
                return Err(StoreError {
                    op: "sweep",
                    key: object.location.to_string(),
                    source: io::Error::other(source),
                });
            }
        }
    }
    for common in listed.common_prefixes {
        n = n.saturating_add(Box::pin(sweep_listed(inner, &common)).await?);
    }
    Ok(n)
}

fn build_client(config: &S3Config) -> Result<AmazonS3> {
    let mut builder = AmazonS3Builder::new()
        .with_region(&config.region)
        .with_bucket_name(&config.bucket)
        .with_conditional_put(object_store::aws::S3ConditionalPut::ETagMatch)
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
    // Both arms resolve at sign time. Static is a closure that
    // returns the same keys; Refresh is the caller-owned callback.
    // Neither set is stored on a worker for the life of the client.
    builder = builder.with_credentials(Arc::new(request_credentials(&config.credentials)));
    builder.build().map_err(|source| StoreError {
        op: "open",
        key: config.bucket.clone(),
        source: io::Error::other(source),
    })
}

/// One provider for both credential arms. `get_credential` consults
/// the arm on every sign; the Refresh callback runs on `spawn_blocking`
/// so blocking I/O never occupies a tokio worker.
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

fn join_prefix(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}/{key}")
    }
}

fn infra(op: &'static str, key: &StoreKey, source: ObjError) -> StoreError {
    StoreError {
        op,
        key: key.to_string(),
        source: io::Error::other(source),
    }
}

/// A body-stream failure is an infrastructure error, never a raw
/// vendor error leaking past the store channel.
fn stream_err(op: &'static str, key: &StoreKey, source: ObjError) -> StoreError {
    infra(op, key, source)
}

fn etag_of(op: &'static str, key: &StoreKey, raw: Option<String>) -> Result<Etag> {
    raw.map(Etag).ok_or_else(|| StoreError {
        op,
        key: key.to_string(),
        source: io::Error::new(io::ErrorKind::InvalidData, "vendor omitted ETag"),
    })
}

/// User-metadata key the fencing generation rides. Create writes it;
/// swap If-Match-heads it and refuses `body.token <` the stored value.
const GENERATION_META: &str = "generation";

fn generation_key() -> Attribute {
    Attribute::Metadata(Cow::Borrowed(GENERATION_META))
}

fn generation_attributes(token: u64) -> Attributes {
    let mut attributes = Attributes::new();
    attributes.insert(generation_key(), token.to_string().into());
    attributes
}

/// The fencing generation stored on the object. Absent metadata is
/// generation 0 — an unfenced occupant a later higher token can take.
fn stored_generation(attributes: &Attributes) -> u64 {
    attributes
        .get(&generation_key())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

/// 20: a stale holder's write is the token the CAS does not win.
/// A matching etag is not a waiver.
fn swap_fence(token: u64, stored: u64) -> Option<Swap> {
    if token < stored {
        Some(Swap::Moved)
    } else {
        None
    }
}

fn fenced_put(mode: PutMode, token: u64) -> PutOptions {
    PutOptions {
        mode,
        attributes: generation_attributes(token),
        ..PutOptions::default()
    }
}

/// 409 Conflict, a timed-out PUT, and any other unproved transport
/// result. `object_store` maps 409 onto `AlreadyExists`, so the walk
/// has to read the status out of the source chain — the variant
/// alone is not a proof.
fn is_unproved(err: &ObjError) -> bool {
    if unproved_text(&err.to_string()) {
        return true;
    }
    let mut current = std::error::Error::source(err);
    while let Some(e) = current {
        if unproved_text(&e.to_string()) {
            return true;
        }
        current = std::error::Error::source(e);
    }
    false
}

fn unproved_text(text: &str) -> bool {
    text.contains("409")
        || text.contains("Conflict")
        || text.contains("CONFLICT")
        || text.contains("timed out")
        || text.contains("TimedOut")
}

/// Create-only: 412 / a remapped `AlreadyExists` is a proved
/// occupation. 409 and a timeout are not — they stay `Ambiguous`.
fn create_from_put(key: &StoreKey, source: ObjError) -> Result<Create> {
    if is_unproved(&source) {
        Ok(Create::Ambiguous)
    } else if matches!(
        source,
        ObjError::AlreadyExists { .. } | ObjError::Precondition { .. }
    ) {
        Ok(Create::Exists)
    } else {
        Err(infra("put_create", key, source))
    }
}

/// Swap: 412 / 404 is a proved `Moved`. 409 lands as `AlreadyExists`
/// in the crate and is never a proved mismatch.
fn swap_from_put(key: &StoreKey, source: ObjError) -> Result<Swap> {
    if is_unproved(&source) || matches!(source, ObjError::AlreadyExists { .. }) {
        Ok(Swap::Ambiguous)
    } else if matches!(
        source,
        ObjError::Precondition { .. } | ObjError::NotFound { .. }
    ) {
        Ok(Swap::Moved)
    } else {
        Err(infra("put_swap", key, source))
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

impl ObjectStore for S3Store {
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>> {
        let path = self.object_path(key)?;
        self.block(async {
            match self.inner.get_opts(&path, GetOptions::new()).await {
                Ok(result) => {
                    let tag = etag_of("get", key, result.meta.e_tag.clone())?;
                    let bytes = result
                        .bytes()
                        .await
                        .map_err(|source| stream_err("get", key, source))?;
                    Ok(Some(Fetched {
                        bytes: bytes.to_vec(),
                        etag: tag,
                    }))
                }
                Err(ObjError::NotFound { .. }) => Ok(None),
                Err(source) => Err(infra("get", key, source)),
            }
        })?
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll> {
        let path = self.object_path(key)?;
        let options = GetOptions::new().with_if_none_match(Some(etag.0.clone()));
        self.block(async {
            match self.inner.get_opts(&path, options).await {
                Ok(result) => {
                    let tag = etag_of("get_if_changed", key, result.meta.e_tag.clone())?;
                    let bytes = result
                        .bytes()
                        .await
                        .map_err(|source| stream_err("get_if_changed", key, source))?;
                    Ok(Poll::Changed(Fetched {
                        bytes: bytes.to_vec(),
                        etag: tag,
                    }))
                }
                Err(ObjError::NotModified { .. }) => Ok(Poll::Unchanged),
                Err(source) => Err(infra("get_if_changed", key, source)),
            }
        })?
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> Result<Create> {
        let body = body.into();
        let path = self.object_path(key)?;
        let payload = body.bytes.to_vec();
        let opts = fenced_put(PutMode::Create, body.token);
        let raw = self.block(async {
            match self.inner.put_opts(&path, payload.into(), opts).await {
                Ok(result) => Ok(Create::Created(etag_of("put_create", key, result.e_tag)?)),
                Err(source) => create_from_put(key, source),
            }
        })??;
        prove_create(self, key, body.bytes, raw)
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> Result<Swap> {
        let body = body.into();
        let path = self.object_path(key)?;
        let payload = body.bytes.to_vec();
        let expected = etag.0.clone();
        let token = body.token;
        let head = GetOptions::new()
            .with_head(true)
            .with_if_match(Some(expected.clone()));
        let opts = fenced_put(
            PutMode::Update(UpdateVersion {
                e_tag: Some(expected),
                version: None,
            }),
            token,
        );
        let raw = self.block(async {
            match self.inner.get_opts(&path, head).await {
                Ok(result) => {
                    if let Some(moved) = swap_fence(token, stored_generation(&result.attributes)) {
                        return Ok(moved);
                    }
                }
                Err(ObjError::NotFound { .. } | ObjError::Precondition { .. }) => {
                    return Ok(Swap::Moved);
                }
                Err(source) => return Err(infra("put_swap", key, source)),
            }
            match self.inner.put_opts(&path, payload.into(), opts).await {
                Ok(result) => Ok(Swap::Swapped(etag_of("put_swap", key, result.e_tag)?)),
                Err(source) => swap_from_put(key, source),
            }
        })??;
        prove_swap(self, key, body.bytes, raw)
    }

    fn delete(&self, key: &StoreKey) -> Result<()> {
        let path = self.object_path(key)?;
        self.block(async {
            match self.inner.delete(&path).await {
                Ok(()) | Err(ObjError::NotFound { .. }) => Ok(()),
                Err(source) => Err(infra("delete", key, source)),
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

    #[test]
    fn join_prefix_omits_the_slash_on_an_empty_prefix() {
        assert_eq!(join_prefix("", "manifest"), "manifest");
        assert_eq!(
            join_prefix("smoke/run", "log/c00000000/1"),
            "smoke/run/log/c00000000/1"
        );
    }

    #[test]
    fn constructor_builds_without_touching_the_network() {
        let store = S3Store::new(&S3Config {
            endpoint: None,
            region: "us-east-1".into(),
            bucket: "example".into(),
            credentials: static_keys(),
            prefix: "/smoke/run/".into(),
        })
        .expect("build");
        let path = store.object_path(&StoreKey::of("manifest")).expect("path");
        assert_eq!(path.as_ref(), "smoke/run/manifest");
    }

    #[test]
    fn constructor_refuses_inside_an_async_context() {
        let nested = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("nested runtime");
        let opened = nested.block_on(async {
            S3Store::new(&S3Config {
                endpoint: None,
                region: "us-east-1".into(),
                bucket: "example".into(),
                credentials: static_keys(),
                prefix: String::new(),
            })
        });
        match opened {
            Err(err) => assert_eq!(err.op, "open"),
            Ok(_) => panic!("S3Store must refuse construction inside an async context"),
        }
    }

    #[test]
    fn verbs_refuse_inside_an_async_context() {
        let store = S3Store::new(&S3Config {
            endpoint: None,
            region: "us-east-1".into(),
            bucket: "example".into(),
            credentials: static_keys(),
            prefix: String::new(),
        })
        .expect("build");
        let key = StoreKey::of("manifest");
        let nested = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("nested runtime");
        let got = nested.block_on(async { store.get(&key) });
        match got {
            Err(err) => assert_eq!(err.op, "block"),
            Ok(_) => panic!("a sync verb is uncallable from an async context"),
        }
    }

    #[test]
    fn constructor_accepts_a_refresh_without_calling_it() {
        let store = S3Store::new(&S3Config {
            endpoint: Some("https://example.r2.cloudflarestorage.com".into()),
            region: "auto".into(),
            bucket: "example".into(),
            credentials: S3Credentials::Refresh(Arc::new(|| {
                panic!("refresh is not called at construction")
            })),
            prefix: String::new(),
        })
        .expect("build");
        let path = store
            .object_path(&StoreKey::of("ckpt/digest.mdb"))
            .expect("path");
        assert_eq!(path.as_ref(), "ckpt/digest.mdb");
    }

    #[test]
    fn constructor_refuses_region_auto_without_an_endpoint() {
        let opened = S3Store::new(&S3Config {
            endpoint: None,
            region: "auto".into(),
            bucket: "example".into(),
            credentials: static_keys(),
            prefix: String::new(),
        });
        match opened {
            Err(err) => assert_eq!(err.op, "open"),
            Ok(_) => panic!("region auto without an endpoint is refused"),
        }
    }

    #[test]
    fn constructor_refuses_a_reserved_prefix() {
        let opened = S3Store::new(&S3Config {
            endpoint: None,
            region: "us-east-1".into(),
            bucket: "example".into(),
            credentials: static_keys(),
            prefix: "~tmp/smoke".into(),
        });
        match opened {
            Err(err) => assert_eq!(err.op, "open"),
            Ok(_) => panic!("reserved prefix is refused"),
        }
    }

    fn conflict_exists(path: &str) -> ObjError {
        ObjError::AlreadyExists {
            path: path.into(),
            source: "409 Conflict".into(),
        }
    }

    #[test]
    fn conflict_on_create_is_ambiguous_never_exists() {
        let key = StoreKey::of("log/c00000000/1");
        assert!(matches!(
            create_from_put(&key, conflict_exists(key.as_str())),
            Ok(Create::Ambiguous)
        ));
        assert!(
            !matches!(
                create_from_put(&key, conflict_exists(key.as_str())),
                Ok(Create::Exists | Create::Created(_))
            ),
            "409 is not a proved occupation"
        );
        let proved = ObjError::Precondition {
            path: key.to_string(),
            source: "412 Precondition Failed".into(),
        };
        assert!(matches!(create_from_put(&key, proved), Ok(Create::Exists)));
    }

    #[test]
    fn conflict_on_swap_is_ambiguous_never_moved() {
        let key = StoreKey::of("manifest");
        assert!(matches!(
            swap_from_put(&key, conflict_exists(key.as_str())),
            Ok(Swap::Ambiguous)
        ));
        assert!(
            !matches!(
                swap_from_put(&key, conflict_exists(key.as_str())),
                Ok(Swap::Moved | Swap::Swapped(_))
            ),
            "409 is not a proved mismatch"
        );
        let proved = ObjError::Precondition {
            path: key.to_string(),
            source: "412 Precondition Failed".into(),
        };
        assert!(matches!(swap_from_put(&key, proved), Ok(Swap::Moved)));
    }

    #[test]
    fn timed_out_put_is_ambiguous() {
        let key = StoreKey::of("log/c00000000/1");
        let timed = ObjError::Generic {
            store: "S3",
            source: "request timed out".into(),
        };
        assert!(matches!(
            create_from_put(&key, timed),
            Ok(Create::Ambiguous)
        ));
    }

    #[test]
    fn stream_failure_is_err_store() {
        let key = StoreKey::of("ckpt/digest.mdb");
        let err: StoreError = stream_err(
            "get",
            &key,
            ObjError::Generic {
                store: "S3",
                source: "body closed mid-stream".into(),
            },
        );
        assert_eq!(err.op, "get");
        assert_eq!(err.key, key.as_str());
        assert!(err.source.to_string().contains("body closed mid-stream"));
    }

    #[test]
    fn static_keys_are_consulted_per_request() {
        let provider = request_credentials(&static_keys());
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let first = provider.get_credential().await.expect("first");
            let second = provider.get_credential().await.expect("second");
            assert_eq!(first.key_id, "AKIAEXAMPLE");
            assert_eq!(second.key_id, first.key_id);
            assert!(
                !Arc::ptr_eq(&first, &second),
                "each sign receives a fresh credential value"
            );
        });
    }

    #[test]
    fn a_lower_token_loses_swap_when_the_etag_matches() {
        let stored = generation_attributes(7);
        assert_eq!(stored_generation(&stored), 7);
        assert_eq!(stored_generation(&Attributes::new()), 0);
        assert!(
            matches!(swap_fence(3, stored_generation(&stored)), Some(Swap::Moved)),
            "a matching etag does not waive body.token < stored generation"
        );
        assert!(
            swap_fence(7, 7).is_none(),
            "the current token is the generation the CAS still wins"
        );
        assert!(swap_fence(8, 7).is_none());
        assert!(
            matches!(swap_fence(0, 1), Some(Swap::Moved)),
            "an unfenced write loses to a stored generation"
        );
    }

    #[test]
    fn refresh_is_consulted_per_request() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let calls = Arc::new(AtomicU64::new(0));
        let counted = Arc::clone(&calls);
        let provider = RefreshProvider {
            refresh: Arc::new(move || {
                counted.fetch_add(1, Ordering::SeqCst);
                Ok(StaticKeys {
                    access_key_id: "AKIA".into(),
                    secret_access_key: "secret".into(),
                    session_token: None,
                })
            }),
        };
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            provider.get_credential().await.expect("first");
            provider.get_credential().await.expect("second");
        });
        assert_eq!(calls.load(Ordering::SeqCst), 2, "refresh is not memoized");
    }
}
