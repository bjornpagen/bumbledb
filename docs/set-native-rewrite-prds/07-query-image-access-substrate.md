# 07 Query Image Access Substrate

## Purpose

Build the new query-image substrate over set-native access namespaces. This PRD provides the storage/query bridge used by the executor rewrite.

## Required Access APIs

Relation image must expose:

```text
relation_cardinality()
access_prefix_exists(access, key)
access_prefix_cardinality(access, key)
access_prefix_iter(access, key)
access_key_domain_iter(access, prefix, projected_fields)
tuple_bytes(tuple_fingerprint)
decode_tuple(tuple_fingerprint)
```

The API must distinguish:

- tuple cardinality
- distinct key-domain cardinality
- existence
- tuple payload fetch

## Required Layout

Access images should be compact and contiguous:

- Encoded key columns or flat key byte slab.
- Tuple fingerprint slab.
- Prefix range index or trie levels with subtree cardinalities.
- Optional payload column slabs only when an access path explicitly includes payload fields.

## Required Code Changes

- Replace `RelationIndexImage` full-entry `bytes` with set-native access image structures.
- Replace `prefix_count` with cardinality functions that use stored range length or subtree counts.
- Replace `entries_with_prefix` callers with typed access iterators.
- Keep fixed-width `[u8; 1]`, `[u8; 8]`, `[u8; 16]` encoded values for column/key storage.
- Delete row-id accessors from `RelationImage` public API.

## Acceptance Gates

- Prefix existence is O(log n) or O(depth), not O(prefix cardinality).
- Prefix cardinality does not scan matching entries.
- Access iteration returns deterministic order.
- Full tuple decode happens only at output/API boundary or when a predicate requires it.
- Query image memory counters report relation/access/key/payload bytes separately.

## Tests Required

- Prefix exists/cardinality/iter consistency over generated access keys.
- Compound prefix cardinality.
- Range prefix cardinality.
- Tuple fingerprint lookup round trip.
- Query image scoped loading loads only requested relations/accesses/payloads.

## Non-Goals

- No generic document payload store.
- No old full-entry `RelationIndexImage` compatibility.
