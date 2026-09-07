#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

output="${1:?usage: prepare-rootfs.sh OUTPUT}"
output="$(cd "$output" && pwd)"
[ -f "$output/bumbledb-static-smoke" ]
[ -f "$output/bumbledb-log-duty" ]
# Finch's macOS bind mount cannot hold Linux device nodes. Keep the disposable
# root inside the container and export only its portable cpio archive.
rootfs="$(mktemp -d /tmp/bumbledb-static-rootfs.XXXXXX)"
mkdir -p "$rootfs/bin" "$rootfs/dev" "$rootfs/proc" "$rootfs/tmp"
chmod 1777 "$rootfs/tmp"
cp "$output/bumbledb-static-smoke" "$output/bumbledb-log-duty" "$rootfs/"
cp /bin/busybox.static "$rootfs/bin/busybox"
cp scripts/static-linux-arm64/init.sh "$rootfs/init"
chmod 755 "$rootfs/init" "$rootfs/bin/busybox"
mknod -m 600 "$rootfs/dev/console" c 5 1
mknod -m 666 "$rootfs/dev/null" c 1 3

echo "==> execute static core in a fresh root with no libraries"
chroot "$rootfs" /bumbledb-static-smoke
mkdir "$rootfs/tmp/host"
chroot "$rootfs" /bumbledb-log-duty status --fs-root /tmp/host --prefix static > "$output/chroot-duty-status.txt"
grep -q '^condition: Missing$' "$output/chroot-duty-status.txt"
[ ! -e "$rootfs/tmp/host/static/HEAD" ]

cp /boot/vmlinuz-virt "$output/vmlinuz-virt"
(cd "$rootfs" && find . -print0 | cpio --null --create --format=newc --owner=0:0) | gzip -1 > "$output/initramfs.cpio.gz"
printf '%s\n' "$rootfs" > "$output/rootfs-path.txt"
echo "static chroot checks passed; full-system QEMU kernel and initramfs prepared"
