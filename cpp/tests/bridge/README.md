# tests/bridge/

Runtime tests of the raw C ABI (TODO_CPP §35), independent of reflection:
create/open/close, read/write callbacks, aborts, scans, prepare/execute,
keyed gets, bulk import, error/prepared destruction. These answer "is the
foreign bridge correct?"; the cookbook tests answer "is the reflective
language correct?" — the two are never conflated. Empty until the bridge
crate lands.
