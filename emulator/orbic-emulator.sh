#!/bin/sh
# Boot the Orbic RC400L emulator in QEMU (runs inside Docker).
#
# Prerequisites:
#   - emulator/vm/zImage and vexpress-v2p-ca15-tc1.dtb (from build-kernel.sh)
#   - emulator/vm/disk.img (from build-images.sh)
#
# The emulator boots with:
#   - Serial console on stdio (ttyAMA0)
#   - Port 8080 forwarded (rayhunter web UI)
#   - Port 5555 forwarded (ADB)
#   - Root filesystem mounted read-only (like a fresh device)
#   - /data mounted read-write from second partition
#
# Usage: ./emulator/orbic-emulator.sh
#
# To exit QEMU: Ctrl-A then X

set -e

cd "$(dirname "$0")"

for f in vm/zImage vm/vexpress-v2p-ca15-tc1.dtb vm/disk.img; do
    if [ ! -f "$f" ]; then
        echo "Missing: $f" >&2
        echo "Run scripts/build-kernel.sh and scripts/build-images.sh first." >&2
        exit 1
    fi
done

# Make a working copy so the original stays clean.
cp vm/disk.img vm/disk-run.img

echo "=== Starting Orbic emulator ==="
echo "    Web UI: http://localhost:8080"
echo "    ADB:    adb connect localhost:5555"
echo "    Exit:   Ctrl-A then X"
echo ""

docker run --rm -it \
    -v "$(pwd)/vm":/vm \
    -p 8080:8080 \
    -p 5555:5555 \
    -v "$(pwd)/scripts/boot-inner.sh":/boot.sh:ro \
    debian:bullseye sh /boot.sh
