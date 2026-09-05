//! C07 — the conditional-store verbs the hosted publication machine needs.
//!
//! This is P04's *proposed* seam for P05's backend. P05 owns the actual S3 /
//! filesystem / memory adapters, their durability ordering, path namespace,
//! lock exclusion and fault taxonomy. P04 owns the publication machine that
//! calls these verbs and interprets their conditional-result grammar. The
//! trait is deliberately the exact set of operations chapter 20/21 use — read
//! a versioned head, conditionally create/replace it, put/get immutable
//! objects, list and delete — not a generic five-verb KV framework.
//!
//! The three-way conditional grammar is load-bearing: a transport failure
//! after dispatch is `Indeterminate`, never a manufactured success or failure.
//! Emulator green is not S3 qualification; that evidence is P05/P12's F3 lane.

/// An opaque backend version token for the head object (S3 version-id/ETag,
/// or a filesystem generation). The machine never parses it; it only feeds an
/// exact token back into a conditional replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadVersion(pub Box<[u8]>);

/// The head object read: its bytes plus the exact version token to condition a
/// replacement on. `Absent` is a definite "no head", distinct from a transport
/// failure (which is `Err`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadRead {
    Present {
        version: HeadVersion,
        body: Box<[u8]>,
    },
    Absent,
}

/// The outcome of a conditional head create/replace. These three arms are the
/// entire certainty grammar the publication machine reasons about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalOutcome {
    /// The precondition held and this exact body is now the head. Carries the
    /// new version token so the caller can chain further maintenance.
    Published { version: HeadVersion },
    /// The precondition definitively failed: another writer won, or (for a
    /// create) a head already exists. Not this attempt's publication.
    PreconditionFailed,
    /// The request was dispatched but its outcome is unknown (lost response,
    /// timeout after send). Never interpreted as either published or failed;
    /// the machine resolves it by reading the head/receipt.
    Indeterminate,
}

/// An immutable object read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectRead {
    Present { body: Box<[u8]> },
    Absent,
}

/// The result of putting an immutable content-addressed object. Immutable
/// objects may use verified content equality as publication evidence (an
/// ambiguous re-PUT of identical bytes is absorbed); a *head* replacement may
/// not — it needs unique transition evidence, hence [`ConditionalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PutOutcome {
    /// The object is durably present with the expected content.
    Stored,
    /// Dispatched, outcome unknown; the caller re-reads to confirm content.
    Indeterminate,
}

/// One bounded listing page: the object keys in this page and an optional
/// continuation token. A continuation token is an optimization, not a global
/// snapshot promise (chapter 21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPage {
    pub keys: Vec<String>,
    pub next: Option<Box<[u8]>>,
}

/// The backend's own infrastructure failure. P05 owns the concrete taxonomy;
/// the machine only needs to distinguish "no definite outcome" (retry or
/// return uncertainty) from a definite conditional arm.
pub trait ConditionalStore {
    type Error;

    /// Read the current head at `head_key`, or `Absent`.
    /// # Errors
    /// A transport/auth/IO failure with no definite head observation.
    fn read_head(&self, head_key: &str) -> Result<HeadRead, Self::Error>;

    /// Create the head only if it does not exist. `PreconditionFailed` means a
    /// head already exists — a never-reused head is never re-created over.
    /// # Errors
    /// Transport failure with no definite create observation.
    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, Self::Error>;

    /// Replace the head only if its current version equals `expected`. The
    /// successful atomic replacement is the hosted publication linearization
    /// point. Every proposed body differs through its monotone head revision,
    /// even when application state does not (so equal-bytes ABA is impossible).
    /// # Errors
    /// Transport failure with no definite replacement observation.
    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, Self::Error>;

    /// Put an immutable content-addressed object. Re-putting identical bytes
    /// is idempotent (content equality is evidence). The key encodes epoch and
    /// digest; P05 verifies length/digest before this returns `Stored`.
    /// # Errors
    /// Transport failure with no definite store observation.
    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, Self::Error>;

    /// Get an immutable object.
    /// # Errors
    /// Transport failure with no definite read observation.
    fn get_object(&self, key: &str) -> Result<ObjectRead, Self::Error>;

    /// List actual extant object keys under `prefix`, one bounded page.
    /// Listing enumerates real names, never a historical slot count.
    /// # Errors
    /// Transport failure with no definite listing.
    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, Self::Error>;

    /// Delete one eligible object key. Idempotent: deleting an absent key
    /// succeeds. Never deletes the head through this verb.
    /// # Errors
    /// Transport failure; the caller retains resumable deletion progress.
    fn delete_object(&self, key: &str) -> Result<(), Self::Error>;
}

/// Every verb takes `&self`, so a shared reference to a store is itself a
/// store: one adapter instance can serve the publication machine, the
/// checkpointer and a collector concurrently without a wrapper type.
macro_rules! delegate_conditional_store {
    ([$($generics:tt)+] $ty:ty) => {
        impl<$($generics)+> ConditionalStore for $ty {
            type Error = T::Error;

            fn read_head(&self, head_key: &str) -> Result<HeadRead, Self::Error> {
                (**self).read_head(head_key)
            }

            fn create_head(
                &self,
                head_key: &str,
                body: &[u8],
            ) -> Result<ConditionalOutcome, Self::Error> {
                (**self).create_head(head_key, body)
            }

            fn replace_head(
                &self,
                head_key: &str,
                expected: &HeadVersion,
                body: &[u8],
            ) -> Result<ConditionalOutcome, Self::Error> {
                (**self).replace_head(head_key, expected, body)
            }

            fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, Self::Error> {
                (**self).put_object(key, body)
            }

            fn get_object(&self, key: &str) -> Result<ObjectRead, Self::Error> {
                (**self).get_object(key)
            }

            fn list_objects(
                &self,
                prefix: &str,
                after: Option<&[u8]>,
            ) -> Result<ListPage, Self::Error> {
                (**self).list_objects(prefix, after)
            }

            fn delete_object(&self, key: &str) -> Result<(), Self::Error> {
                (**self).delete_object(key)
            }
        }
    };
}

delegate_conditional_store!(['a, T: ConditionalStore + ?Sized] &'a T);
delegate_conditional_store!([T: ConditionalStore + ?Sized] std::sync::Arc<T>);
