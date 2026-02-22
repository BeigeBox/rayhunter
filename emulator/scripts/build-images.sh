#!/bin/sh
# Convert reference tarballs into a clean QEMU-bootable disk image.
#
# Requires Docker (uses Alpine container for mkfs.ext4, loopback mounts,
# and partition table creation — not available on macOS natively).
#
# Input:  emulator/reference/{rootfs,data}.tar.gz
# Output: emulator/vm/disk.img (partitioned: rootfs + data)
#
# The inner script cleans dev-device artifacts (wifi creds, SSH keys,
# test QMDLs, custom configs) so the image matches a fresh device.
#
# Usage: ./emulator/scripts/build-images.sh

set -e

cd "$(dirname "$0")/../.."

REF="emulator/reference"
VM="emulator/vm"
DIST="dist/config.toml.in"

for f in "$REF/rootfs.tar.gz" "$REF/data.tar.gz" "$DIST"; do
    if [ ! -f "$f" ]; then
        echo "Missing: $f" >&2
        echo "Run emulator/scripts/dump-real-device.sh first." >&2
        exit 1
    fi
done

mkdir -p "$VM"

echo "=== Building disk image via Docker ==="
docker run --rm --privileged \
    -v "$(pwd)/$REF":/reference:ro \
    -v "$(pwd)/$VM":/vm \
    -v "$(pwd)/$DIST":/config.toml.in:ro \
    -v "$(pwd)/emulator/scripts/build-images-inner.sh":/build.sh:ro \
    alpine:latest sh /build.sh

echo "=== Image built ==="
ls -lh "$VM/disk.img"
