#!/bin/busybox sh
# This is an initramfs PID1, never a host/container test entrypoint.
set -eu
[ "$$" -eq 1 ] || { echo "static init must be guest PID1" >&2; exit 2; }
/bin/busybox mount -t proc proc /proc
case "$(/bin/busybox cat /proc/cmdline)" in
    *bumbledb.static-probe=1*) ;;
    *) echo "static init requires the explicit guest boot marker" >&2; exit 2 ;;
esac

finish() {
    result=$?
    trap - EXIT
    if [ "$result" -eq 0 ]; then
        echo "BUMBLEDB_STATIC_QEMU: PASS"
    else
        echo "BUMBLEDB_STATIC_QEMU: FAIL ($result)"
    fi
    /bin/busybox poweroff -f
    /bin/busybox halt -f
}
trap finish EXIT

# No runtime loader or shared libraries exist in this fresh root filesystem.
[ ! -e /lib ] && [ ! -e /usr/lib ]
/bin/busybox mkdir -p /tmp/host
/bumbledb-static-smoke
/bumbledb-log-duty status --fs-root /tmp/host --prefix static > /tmp/duty-status
/bin/busybox cat /tmp/duty-status
/bin/busybox grep -q '^condition: Missing$' /tmp/duty-status
[ ! -e /tmp/host/static/HEAD ]
