use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

use crate::tuple::EncodedTuple;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KeyRef<'key> {
    bytes: &'key [u8],
}

impl<'key> KeyRef<'key> {
    pub(crate) fn new(bytes: &'key [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn bytes(self) -> &'key [u8] {
        self.bytes
    }
}

#[derive(Clone, Debug)]
pub(crate) enum KeyOwned {
    K8([u8; 8]),
    K16([u8; 16]),
    Heap(Vec<u8>),
}

impl KeyOwned {
    pub(crate) fn from_slice(bytes: &[u8]) -> Self {
        match bytes.len() {
            8 => {
                let mut out = [0; 8];
                out.copy_from_slice(bytes);
                Self::K8(out)
            }
            16 => {
                let mut out = [0; 16];
                out.copy_from_slice(bytes);
                Self::K16(out)
            }
            _ => Self::Heap(bytes.to_vec()),
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Self::K8(bytes) => bytes,
            Self::K16(bytes) => bytes,
            Self::Heap(bytes) => bytes,
        }
    }

    pub(crate) fn as_key_ref(&self) -> KeyRef<'_> {
        KeyRef::new(self.bytes())
    }

    pub(crate) fn to_encoded_tuple(&self) -> EncodedTuple {
        EncodedTuple::from_bytes(self.bytes().to_vec())
    }

    #[cfg(test)]
    fn is_inline(&self) -> bool {
        !matches!(self, Self::Heap(_))
    }
}

impl PartialEq for KeyOwned {
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}

impl Eq for KeyOwned {}

impl Hash for KeyOwned {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes().hash(state);
    }
}

impl Borrow<[u8]> for KeyOwned {
    fn borrow(&self) -> &[u8] {
        self.bytes()
    }
}

impl PartialEq<KeyRef<'_>> for KeyOwned {
    fn eq(&self, other: &KeyRef<'_>) -> bool {
        self.bytes() == other.bytes()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct KeyScratch {
    inline: [u8; 32],
    heap: Vec<u8>,
    len: usize,
}

impl KeyScratch {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set(&mut self, bytes: &[u8]) -> KeyRef<'_> {
        self.len = bytes.len();
        if bytes.len() <= self.inline.len() {
            self.inline[..bytes.len()].copy_from_slice(bytes);
            KeyRef::new(&self.inline[..bytes.len()])
        } else {
            self.heap.clear();
            self.heap.extend_from_slice(bytes);
            KeyRef::new(&self.heap)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hint::black_box;

    use crate::diagnostics::{
        allocation_delta, allocation_snapshot, with_allocation_tracking_for_test,
    };

    use super::*;

    #[test]
    fn borrowed_and_owned_keys_have_equivalent_equality_and_hash() {
        let bytes = [7; 16];
        let borrowed = KeyRef::new(&bytes);
        let owned = KeyOwned::from_slice(&bytes);

        assert_eq!(owned, borrowed);
        assert_eq!(hash_bytes(borrowed.bytes()), hash_key(&owned));
    }

    #[test]
    fn eight_and_sixteen_byte_keys_store_inline() {
        let k8 = KeyOwned::from_slice(&[1; 8]);
        let k16 = KeyOwned::from_slice(&[2; 16]);

        assert!(matches!(k8, KeyOwned::K8(_)));
        assert!(matches!(k16, KeyOwned::K16(_)));
        assert!(k8.is_inline());
        assert!(k16.is_inline());
    }

    #[test]
    fn wider_keys_fall_back_safely() {
        let k24 = KeyOwned::from_slice(&[3; 24]);
        let k64 = KeyOwned::from_slice(&[4; 64]);

        assert!(matches!(k24, KeyOwned::Heap(_)));
        assert!(matches!(k64, KeyOwned::Heap(_)));
        assert_eq!(k24.bytes(), &[3; 24]);
        assert_eq!(k64.bytes(), &[4; 64]);
    }

    #[test]
    fn scratch_probe_keys_do_not_allocate_for_inline_widths() {
        let alloc_calls = with_allocation_tracking_for_test(|| {
            let mut scratch = KeyScratch::new();
            let start = allocation_snapshot();
            assert_eq!(scratch.set(&[1; 8]).bytes(), &[1; 8]);
            assert_eq!(scratch.set(&[2; 16]).bytes(), &[2; 16]);
            allocation_delta(start, allocation_snapshot()).alloc_calls
        });

        assert!(
            alloc_calls < 100,
            "inline scratch key setup should remain below noisy test-harness allocation bounds: {alloc_calls} calls"
        );
    }

    #[test]
    fn inline_owned_keys_allocate_less_than_heap_tuple_keys() {
        let bytes = [9; 8];
        let inline_calls = with_allocation_tracking_for_test(|| {
            let start = allocation_snapshot();
            for _ in 0..1000 {
                let key = KeyOwned::from_slice(&bytes);
                black_box(key.bytes().len());
            }
            allocation_delta(start, allocation_snapshot()).alloc_calls
        });
        let tuple_calls = with_allocation_tracking_for_test(|| {
            let start = allocation_snapshot();
            for _ in 0..1000 {
                let tuple = EncodedTuple::from_bytes(bytes.to_vec());
                black_box(tuple.bytes().len());
            }
            allocation_delta(start, allocation_snapshot()).alloc_calls
        });

        assert!(
            inline_calls * 4 < tuple_calls,
            "inline keys should allocate materially less than heap tuple keys: inline={inline_calls} tuple={tuple_calls}"
        );
    }

    fn hash_key(key: &KeyOwned) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_bytes(bytes: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }
}
