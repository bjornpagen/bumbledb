#!/usr/bin/env bash
# The RAM-backed scratch volume for the adversarial lanes
# (docs/architecture/60-validation.md § the ramdisk sanction). Verify,
# differential, and fuzz lanes may run their scratch stores on RAM —
# they check answers, not wall clocks. Every timed lane refuses
# RAM-backed volumes: `bench` checks its corpus `--dir` (the read
# families time against it) and its write scratch alike (the
# device-honesty rule; the bench crate's detector enforces it).
#
# Subcommands:
#   create  [--size-gib N] [--apfs] [--name NAME]  attach + format + mount; prints the mount path
#   destroy [--name NAME]                          detach/unmount (idempotent)
#   path    [--name NAME]                          prints the mount path if live; exit 1 otherwise
#
# Wiring: the fuzz harness's per-iteration store directories
# (fuzz/src/lib.rs StoreDir) and the bench crate's test scratch
# directories (crates/bumbledb-bench/src/fixture.rs TempDir) respect
# BUMBLEDB_SCRATCH_DIR, so a lane points itself at the ram disk with:
#
#   export BUMBLEDB_SCRATCH_DIR="$(scripts/ramdisk.sh path || scripts/ramdisk.sh create)"
#
# The verify corpus lanes need no env var — `--dir` already points them
# anywhere, the ram disk included. `bench` is the opposite: it refuses
# a RAM-backed `--dir` with the device-honesty refusal (a timed number
# measured on RAM is a lie). Scratch and verify on the ram disk: yes;
# bench: no.
#
# The sentinel: create writes `.bumbledb-ramdisk` (first line the magic
# below, second line the backing identity) at the volume root. It is the
# script's ownership contract — destroy refuses to unmount a directory
# it did not create, and any future std-only RAM-backedness check may
# parse it instead of the platform mount tables.
#
# Sizing: the default is 5 GiB because an EPHEMERAL store's data file
# is ftruncated to the full 4 GiB EPHEMERAL map at open (`MDB_WRITEMAP`,
# MAP_SIZE_EPHEMERAL in crates/bumbledb/src/storage/env.rs — the
# per-kind split: the DURABLE ceiling is 32 GiB but durable opens
# allocate nothing eagerly and never ftruncate, so only the ephemeral
# constant sizes this volume; the scratch kind deliberately keeps the
# small map so this stays a casual ask of a dev machine —
# docs/architecture/50-storage.md § the ephemeral store kind), and the
# default filesystem (HFS+) has no sparse files — a volume below
# map size + slack refuses every `Db::ephemeral` open with a typed
# StorageFull-carrying Lmdb error, which breaks the sanctioned
# BUMBLEDB_SCRATCH_DIR wiring for the ephemeral crashpoint sweep (the
# fixit record). Shrink below 5 GiB only for lanes that open no
# ephemeral store.
#
# macOS arm (the canonical M2 Max): hdiutil ram:// needs no sudo; the
# default filesystem is non-journaled HFS+ (`diskutil erasevolume HFS+`
# — the journaled personality is spelled "Journaled HFS+"); --apfs is
# the fallback flag (measured ~0.4 ms slower per small commit,
# the phase-R harness, crates/bumbledb/tests/ramdisk_phase_r.rs). Every attach is guarded by a trap:
# a failed create detaches its own device.
#
# Linux arm: WRITTEN CAREFULLY BUT UNTESTED (the owner's explicit
# instruction — this machine is the macOS M2 Max; the code below has
# never run). Root: a private tmpfs mount with the noswap option where
# the kernel supports it (6.4+), falling back without. Non-root: a
# private subdirectory of /dev/shm (already tmpfs; the size argument is
# then advisory only).

set -euo pipefail

MAGIC="bumbledb-ramdisk-v1"
NAME="bumbledb-scratch"
SIZE_GIB=5
FS="HFS+"

usage() {
  sed -n '2,20p' "$0" >&2
  exit 2
}

[ $# -ge 1 ] || usage
cmd="$1"
shift

while [ $# -gt 0 ]; do
  case "$1" in
    --size-gib)
      SIZE_GIB="$2"
      shift 2
      ;;
    --apfs) FS="APFS"; shift ;;
    --name)
      NAME="$2"
      shift 2
      ;;
    *) usage ;;
  esac
done

case "$(uname -s)" in
  Darwin) OS=mac ;;
  Linux) OS=linux ;;
  *)
    echo "ramdisk.sh: unsupported platform $(uname -s)" >&2
    exit 1
    ;;
esac

mount_point() {
  if [ "$OS" = mac ]; then
    printf '/Volumes/%s\n' "$NAME"
  elif [ "$(id -u)" = 0 ]; then
    printf '/mnt/%s\n' "$NAME"
  else
    printf '/dev/shm/%s\n' "$NAME"
  fi
}

sentinel() { printf '%s/.bumbledb-ramdisk\n' "$1"; }

cmd_path() {
  local mnt
  mnt="$(mount_point)"
  if [ -f "$(sentinel "$mnt")" ] && head -n 1 "$(sentinel "$mnt")" | grep -qx "$MAGIC"; then
    printf '%s\n' "$mnt"
  else
    echo "ramdisk.sh: no live ram disk named $NAME" >&2
    exit 1
  fi
}

cmd_create_mac() {
  local mnt dev
  mnt="$(mount_point)"
  if [ -e "$mnt" ]; then
    echo "ramdisk.sh: $mnt already exists — destroy it first or pick --name" >&2
    exit 1
  fi
  # 512-byte sectors: GiB * 2^21.
  dev="$(hdiutil attach -nomount "ram://$((SIZE_GIB * 2097152))" | head -n 1 | awk '{print $1}')"
  # The teardown law: a create that fails past the attach detaches its
  # own device before exiting. The device name is expanded into the
  # trap string NOW (double quotes): `dev` is function-local, and an
  # EXIT trap fires after the function scope is torn down — under
  # `set -u` a deferred `$dev` dies as 'unbound variable' and the
  # wired device leaks (reproduced; the fixit record).
  trap "hdiutil detach '$dev' >/dev/null 2>&1 || hdiutil detach -force '$dev' >/dev/null 2>&1 || true" EXIT
  diskutil erasevolume "$FS" "$NAME" "$dev" >/dev/null
  printf '%s\n%s\n' "$MAGIC" "$dev" >"$(sentinel "$mnt")"
  trap - EXIT
  printf '%s\n' "$mnt"
}

cmd_destroy_mac() {
  local mnt dev
  mnt="$(mount_point)"
  if [ ! -e "$mnt" ]; then
    echo "ramdisk.sh: nothing to destroy at $mnt" >&2
    return 0
  fi
  if [ ! -f "$(sentinel "$mnt")" ] || ! head -n 1 "$(sentinel "$mnt")" | grep -qx "$MAGIC"; then
    echo "ramdisk.sh: $mnt carries no sentinel — not ours, refusing to destroy" >&2
    exit 1
  fi
  dev="$(sed -n '2p' "$(sentinel "$mnt")")"
  hdiutil detach "$dev" >/dev/null || hdiutil detach -force "$dev" >/dev/null
}

# ---- Linux arm: written carefully but UNTESTED (see the header) -------

cmd_create_linux() {
  local mnt
  mnt="$(mount_point)"
  if [ -e "$mnt" ]; then
    echo "ramdisk.sh: $mnt already exists — destroy it first or pick --name" >&2
    exit 1
  fi
  mkdir -p "$mnt"
  if [ "$(id -u)" = 0 ]; then
    # A mount that fails leaves no half-made volume behind. Expanded at
    # trap-set time (double quotes): `mnt` is function-local and the
    # EXIT trap outlives the function scope — the same unbound-variable
    # leak the macOS arm had (the fixit record).
    trap "umount '$mnt' >/dev/null 2>&1 || true; rmdir '$mnt' >/dev/null 2>&1 || true" EXIT
    # noswap (kernel 6.4+) keeps the scratch honest RAM; older kernels
    # refuse the option, so fall back without it.
    mount -t tmpfs -o "size=${SIZE_GIB}G,noswap" bumbledb-scratch "$mnt" 2>/dev/null ||
      mount -t tmpfs -o "size=${SIZE_GIB}G" bumbledb-scratch "$mnt"
    printf '%s\n%s\n' "$MAGIC" "tmpfs:$mnt" >"$(sentinel "$mnt")"
    trap - EXIT
  else
    # /dev/shm is tmpfs by convention; verify rather than assume.
    if ! awk '$2 == "/dev/shm" && $3 == "tmpfs"' /proc/mounts | grep -q .; then
      rmdir "$mnt"
      echo "ramdisk.sh: /dev/shm is not tmpfs here and we are not root — no RAM-backed scratch available" >&2
      exit 1
    fi
    # The size argument is advisory in this arm: the subdirectory
    # shares /dev/shm's own limit.
    printf '%s\n%s\n' "$MAGIC" "shm:$mnt" >"$(sentinel "$mnt")"
  fi
  printf '%s\n' "$mnt"
}

cmd_destroy_linux() {
  local mnt
  mnt="$(mount_point)"
  if [ ! -e "$mnt" ]; then
    echo "ramdisk.sh: nothing to destroy at $mnt" >&2
    return 0
  fi
  if [ ! -f "$(sentinel "$mnt")" ] || ! head -n 1 "$(sentinel "$mnt")" | grep -qx "$MAGIC"; then
    echo "ramdisk.sh: $mnt carries no sentinel — not ours, refusing to destroy" >&2
    exit 1
  fi
  if [ "$(id -u)" = 0 ] && awk -v m="$mnt" '$2 == m && $3 == "tmpfs"' /proc/mounts | grep -q .; then
    umount "$mnt"
    rmdir "$mnt"
  else
    rm -rf "$mnt"
  fi
}

case "$cmd" in
  create) "cmd_create_${OS}" ;;
  destroy) "cmd_destroy_${OS}" ;;
  path) cmd_path ;;
  *) usage ;;
esac
