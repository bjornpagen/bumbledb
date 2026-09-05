//! `S3Store`: the conditional-store verbs over one S3 bucket/prefix. `SigV4`
//! and the conditional headers ride the `object_store` crate; this module
//! maps vendor outcomes onto the three-way conditional grammar and nothing
//! else. A 409/timeout is `Indeterminate`, never a proved publication or
//! failure; a 412 (`Precondition`) is a proved loss. Conditional writes are
//! never retried by the transport. Credentials are consulted per request,
//! off the worker threads. `ETags` stay opaque version tokens.
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
    CredentialProvider, Error as ObjError, GetOptions, ObjectStore as _, ObjectStoreExt as _,
    PutMode, PutOptions, RetryConfig, UpdateVersion,
};
use tokio::runtime::{Builder, Handle, Runtime};

use super::key_ok;
use crate::writer::verbs::{
    ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion, ListPage, ObjectRead, PutOutcome,
};

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
/// Never a protocol outcome.
#[derive(Debug)]
pub struct S3Error {
    pub op: &'static str,
    pub key: String,
    pub source: io::Error,
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
            return Err(S3Error {
                op: "open",
                key: config.bucket.clone(),
                source: io::Error::other("S3Store is constructed outside an async context"),
            });
        }
        if config.region == "auto" && config.endpoint.is_none() {
            return Err(S3Error {
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
            .map_err(|source| S3Error {
                op: "open",
                key: config.bucket.clone(),
                source,
            })?;
        let handle = runtime.handle().clone();
        let inner = build_client(config)?;
        Ok(Self {
            inner,
            handle,
            _runtime: Arc::new(runtime),
        })
    }

    fn path_of(op: &'static str, key: &str) -> Result<Path, S3Error> {
        if !key_ok(key) {
            return Err(S3Error {
                op,
                key: key.to_string(),
                source: io::Error::new(io::ErrorKind::InvalidInput, "invalid store key"),
            });
        }
        Path::parse(key).map_err(|source| S3Error {
            op,
            key: key.to_string(),
            source: io::Error::other(source),
        })
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
            return Err(S3Error {
                op,
                key: key.to_string(),
                source: io::Error::other("store verbs are synchronous"),
            });
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
    builder.build().map_err(|source| S3Error {
        op: "open",
        key: config.bucket.clone(),
        source: io::Error::other(source),
    })
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
    S3Error {
        op,
        key: key.to_string(),
        source: io::Error::other(source),
    }
}

/// 409 Conflict, a timed-out PUT, and any other unproved transport result.
/// `object_store` maps 409 onto `AlreadyExists`, so the walk reads the
/// status out of the source chain — the variant alone is not a proof.
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

fn version_of(op: &'static str, key: &str, raw: Option<String>) -> Result<HeadVersion, S3Error> {
    raw.map(|etag| HeadVersion(Box::from(etag.into_bytes())))
        .ok_or_else(|| S3Error {
            op,
            key: key.to_string(),
            source: io::Error::new(io::ErrorKind::InvalidData, "vendor omitted ETag"),
        })
}

fn etag_text(version: &HeadVersion) -> String {
    String::from_utf8_lossy(&version.0).into_owned()
}

impl ConditionalStore for S3Store {
    type Error = S3Error;

    fn read_head(&self, head_key: &str) -> Result<HeadRead, S3Error> {
        let path = Self::path_of("read_head", head_key)?;
        self.block("read_head", head_key, async {
            match self.inner.get_opts(&path, GetOptions::new()).await {
                Ok(result) => {
                    let version = version_of("read_head", head_key, result.meta.e_tag.clone())?;
                    let bytes = result
                        .bytes()
                        .await
                        .map_err(|source| infra("read_head", head_key, source))?;
                    Ok(HeadRead::Present {
                        version,
                        body: Box::from(bytes.as_ref()),
                    })
                }
                Err(ObjError::NotFound { .. }) => Ok(HeadRead::Absent),
                Err(source) => Err(infra("read_head", head_key, source)),
            }
        })?
    }

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, S3Error> {
        let path = Self::path_of("create_head", head_key)?;
        let payload = body.to_vec();
        self.block("create_head", head_key, async {
            let opts = PutOptions {
                mode: PutMode::Create,
                ..PutOptions::default()
            };
            match self.inner.put_opts(&path, payload.into(), opts).await {
                Ok(result) => Ok(ConditionalOutcome::Published {
                    version: version_of("create_head", head_key, result.e_tag)?,
                }),
                Err(source) if is_unproved(&source) => Ok(ConditionalOutcome::Indeterminate),
                Err(ObjError::AlreadyExists { .. } | ObjError::Precondition { .. }) => {
                    Ok(ConditionalOutcome::PreconditionFailed)
                }
                Err(source) => Err(infra("create_head", head_key, source)),
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
                Ok(result) => Ok(ConditionalOutcome::Published {
                    version: version_of("replace_head", head_key, result.e_tag)?,
                }),
                Err(source) if is_unproved(&source) => Ok(ConditionalOutcome::Indeterminate),
                Err(ObjError::Precondition { .. } | ObjError::NotFound { .. }) => {
                    Ok(ConditionalOutcome::PreconditionFailed)
                }
                // The crate can remap a conditional conflict onto
                // AlreadyExists; without a proved status that is unproved.
                Err(ObjError::AlreadyExists { .. }) => Ok(ConditionalOutcome::Indeterminate),
                Err(source) => Err(infra("replace_head", head_key, source)),
            }
        })?
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, S3Error> {
        let path = Self::path_of("put_object", key)?;
        let payload = body.to_vec();
        self.block("put_object", key, async {
            match self.inner.put(&path, payload.into()).await {
                Ok(_) => Ok(PutOutcome::Stored),
                Err(source) if is_unproved(&source) => Ok(PutOutcome::Indeterminate),
                Err(source) => Err(infra("put_object", key, source)),
            }
        })?
    }

    fn get_object(&self, key: &str) -> Result<ObjectRead, S3Error> {
        let path = Self::path_of("get_object", key)?;
        self.block("get_object", key, async {
            match self.inner.get_opts(&path, GetOptions::new()).await {
                Ok(result) => {
                    let bytes = result
                        .bytes()
                        .await
                        .map_err(|source| infra("get_object", key, source))?;
                    Ok(ObjectRead::Present {
                        body: Box::from(bytes.as_ref()),
                    })
                }
                Err(ObjError::NotFound { .. }) => Ok(ObjectRead::Absent),
                Err(source) => Err(infra("get_object", key, source)),
            }
        })?
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, S3Error> {
        const PAGE: usize = 1_000;
        // Recursive delimiter listing enumerates actual extant names — no
        // historical slot scan. The returned page is bounded; the caller
        // resumes with the continuation token.
        let trimmed = prefix.trim_end_matches('/');
        let root = Path::parse(trimmed).map_err(|source| S3Error {
            op: "list_objects",
            key: prefix.to_string(),
            source: io::Error::other(source),
        })?;
        let resume: Option<String> = after.map(|token| String::from_utf8_lossy(token).into_owned());
        let mut keys: Vec<String> = self.block("list_objects", prefix, async {
            let mut found = Vec::new();
            let mut stack = vec![root];
            while let Some(dir) = stack.pop() {
                let listed = self
                    .inner
                    .list_with_delimiter(Some(&dir))
                    .await
                    .map_err(|source| infra("list_objects", prefix, source))?;
                for object in listed.objects {
                    found.push(object.location.to_string());
                }
                stack.extend(listed.common_prefixes);
            }
            Ok::<_, S3Error>(found)
        })??;
        keys.sort();
        let keys: Vec<String> = keys
            .into_iter()
            .filter(|key| key.starts_with(prefix))
            .filter(|key| resume.as_deref().is_none_or(|resume| key.as_str() > resume))
            .take(PAGE)
            .collect();
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
            assert!(store.read_head("t/HEAD").is_err());
            assert!(store.get_object("t/objects/1/chunk/aa").is_err());
        });
    }

    #[test]
    fn hostile_keys_refuse_before_any_request() {
        let store = S3Store::new(&config()).expect("build");
        for key in ["~tmp/x", "a/../b", "a//b", "x.lock"] {
            assert!(store.get_object(key).is_err(), "{key}");
        }
    }

    #[test]
    fn unproved_conflicts_and_timeouts_are_indeterminate_shapes() {
        let conflict = ObjError::AlreadyExists {
            path: "p".into(),
            source: Box::new(io::Error::other("Client error with status 409 Conflict")),
        };
        assert!(is_unproved(&conflict));
        let timeout = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::new(io::ErrorKind::TimedOut, "timed out")),
        };
        assert!(is_unproved(&timeout));
        let denied = ObjError::Generic {
            store: "S3",
            source: Box::new(io::Error::other("403 Forbidden")),
        };
        assert!(!is_unproved(&denied), "a proved denial is not ambiguity");
    }
}
