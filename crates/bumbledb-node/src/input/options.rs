//! Runtime options; an absent field keeps the runtime default.
use std::time::Duration;

use napi_derive::napi;
use serde::Deserialize;

use crate::runtime::Options;

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeOptionsIn {
    pub workers: Option<u32>,
    pub queue_capacity: Option<u32>,
    pub cleanup_capacity: Option<u32>,
    pub owner_capacity: Option<u32>,
    pub native_handle_capacity: Option<u32>,
    pub cleanup_timeout_ms: Option<u32>,
}

impl From<RuntimeOptionsIn> for Options {
    fn from(value: RuntimeOptionsIn) -> Self {
        let defaults = Self::default();
        let count = |value: Option<u32>, default: usize| value.map_or(default, |n| n as usize);
        Self {
            workers: count(value.workers, defaults.workers),
            queue_capacity: count(value.queue_capacity, defaults.queue_capacity),
            cleanup_capacity: count(value.cleanup_capacity, defaults.cleanup_capacity),
            owner_capacity: count(value.owner_capacity, defaults.owner_capacity),
            native_handle_capacity: count(
                value.native_handle_capacity,
                defaults.native_handle_capacity,
            ),
            cleanup_timeout: value
                .cleanup_timeout_ms
                .map_or(defaults.cleanup_timeout, |ms| {
                    Duration::from_millis(u64::from(ms))
                }),
        }
    }
}
