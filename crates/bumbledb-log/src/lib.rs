//! The bumbledb log: a database is the create-only sequence of objects
//! `log/{seq}`, each one entry: a batch of commands or a migration step. A
//! batch names the head it was judged at; its [`standing`] where it lands says
//! whether that judgment holds. [`Machine`] runs the protocol without doing
//! I/O; a [`Replica`] holds the decided state; checkpoint images bound replay.

mod cache;
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

pub use cache::{Cache, CacheDb};
pub use command::{Command, CommandRef, Precondition};
pub use entry::{Batch, Body, Entry, Freeze, Genesis, Migration, MigrationId, Proposal, Thaw};
pub use fold::{Folded, Misplaced, Standing, fold, migrated_head, standing};
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
    Bundle, BundleError, BundledMigration, CacheError, Image, Migrated, Population, Replica, Update,
};
