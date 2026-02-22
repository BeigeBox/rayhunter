#!/bin/sh
# Dump the Orbic RC400L's filesystems and metadata for emulator reference.
#
# Prerequisites:
#   - SSH access to the Orbic (dropbear running, key auth configured)
#   - ADB connected to the Orbic (for UBI volume dumps)
#   - /bin/su on the Orbic linked to rootshell (for ADB root access)
#
# Output goes to emulator/reference/. Nothing is written on-device;
# tar streams to stdout and ADB exec-out pipes directly to host files.
#
# Usage: ./emulator/scripts/dump-real-device.sh [ssh-host]
#   ssh-host defaults to root@192.168.1.208 (Orbic in wifi client mode)

set -e

HOST="${1:-root@192.168.1.208}"
OUT="emulator/reference"

mkdir -p "$OUT/mtd"

echo "=== Collecting metadata via SSH ==="
ssh "$HOST" 'cat /proc/version'            > "$OUT/kernel_version.txt"
ssh "$HOST" 'cat /proc/mtd'                > "$OUT/mtd.txt"
ssh "$HOST" 'cat /proc/mounts'             > "$OUT/mounts.txt"
ssh "$HOST" 'df -k'                        > "$OUT/df.txt"
ssh "$HOST" 'ubinfo -a 2>/dev/null'        > "$OUT/ubinfo.txt"
ssh "$HOST" 'cat /etc/passwd'              > "$OUT/passwd.txt"
ssh "$HOST" 'cat /etc/group'               > "$OUT/group.txt"
ssh "$HOST" 'ls -la /dev/diag'             > "$OUT/diag_stat.txt"
ssh "$HOST" 'cat /etc/inittab 2>/dev/null' > "$OUT/inittab.txt"

echo "=== Dumping root filesystem via SSH (tar to stdout) ==="
ssh "$HOST" 'cd / && tar czf - \
    --exclude proc --exclude sys --exclude dev \
    --exclude data --exclude usrdata --exclude firmware \
    --exclude cache --exclude run \
    --exclude var/volatile --exclude media/ram .' \
    > "$OUT/rootfs.tar.gz"
echo "  rootfs.tar.gz: $(du -h "$OUT/rootfs.tar.gz" | cut -f1)"

echo "=== Dumping /data via SSH ==="
ssh "$HOST" 'cd /data && tar czf - .' > "$OUT/data.tar.gz"
echo "  data.tar.gz: $(du -h "$OUT/data.tar.gz" | cut -f1)"

echo "=== Dumping /firmware via SSH ==="
ssh "$HOST" 'cd /firmware && tar czf - .' > "$OUT/firmware.tar.gz"
echo "  firmware.tar.gz: $(du -h "$OUT/firmware.tar.gz" | cut -f1)"

echo "=== Dumping raw bootloader (mtd0/sbl, not UBI-managed) via SSH ==="
ssh "$HOST" 'cat /dev/mtdblock0' > "$OUT/mtd/mtd0_sbl.img"
echo "  mtd0_sbl.img: $(du -h "$OUT/mtd/mtd0_sbl.img" | cut -f1)"

echo "=== Dumping UBI volumes via ADB (faster than SSH) ==="
for vol in "ubi0_0:rootfs" "ubi0_1:usrfs" "ubi0_2:cachefs" "ubi1_0:modem" "ubi3_0:usrdata"; do
    dev="${vol%%:*}"
    name="${vol##*:}"
    outfile="$OUT/mtd/${dev}_${name}.img"
    echo "  /dev/$dev -> $outfile"
    adb exec-out "su -c \"dd if=/dev/$dev bs=131072 2>/dev/null\"" > "$outfile"
    echo "    $(du -h "$outfile" | cut -f1)"
done

echo "=== Done. Reference data in $OUT/ ==="
du -sh "$OUT"
