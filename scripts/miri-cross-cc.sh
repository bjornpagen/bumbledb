#!/bin/sh
# Stand-in C compiler for the Miri lane's cross-interpretation pass
# (scripts/miri.sh, the crucible packet (git ecec1dc3)).
#
# Why it exists: `cargo miri test --target x86_64-unknown-linux-gnu`
# still runs every build script natively, and lmdb-master-sys's build
# script compiles LMDB's C for the *requested* target — which needs a
# linux cross toolchain this host does not have. Under Miri that
# staticlib is a build-graph artifact only: cargo-miri never links a
# native binary (the "executable" it produces is interpreter metadata),
# and the lane's test filter never calls into LMDB (an mdb_* call is
# the exact FFI wall the lane is scoped around — Miri would refuse it
# by name). So a host-arch compile of the same sources satisfies the
# build graph without pretending to be a linux toolchain.
#
# Mechanism: strip the --target/-target arguments and compile with the
# host cc. Everything else (includes, -o, -c, archiving by cc-rs)
# passes through untouched.
args=""
skip=0
for a in "$@"; do
    if [ "$skip" -eq 1 ]; then
        skip=0
        continue
    fi
    case "$a" in
        --target=*) ;;
        -target | --target) skip=1 ;;
        *) args="$args '$(printf %s "$a" | sed "s/'/'\\\\''/g")'" ;;
    esac
done
eval "exec cc $args"
