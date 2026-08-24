# 20 — linux-arm64 on Amazon Linux, the duty binary, SDK 0.17.2

## The platform package

`@bjornpagen/bumbledb-linux-arm64`: the napi `.node` for
aarch64-unknown-linux-gnu, built INSIDE an `amazonlinux:2023` arm64
container (40) so the linked glibc is 2.34 — Lambda's own runtime
floor; never an Ubuntu userspace. The build/loader roster widens from
a singleton to a set of two (`darwin-arm64`, `linux-arm64`); the
pack-time pin injection covers both platform packages; the shipped-set
tests (loader roster == publish roster, .gitignore carve-outs, dev-twin
derivation) widen with it. The platform package records the AL2023
baseline where the darwin package records its own.

## SDK 0.17.2 (prepared in lockstep, owner-published)

Payload: `internalDescriptor` (10's centerpiece, the one-descriptor-
authority export beside `internalBlake3`) and the two-platform shipped
set. C ABI stays generation 4; storage stays format 8; no fingerprint
pin moves. ts-log 0.18.0's peer becomes `^0.17.2`.

## The Rust S3 store — box B closes, because its consumer now exists

The duty Lambda must compact (Rust-only) and speak to the bucket — it
is the Rust cloud consumer whose absence kept `S3Store` out of scope.
It lands per proposals/40 as written: over the `object_store` crate,
SigV4, the conditional headers, the retry/GET-verify law already in
store.rs — scoped to ONE storage-class target; the dual-class
constructor arrives with its Express trigger. This pass is the
network-enabled session the receipt boxes have waited for; B and C
close together (30 owns C).

## The duty binary

`crates/bumbledb-log/src/bin/duty.rs` (~100 lines over existing
machinery), two modes from one body:

- `--once` — the Lambda mode: open the prefix as a checkpointer
  (replica + checkpoint/gc rights, no commits, no leases), refresh,
  run ONE cadence check (vector-sum delta or log bytes, the constants
  10-protocol owns), compact + publish under the checkpoint order if
  crossed, run the gc retention law, exit 0. EventBridge fires it on a
  schedule; idempotent by the checkpoint order and content addressing,
  so overlapping invocations are benign races the protocol already
  absorbs.
- default — the resident loop for case-5 machines (the same body,
  slept on a cadence), which is the recorded Rust-sidecar deviation
  made runnable for the local fleet too.

There is no duty Lambda: a custom runtime would owe the Runtime API
loop (a bootstrap, a runtime crate, a second function). Instead the
plain `--once` executable ships INSIDE the app Lambda's bundle, and an
EventBridge schedule invokes the same function with a duty event whose
handler arm `execFile`s the binary — one function, two event shapes,
zero new runtimes (50 wires it). The binary is built aarch64 in the
same AL2023 container and is a dependency-honest plain executable.

## Cross-compilation stance

The CI arm runner with the amazonlinux:2023 container is the ONE
builder for both linux artifacts (`bumbledb.linux-arm64.node`,
`bumbledb-log-duty`). No local cross toolchain is built or documented.
