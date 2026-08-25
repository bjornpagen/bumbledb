//! `S3Store`: the five verbs over one S3-compatible target. `SigV4` and
//! the conditional headers ride the `object_store` crate; this module
//! maps their outcomes onto the trait's sums and nothing else. A 409
//! is `Ambiguous`, never a proved `Exists` or `Moved`. Conditional
//! writes are not retried by the transport. Credentials are consulted
//! per request, off the worker threads.

use std::io;
use std::sync::Arc;

use object_store::aws::{AmazonS3, AmazonS3Builder, AwsCredential};
use object_store::path::Path;
use object_store::{
    CredentialProvider, Error as ObjError, GetOptions, ObjectStore as _, ObjectStoreExt as _,
    PutMode, RetryConfig, UpdateVersion,
};
use tokio::runtime::{Builder, Handle, Runtime};

use super::{
    Create, Etag, Fetched, ObjectStore, Poll, Result, StoreError, StoreKey, Swap, parse_prefix,
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
    builder = match &config.credentials {
        S3Credentials::Static {
            access_key_id,
            secret_access_key,
            session_token,
        } => {
            let mut built = builder
                .with_access_key_id(access_key_id)
                .with_secret_access_key(secret_access_key);
            if let Some(token) = session_token {
                built = built.with_token(token);
            }
            built
        }
        S3Credentials::Refresh(refresh) => builder.with_credentials(Arc::new(RefreshProvider {
            refresh: Arc::clone(refresh),
        })),
    };
    builder.build().map_err(|source| StoreError {
        op: "open",
        key: config.bucket.clone(),
        source: io::Error::other(source),
    })
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

fn etag_of(op: &'static str, key: &StoreKey, raw: Option<String>) -> Result<Etag> {
    raw.map(Etag).ok_or_else(|| StoreError {
        op,
        key: key.to_string(),
        source: io::Error::new(io::ErrorKind::InvalidData, "vendor omitted ETag"),
    })
}

/// 409 Conflict is `Ambiguous`. object_store maps it onto `AlreadyExists`.
fn is_conflict(err: &ObjError) -> bool {
    let text = err.to_string();
    text.contains("409") || text.contains("Conflict")
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
                        .map_err(|source| infra("get", key, source))?;
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
                        .map_err(|source| infra("get_if_changed", key, source))?;
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

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> Result<Create> {
        let path = self.object_path(key)?;
        let payload = bytes.to_vec();
        self.block(async {
            match self
                .inner
                .put_opts(&path, payload.into(), PutMode::Create.into())
                .await
            {
                Ok(result) => Ok(Create::Created(etag_of("put_create", key, result.e_tag)?)),
                Err(source) if is_conflict(&source) => Ok(Create::Ambiguous),
                Err(ObjError::AlreadyExists { .. } | ObjError::Precondition { .. }) => {
                    Ok(Create::Exists)
                }
                Err(source) => Err(infra("put_create", key, source)),
            }
        })?
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> Result<Swap> {
        let path = self.object_path(key)?;
        let payload = bytes.to_vec();
        let mode = PutMode::Update(UpdateVersion {
            e_tag: Some(etag.0.clone()),
            version: None,
        });
        self.block(async {
            match self
                .inner
                .put_opts(&path, payload.into(), mode.into())
                .await
            {
                Ok(result) => Ok(Swap::Swapped(etag_of("put_swap", key, result.e_tag)?)),
                Err(source) if is_conflict(&source) => Ok(Swap::Ambiguous),
                Err(ObjError::Precondition { .. } | ObjError::NotFound { .. }) => Ok(Swap::Moved),
                Err(source) => Err(infra("put_swap", key, source)),
            }
        })?
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
        assert_eq!(join_prefix("", "manifest.json"), "manifest.json");
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
        let path = store
            .object_path(&StoreKey::of("manifest.json"))
            .expect("path");
        assert_eq!(path.as_ref(), "smoke/run/manifest.json");
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
}
