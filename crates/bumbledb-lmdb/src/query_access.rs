use smallvec::SmallVec;

use crate::{EncodedOwned, EncodedRef, HashTrieIndex, PrefixProbe, Result};

pub(crate) type SmallEncodedRefs<'a> = SmallVec<[EncodedRef<'a>; 8]>;

pub(crate) enum AccessSource<'a> {
    HashTrie(&'a HashTrieIndex),
}

pub(crate) trait AccessProbe {
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> Result<bool>;
    fn count(&self, prefix: &[EncodedRef<'_>]) -> Result<usize>;
}

impl AccessProbe for AccessSource<'_> {
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> Result<bool> {
        match self {
            AccessSource::HashTrie(index) => Ok(PrefixProbe::exists(*index, prefix)),
        }
    }

    fn count(&self, prefix: &[EncodedRef<'_>]) -> Result<usize> {
        match self {
            AccessSource::HashTrie(index) => Ok(PrefixProbe::count(*index, prefix)),
        }
    }
}

pub(crate) fn encoded_refs(prefix: &[EncodedOwned]) -> SmallEncodedRefs<'_> {
    prefix.iter().map(EncodedOwned::as_ref).collect()
}
