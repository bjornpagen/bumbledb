//! SQLite as an oracle: schema and value mapping, query translation, and the
//! `verify` run that gates every timed family.
pub mod sqlmap;
pub mod translate;
pub mod verify;
