#!/bin/sh
# Runs inside Docker container (debian:bullseye, amd64).
# Cross-compiles Linux 3.18.48 for QEMU vexpress-a15.
#
# Inputs:  /build (cached toolchain + kernel source)
# Outputs: /out/zImage, /out/vexpress-v2p-ca15-tc1.dtb

set -e

KERNEL_VERSION="3.18.48"
TOOLCHAIN_URL="https://toolchains.bootlin.com/downloads/releases/toolchains/armv7-eabihf/tarballs/armv7-eabihf--musl--stable-2018.11-1.tar.bz2"
TOOLCHAIN_DIR="armv7-eabihf--musl--stable-2018.11-1"

apt-get update -qq
apt-get install -y -qq build-essential bc wget xz-utils bzip2 flex bison libssl-dev >/dev/null 2>&1

cd /build

if [ ! -d $TOOLCHAIN_DIR ]; then
    echo "--- Downloading Bootlin toolchain (GCC 7.3.0, armv7-eabihf, musl) ---"
    wget -q "$TOOLCHAIN_URL" -O toolchain.tar.bz2
    tar xf toolchain.tar.bz2
    rm toolchain.tar.bz2
fi

CROSS=/build/$TOOLCHAIN_DIR/bin/arm-buildroot-linux-musleabihf-
export PATH=/build/$TOOLCHAIN_DIR/bin:$PATH

if [ ! -d linux-$KERNEL_VERSION ]; then
    if [ ! -f kernel.tar.xz ]; then
        echo "--- Downloading kernel $KERNEL_VERSION ---"
        wget -q "https://cdn.kernel.org/pub/linux/kernel/v3.x/linux-$KERNEL_VERSION.tar.xz" -O kernel.tar.xz
    fi
    echo "--- Extracting kernel source ---"
    tar xf kernel.tar.xz
    rm kernel.tar.xz
fi

cd linux-$KERNEL_VERSION

# GCC 10+ defaults to -fno-common which breaks DTC in 3.18 (yylloc
# multiple definition). Append -fcommon to restore old behavior.
grep -q '\-fcommon' Makefile || sed -i '/^HOSTCFLAGS.*=/s/$/ -fcommon/' Makefile

make ARCH=arm CROSS_COMPILE=$CROSS vexpress_defconfig

# Enable options we need beyond the default
scripts/config --enable EXT4_FS
scripts/config --enable DEVTMPFS
scripts/config --enable DEVTMPFS_MOUNT
scripts/config --enable MODULES
scripts/config --enable MODULE_UNLOAD
scripts/config --enable UNIX
scripts/config --enable INET
scripts/config --enable NET
scripts/config --enable PACKET
scripts/config --enable MMC
scripts/config --enable MMC_ARMMMCI
scripts/config --enable SMSC911X

make ARCH=arm CROSS_COMPILE=$CROSS olddefconfig

echo "--- Building kernel ---"
make ARCH=arm CROSS_COMPILE=$CROSS -j$(nproc) zImage dtbs

cp arch/arm/boot/zImage /out/zImage
cp arch/arm/boot/dts/vexpress-v2p-ca15-tc1.dtb /out/vexpress-v2p-ca15-tc1.dtb

echo "--- Done ---"
ls -lh /out/zImage /out/vexpress-v2p-ca15-tc1.dtb
