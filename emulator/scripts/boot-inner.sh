#!/bin/sh
# Runs inside Docker container. Installs QEMU and boots the emulator.
#
# Inputs (bind-mounted):
#   /vm/zImage, /vm/vexpress-v2p-ca15-tc1.dtb, /vm/disk-run.img

set -e

apt-get update -qq >/dev/null 2>&1
apt-get install -y -qq qemu-system-arm >/dev/null 2>&1

exec qemu-system-arm \
    -M vexpress-a15 -m 512M \
    -kernel /vm/zImage \
    -dtb /vm/vexpress-v2p-ca15-tc1.dtb \
    -append "root=/dev/mmcblk0p1 console=ttyAMA0 rootfstype=ext4 ro" \
    -drive file=/vm/disk-run.img,format=raw,if=sd \
    -net nic -net user,hostfwd=tcp::8080-:8080,hostfwd=tcp::5555-:5555 \
    -nographic \
    -no-reboot
