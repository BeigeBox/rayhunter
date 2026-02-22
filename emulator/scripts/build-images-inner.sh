#!/bin/sh
# Runs inside Docker container. Creates a single partitioned disk image
# from the reference tarballs.
#
# Partition layout (MBR):
#   p1: 128 MB ext4, rootfs (mounted ro by kernel)
#   p2: 220 MB ext4, /data  (mounted rw by init)
#
# Inputs (bind-mounted): /reference/{rootfs,data}.tar.gz, /config.toml.in
# Output (bind-mounted): /vm/disk.img

set -e

apk add --no-cache e2fsprogs sfdisk >/dev/null

DISK=/vm/disk.img
ROOTFS_MB=128
# QEMU requires SD images to be a power of 2
TOTAL_MB=512

echo "--- Creating ${TOTAL_MB} MB disk image ---"
dd if=/dev/zero of=$DISK bs=1M count=$TOTAL_MB status=none

# p1: 128 MB rootfs, p2: rest of disk for data
sfdisk $DISK <<EOF
label: dos
1M,${ROOTFS_MB}M,L
,,L
EOF

# Extract partition offsets from sfdisk output (in sectors, 512 bytes each)
P1_START=$(sfdisk -d $DISK | grep 'img1' | sed 's/.*start= *\([0-9]*\).*/\1/')
P1_SIZE=$(sfdisk -d $DISK | grep 'img1' | sed 's/.*size= *\([0-9]*\).*/\1/')
P2_START=$(sfdisk -d $DISK | grep 'img2' | sed 's/.*start= *\([0-9]*\).*/\1/')
P2_SIZE=$(sfdisk -d $DISK | grep 'img2' | sed 's/.*size= *\([0-9]*\).*/\1/')

P1_OFFSET=$((P1_START * 512))
P1_BYTES=$((P1_SIZE * 512))
P2_OFFSET=$((P2_START * 512))
P2_BYTES=$((P2_SIZE * 512))

# Create individual partition images, then dd them into the disk image
echo "--- Building rootfs partition ---"
dd if=/dev/zero of=/tmp/rootfs.ext4 bs=1 count=0 seek=$P1_BYTES status=none
mkfs.ext4 -q -O ^metadata_csum,^64bit /tmp/rootfs.ext4
mkdir -p /mnt/rootfs
mount -o loop /tmp/rootfs.ext4 /mnt/rootfs
tar xpzf /reference/rootfs.tar.gz -C /mnt/rootfs

# Create mount point directories excluded from the tar dump
mkdir -p /mnt/rootfs/dev /mnt/rootfs/proc /mnt/rootfs/sys
mkdir -p /mnt/rootfs/data /mnt/rootfs/cache /mnt/rootfs/run
mkdir -p /mnt/rootfs/firmware /mnt/rootfs/usrdata

rm -rf /mnt/rootfs/etc/dropbear
rm -f /mnt/rootfs/root/.ssh/authorized_keys
rm -f /mnt/rootfs/etc/wpa_supplicant.conf

if [ -f /mnt/rootfs/etc/inittab ]; then
    sed -i "s|ttyHSL0|ttyAMA0|g" /mnt/rootfs/etc/inittab
fi

umount /mnt/rootfs
dd if=/tmp/rootfs.ext4 of=$DISK bs=512 seek=$P1_START conv=notrunc status=none
rm /tmp/rootfs.ext4

echo "--- Building data partition ---"
dd if=/dev/zero of=/tmp/data.ext4 bs=1 count=0 seek=$P2_BYTES status=none
mkfs.ext4 -q -O ^metadata_csum,^64bit /tmp/data.ext4
mkdir -p /mnt/data
mount -o loop /tmp/data.ext4 /mnt/data
tar xpzf /reference/data.tar.gz -C /mnt/data

rm -f /mnt/data/rayhunter/wifi-creds.conf
rm -f /mnt/data/rayhunter/wpa_sta.conf
rm -f /mnt/data/rayhunter-data/wifi-creds.conf
rm -f /mnt/data/rayhunter-data/wpa_sta.conf
rm -rf /mnt/data/rayhunter/qmdl/*
rm -rf /mnt/data/rayhunter-data/qmdl/*
rm -rf /mnt/data/rayhunter/crash-logs/*
rm -rf /mnt/data/rayhunter-data/crash-logs/*

for dir in /mnt/data/rayhunter /mnt/data/rayhunter-data; do
    if [ -d "$dir" ]; then
        cp /config.toml.in "$dir/config.toml"
    fi
done

umount /mnt/data
dd if=/tmp/data.ext4 of=$DISK bs=512 seek=$P2_START conv=notrunc status=none
rm /tmp/data.ext4

echo "--- Done ---"
