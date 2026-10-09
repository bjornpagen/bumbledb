//! The bumbledb log: a database is the create-only sequence of objects
//! `log/{seq}`, each one entry deciding a batch of commands or a migration
//! step. [`Machine`] runs the protocol without doing I/O; a [`Replica`]
//! holds the decided state; checkpoint images bound replay.

mod command;
mod entry;
mod fold;
mod frame;
mod head;
mod ids;
mod io;
mod machine;
mod receipt;
mod replica;

pub use command::{Command, CommandRef, Precondition};
pub use entry::{Body, Decided, Entry, Freeze, Genesis, Migration, MigrationId, Thaw, Verdict};
pub use fold::{Folded, Misplaced, fold, migrated_head};
pub use frame::FrameError;
pub use head::{Comparison, Head, Ledger, Mode, Rejection};
pub use ids::{
    CommandDigest, DatabaseId, ImageDigest, MigrationHash, Millis, Nonce, RequestId, Revision, Seq,
};
pub use io::{
    Body as IoBody, Bucket, CHECKPOINT_PREFIX, CheckpointKey, IoId, IoRequest, IoResponse,
    IoResult, Op, Target, image_key, log_key,
};
pub use machine::{CheckpointPolicy, Config, Done, Input, Machine, Refusal, Settled, Step, Ticket};
pub use receipt::{Delta, Evidence, Outcome, Receipt};
pub use replica::{
    Bundle, BundleError, BundledMigration, CacheError, Image, Judgment, Migrated, Population,
    Replica, Update,
};
