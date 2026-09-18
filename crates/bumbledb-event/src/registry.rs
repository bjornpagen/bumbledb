use std::collections::HashMap;

use crate::{Capacity, Control, Error, Event, Result, Space};

/// A retained namespace for two-word engine bindings. Full canonical space
/// contents select alignment; exact registered keys select values. Unregistered
/// arena indices, stale keys and foreign registry keys cannot construct Events.
/// Dropping the registry does not invalidate previously returned owned values.
/// Every distinct registered Event and its canonical bytes remain retained until
/// the registry drops. The entry limit bounds their count, not their total bytes.
#[derive(Debug, Clone)]
pub struct Registry {
    spaces: HashMap<Vec<u8>, Space>,
    values: HashMap<[u64; 2], Event>,
    canonical: HashMap<Vec<u8>, [u64; 2]>,
    capacity: usize,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new(2_000_000)
    }
}

impl Registry {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            spaces: HashMap::new(),
            values: HashMap::new(),
            canonical: HashMap::new(),
            capacity,
        }
    }

    /// Publish a checked value in this binding namespace. Presentations with
    /// identical space bytes are explicitly aligned into one owned manager.
    /// # Errors
    /// Refuses encoding/alignment failures, cancellation or exhausted capacity.
    pub fn intern(&mut self, value: &Event, control: &dyn Control) -> Result<Event> {
        control.checkpoint()?;
        if let Some(existing) = self.values.get(&value.key().words()) {
            return Ok(existing.clone());
        }
        let descriptor = value.space().full().to_bytes(control)?;
        let canonical = value.to_bytes(control)?;
        if let Some(key) = self.canonical.get(&canonical) {
            return self.resolve(*key, control);
        }
        let aligned = if let Some(space) = self.spaces.get(&descriptor) {
            value.align_to(space, control)?
        } else {
            value.clone()
        };
        if let Some(existing) = self.values.get(&aligned.key().words()) {
            return Ok(existing.clone());
        }
        if self.values.len() >= self.capacity {
            return Err(Error::Capacity(Capacity::RegistryEntries));
        }
        self.values.try_reserve(1)?;
        self.spaces.try_reserve(1)?;
        self.canonical.try_reserve(1)?;
        control.checkpoint()?;
        self.spaces
            .entry(descriptor)
            .or_insert_with(|| aligned.space());
        self.values.insert(aligned.key().words(), aligned.clone());
        self.canonical.insert(canonical, aligned.key().words());
        Ok(aligned)
    }

    /// Decode and publish canonical bytes in this namespace.
    /// # Errors
    /// Refuses malformed bytes, unsupported formats or unavailable resources.
    pub fn decode(&mut self, bytes: &[u8], control: &dyn Control) -> Result<Event> {
        control.checkpoint()?;
        if let Some(key) = self.canonical.get(bytes) {
            return self.resolve(*key, control);
        }
        let decoded = Event::from_bytes(bytes, control)?;
        self.intern(&decoded, control)
    }

    /// Resolve only explicitly registered keys and retain their complete owner.
    /// # Errors
    /// Returns `UnknownKey` for absent, stale or foreign keys, and propagates
    /// cancellation even when the value is a constant or a cache hit.
    pub fn resolve(&self, words: [u64; 2], control: &dyn Control) -> Result<Event> {
        control.checkpoint()?;
        self.values.get(&words).cloned().ok_or(Error::UnknownKey)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
