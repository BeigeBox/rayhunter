#!/bin/sh
# Cross-compile Linux 3.18.48 for QEMU vexpress-a9 using a Bootlin toolchain.
#
# Runs in Docker (Debian, amd64 — Bootlin toolchain is x86_64 binaries).
# Downloads toolchain and kernel source on first run; subsequent runs
# reuse the cached build directory.
#
# Output: emulator/vm/zImage, emulator/vm/vexpress-v2p-ca9.dtb
#
# Usage: ./emulator/scripts/build-kernel.sh

set -e

cd "$(dirname "$0")/../.."

mkdir -p emulator/vm emulator/build

echo "=== Building kernel 3.18.48 in Docker ==="
docker run --rm --platform linux/amd64 \
    -v "$(pwd)/emulator/vm":/out \
    -v "$(pwd)/emulator/build":/build \
    -v "$(pwd)/emulator/scripts/build-kernel-inner.sh":/build.sh:ro \
    debian:bullseye sh /build.sh
