# PRD 09: Probe And Source Frame Integration

## Purpose

Integrate arena source handles with the recursive executor source frame.

## Required Work

- Replace source maps in the arena path with dense atom-indexed source slots where possible.
- Source undo records must store previous compact handles.
- Probe keys must use `KeyScratch` or borrowed key refs.
- `get` must return compact child handles.
- Existing `BTreeMap<AtomOccurrenceId, ColtSource>` may remain only outside hot execution setup or as an adapter until PRD 10 deletes it.

## Passing Criteria

- Hot recursive execution path does not clone source maps.
- Hot recursive execution path does not clone `Rc` source handles.
- Probe key fixture allocates zero heap objects for 8-byte and 16-byte keys.
- q09 exact SQLite comparison passes.
- Global gates pass.
