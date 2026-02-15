#!/bin/bash
# Build USB host and Bluetooth kernel modules for Orbic RC400L
#
# Cross-compiles out-of-tree kernel modules in Docker (debian:buster):
#   - ehci-hcd.ko, ehci-msm.ko (USB 2.0 host controller)
#   - bluetooth.ko, ath3k.ko, btusb.ko (Bluetooth stack)
#   - usbserial.ko, cp210x.ko (USB serial for CP210x-based Meshtastic devices)
#   - cdc-acm.ko (USB CDC ACM for nRF52840-based devices like MeshPocket)
#   - usb_host_enable.ko (patches OTG driver for host mode)
#
# Usage: ./scripts/build-kernel-modules.sh [build|install|load|clean|help]
#
# Prerequisites: Docker (macOS lacks elf.h needed for kernel host tools)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

BUILD_DIR="$PROJECT_DIR/target/kernel-modules"
OUT_DIR="$BUILD_DIR/out"

KERNEL_REPO="https://github.com/the-modem-distro/quectel_eg25_kernel.git"
KERNEL_BRANCH="linux-3.18.140"
ORBIC_SUBLEVEL=48
DOCKER_IMAGE="debian:buster"

ORBIC_IP="${ORBIC_IP:-192.168.1.208}"
MODULE_PATH="/data/rayhunter/modules"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

check_docker() {
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Error: docker not found.${NC}"
        echo "Install Docker Desktop from https://www.docker.com/products/docker-desktop/"
        exit 1
    fi
    if ! docker info &> /dev/null 2>&1; then
        echo -e "${RED}Error: Docker daemon not running.${NC}"
        exit 1
    fi
}

clone_kernel() {
    if [ -d "$BUILD_DIR/quectel_eg25_kernel/.git" ]; then
        echo -e "${YELLOW}Kernel source already cloned, skipping.${NC}"
        return
    fi
    echo -e "${YELLOW}Cloning kernel source...${NC}"
    mkdir -p "$BUILD_DIR"
    git clone --depth 1 --branch "$KERNEL_BRANCH" \
        "$KERNEL_REPO" "$BUILD_DIR/quectel_eg25_kernel"
    echo -e "${GREEN}Kernel source cloned.${NC}"
}

extract_config() {
    if [ -f "$BUILD_DIR/config" ]; then
        echo -e "${YELLOW}Device config already cached, skipping.${NC}"
        return
    fi
    echo -e "${YELLOW}Extracting kernel config from device via ADB...${NC}"
    if ! command -v adb &> /dev/null; then
        echo -e "${RED}Error: adb not found.${NC}"
        echo "Install with: brew install android-platform-tools"
        echo "Or place extracted config at: $BUILD_DIR/config"
        exit 1
    fi
    if ! adb devices | grep -q "device$"; then
        echo -e "${RED}Error: No device connected via ADB.${NC}"
        echo "Or place extracted config at: $BUILD_DIR/config"
        exit 1
    fi
    mkdir -p "$BUILD_DIR"
    adb shell "cat /proc/config.gz | base64" | base64 -d | gunzip > "$BUILD_DIR/config"
    echo -e "${GREEN}Config extracted.${NC}"
}

# The custom usb_host_enable module patches the OTG driver's platform data
# from USB_PERIPHERAL to USB_OTG at runtime, installs a VBUS power stub,
# and creates the msm_hsusb_host platform device that ehci-msm binds to.
write_usb_host_enable() {
    mkdir -p "$BUILD_DIR/usb_host_enable"
    cat > "$BUILD_DIR/usb_host_enable/Makefile" << 'EOF'
obj-m := usb_host_enable.o
EOF
    cat > "$BUILD_DIR/usb_host_enable/usb_host_enable.c" << 'EOF'
/*
 * usb_host_enable.c - Enable USB host mode on Orbic RC400L
 *
 * The Orbic's device tree sets qcom,hsusb-otg-mode=1 (USB_PERIPHERAL),
 * which prevents the OTG driver from creating the host platform device
 * and rejects host registration. This module:
 * 1. Patches pdata->mode to USB_OTG to allow host registration
 * 2. Creates the msm_hsusb_host platform device
 * 3. Triggers ehci-msm to probe and register with the OTG transceiver
 */
#include <linux/module.h>
#include <linux/platform_device.h>
#include <linux/dma-mapping.h>
#include <linux/of.h>
#include <linux/of_platform.h>
#include <linux/usb/msm_hsusb.h>

static struct platform_device *host_pdev;
static u64 host_dma_mask = DMA_BIT_MASK(32);
static enum usb_mode_type orig_mode;
static struct msm_otg_platform_data *pdata_ref;

static int stub_vbus_power(bool on)
{
	pr_info("usb_host_enable: vbus_power(%d) stub\n", on);
	return 0;
}
static int (*orig_vbus_power)(bool on);

static int __init usb_host_enable_init(void)
{
	struct device_node *np;
	struct platform_device *otg_pdev;
	struct msm_otg_platform_data *pdata;
	int ret;

	np = of_find_compatible_node(NULL, NULL, "qcom,hsusb-otg");
	if (!np) {
		pr_err("usb_host_enable: no qcom,hsusb-otg node\n");
		return -ENODEV;
	}

	otg_pdev = of_find_device_by_node(np);
	of_node_put(np);
	if (!otg_pdev) {
		pr_err("usb_host_enable: no OTG platform device\n");
		return -ENODEV;
	}

	pdata = otg_pdev->dev.platform_data;
	if (!pdata) {
		pr_err("usb_host_enable: no platform data\n");
		put_device(&otg_pdev->dev);
		return -ENODEV;
	}

	orig_mode = pdata->mode;
	orig_vbus_power = pdata->vbus_power;
	pdata_ref = pdata;
	pr_info("usb_host_enable: patching OTG mode from %d to %d (USB_OTG)\n",
		orig_mode, USB_OTG);
	pdata->mode = USB_OTG;
	pdata->vbus_power = stub_vbus_power;

	host_pdev = platform_device_alloc("msm_hsusb_host", -1);
	if (!host_pdev) {
		pdata->mode = orig_mode;
		put_device(&otg_pdev->dev);
		return -ENOMEM;
	}

	host_pdev->dev.coherent_dma_mask = DMA_BIT_MASK(32);
	host_pdev->dev.dma_mask = &host_dma_mask;
	host_pdev->dev.parent = &otg_pdev->dev;

	if (otg_pdev->num_resources) {
		ret = platform_device_add_resources(host_pdev,
			otg_pdev->resource, otg_pdev->num_resources);
		if (ret) {
			pr_err("usb_host_enable: add resources failed: %d\n", ret);
			platform_device_put(host_pdev);
			pdata->mode = orig_mode;
			put_device(&otg_pdev->dev);
			host_pdev = NULL;
			return ret;
		}
	}

	ret = platform_device_add(host_pdev);
	if (ret) {
		pr_err("usb_host_enable: add device failed: %d\n", ret);
		platform_device_put(host_pdev);
		pdata->mode = orig_mode;
		put_device(&otg_pdev->dev);
		host_pdev = NULL;
		return ret;
	}

	put_device(&otg_pdev->dev);
	pr_info("usb_host_enable: USB host mode enabled\n");
	return 0;
}

static void __exit usb_host_enable_exit(void)
{
	if (host_pdev)
		platform_device_unregister(host_pdev);
	if (pdata_ref) {
		pdata_ref->mode = orig_mode;
		pdata_ref->vbus_power = orig_vbus_power;
	}
	pr_info("usb_host_enable: USB host mode disabled\n");
}

module_init(usb_host_enable_init);
module_exit(usb_host_enable_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Enable USB host mode on Orbic RC400L");
EOF
}

build_modules() {
    check_docker
    clone_kernel
    extract_config
    write_usb_host_enable

    echo -e "${YELLOW}Building kernel modules in Docker...${NC}"
    mkdir -p "$OUT_DIR"

    docker run --rm --platform linux/amd64 \
        -v "$BUILD_DIR/quectel_eg25_kernel:/src" \
        -v "$BUILD_DIR/config:/config:ro" \
        -v "$BUILD_DIR/usb_host_enable:/usb_host_enable" \
        -v "$OUT_DIR:/out" \
        -w /src "$DOCKER_IMAGE" bash -c "$(cat << 'DOCKERSCRIPT'
set -e

# Debian Buster repos are archived
echo "deb http://archive.debian.org/debian buster main" > /etc/apt/sources.list
echo "deb http://archive.debian.org/debian-security buster/updates main" >> /etc/apt/sources.list
apt-get -o Acquire::Check-Valid-Until=false update -qq
apt-get install -y -qq gcc-arm-linux-gnueabi binutils-arm-linux-gnueabi \
    build-essential bc > /dev/null 2>&1

# Patch SUBLEVEL to match Orbic vermagic (3.18.48)
sed -i 's/^SUBLEVEL = 140$/SUBLEVEL = 48/' Makefile

# Patch ehci-msm.c: msm_bam_set_usb_host_dev is not exported,
# resolve it at runtime via kallsyms
if ! grep -q kallsyms drivers/usb/host/ehci-msm.c; then
    sed -i '/#include <linux\/usb\/hcd.h>/a #include <linux/kallsyms.h>' \
        drivers/usb/host/ehci-msm.c
    sed -i 's/\tmsm_bam_set_usb_host_dev(\&pdev->dev);/\t{\n\t\tvoid (*bam_fn)(struct device *) = (void (*)(struct device *))\n\t\t\tkallsyms_lookup_name("msm_bam_set_usb_host_dev");\n\t\tif (bam_fn)\n\t\t\tbam_fn(\&pdev->dev);\n\t}/' \
        drivers/usb/host/ehci-msm.c
fi

# Apply device config and enable modules
cp /config .config
sed -i 's/# CONFIG_USB_EHCI_HCD is not set/CONFIG_USB_EHCI_HCD=m/' .config
sed -i 's/# CONFIG_BT is not set/CONFIG_BT=m/' .config
echo "CONFIG_USB_EHCI_MSM=m" >> .config
echo "CONFIG_USB_EHCI_ROOT_HUB_TT=y" >> .config
echo "CONFIG_USB_EHCI_TT_NEWSCHED=y" >> .config
echo "CONFIG_BT_HCIBTUSB=m" >> .config
echo "CONFIG_BT_ATH3K=m" >> .config
echo "CONFIG_USB_SERIAL=m" >> .config
echo "CONFIG_USB_SERIAL_CP210X=m" >> .config
echo "CONFIG_USB_ACM=m" >> .config

export ARCH=arm CROSS_COMPILE=arm-linux-gnueabi-
CC=arm-linux-gnueabi-gcc

# CC= override bypasses scripts/gcc-wrapper.py (needs python2)
make CC=$CC oldconfig < /dev/null 2>&1 | tail -3
echo "Preparing kernel build..."
make CC=$CC modules_prepare -j$(nproc) 2>&1 | tail -3

echo "Building USB host modules..."
make CC=$CC M=drivers/usb/host modules -j$(nproc) 2>&1 | tail -3

echo "Building Bluetooth modules..."
make CC=$CC M=net/bluetooth modules -j$(nproc) 2>&1 | tail -3
make CC=$CC M=drivers/bluetooth modules -j$(nproc) 2>&1 | tail -3

echo "Building USB serial modules..."
make CC=$CC M=drivers/usb/serial modules -j$(nproc) 2>&1 | tail -3

echo "Building USB CDC ACM module..."
make CC=$CC M=drivers/usb/class modules -j$(nproc) 2>&1 | tail -3

echo "Building usb_host_enable module..."
make CC=$CC M=/usb_host_enable -C /src modules -j$(nproc) 2>&1 | tail -3

MODULES="
    drivers/usb/host/ehci-hcd.ko
    drivers/usb/host/ehci-msm.ko
    net/bluetooth/bluetooth.ko
    drivers/bluetooth/ath3k.ko
    drivers/bluetooth/btusb.ko
    drivers/usb/serial/usbserial.ko
    drivers/usb/serial/cp210x.ko
    drivers/usb/class/cdc-acm.ko
    /usb_host_enable/usb_host_enable.ko
"
echo ""
echo "Collecting modules:"
for mod in $MODULES; do
    if [ -f "$mod" ]; then
        arm-linux-gnueabi-strip --strip-debug "$mod"
        cp "$mod" /out/
        SIZE=$(du -h "$mod" | cut -f1)
        echo "  $(basename $mod) ($SIZE)"
    else
        echo "  WARNING: $mod not found"
    fi
done
DOCKERSCRIPT
)"

    echo ""
    echo -e "${GREEN}Build complete. Modules in $OUT_DIR:${NC}"
    ls -lh "$OUT_DIR"/*.ko 2>/dev/null
}

install_modules() {
    if [ ! -d "$OUT_DIR" ] || ! ls "$OUT_DIR"/*.ko &>/dev/null; then
        echo -e "${RED}Error: No modules found. Run '$0 build' first.${NC}"
        exit 1
    fi

    echo -e "${YELLOW}Installing modules to $ORBIC_IP:$MODULE_PATH ...${NC}"
    ssh "root@$ORBIC_IP" "mkdir -p $MODULE_PATH"

    for mod in "$OUT_DIR"/*.ko; do
        NAME=$(basename "$mod")
        echo -n "  $NAME... "
        base64 -i "$mod" | ssh "root@$ORBIC_IP" "base64 -d > $MODULE_PATH/$NAME"
        echo "ok"
    done

    echo -n "  load-modules.sh... "
    ssh "root@$ORBIC_IP" "cat > $MODULE_PATH/load-modules.sh && chmod 755 $MODULE_PATH/load-modules.sh" << 'LOADER'
#!/bin/sh
MP=/data/rayhunter/modules
insmod $MP/ehci-hcd.ko 2>/dev/null || true
insmod $MP/ehci-msm.ko 2>/dev/null || true
insmod $MP/bluetooth.ko 2>/dev/null || true
insmod $MP/ath3k.ko 2>/dev/null || true
insmod $MP/btusb.ko 2>/dev/null || true
insmod $MP/usbserial.ko 2>/dev/null || true
insmod $MP/cp210x.ko 2>/dev/null || true
insmod $MP/cdc-acm.ko 2>/dev/null || true
insmod $MP/usb_host_enable.ko 2>/dev/null || true
sleep 3
LOADER
    echo "ok"

    echo -e "${GREEN}Modules installed.${NC}"
    echo ""
    echo "Load with:  $0 load"
    echo "Or on device: sh $MODULE_PATH/load-modules.sh"
}

load_modules() {
    echo -e "${YELLOW}Loading modules on $ORBIC_IP...${NC}"
    ssh "root@$ORBIC_IP" << 'LOADSCRIPT'
MP=/data/rayhunter/modules

insmod $MP/ehci-hcd.ko 2>/dev/null || true
insmod $MP/ehci-msm.ko 2>/dev/null || true
insmod $MP/bluetooth.ko 2>/dev/null || true
insmod $MP/ath3k.ko 2>/dev/null || true
insmod $MP/btusb.ko 2>/dev/null || true
insmod $MP/usbserial.ko 2>/dev/null || true
insmod $MP/cp210x.ko 2>/dev/null || true
insmod $MP/cdc-acm.ko 2>/dev/null || true
insmod $MP/usb_host_enable.ko 2>/dev/null || true

lsmod | grep -E "ehci|bluetooth|btusb|ath3k|usbserial|cp210x|cdc.acm|usb_host" || true
LOADSCRIPT

    echo ""
    echo -e "${YELLOW}Switching to USB host mode...${NC}"
    OTG_MODE=$(ssh "root@$ORBIC_IP" "cat /sys/kernel/debug/msm_otg/mode 2>/dev/null || echo unknown")
    if [ "$OTG_MODE" = "host" ]; then
        echo -e "  ${GREEN}Already in host mode${NC}"
    else
        ssh "root@$ORBIC_IP" "echo none > /sys/kernel/debug/msm_otg/mode; sleep 1; echo host > /sys/kernel/debug/msm_otg/mode; sleep 1"
        OTG_MODE=$(ssh "root@$ORBIC_IP" "cat /sys/kernel/debug/msm_otg/mode 2>/dev/null || echo unknown")
        if [ "$OTG_MODE" = "host" ]; then
            echo -e "  ${GREEN}OTG mode: host${NC}"
        else
            echo -e "  ${RED}OTG mode: $OTG_MODE (expected 'host')${NC}"
            echo ""
            echo "  A powered USB hub may be backfeeding VBUS, preventing host mode."
            echo "  Unplug the hub from the Orbic, re-run this command, then plug"
            echo "  the hub back in."
        fi
    fi
    echo ""
    echo -e "${GREEN}Done.${NC}"
}

do_clean() {
    echo -e "${YELLOW}Cleaning build directory...${NC}"
    rm -rf "$BUILD_DIR"
    echo -e "${GREEN}Cleaned.${NC}"
}

show_usage() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  build     Build kernel modules in Docker (default)"
    echo "  install   Push modules to device via SSH"
    echo "  load      Load modules and activate USB host mode"
    echo "  clean     Remove build artifacts"
    echo ""
    echo "Environment variables:"
    echo "  ORBIC_IP  Device IP address (default: 192.168.1.208)"
    echo ""
    echo "Module load order:"
    echo "  ehci-hcd -> ehci-msm -> bluetooth -> ath3k -> btusb -> usbserial -> cp210x -> cdc-acm -> usb_host_enable"
    echo ""
    echo "After loading, the script transitions the OTG state machine to host"
    echo "mode by writing 'none' then 'host' to /sys/kernel/debug/msm_otg/mode."
    echo "ADB over USB will be unavailable while in host mode."
}

COMMAND="${1:-build}"

case "$COMMAND" in
    build)
        build_modules
        ;;
    install)
        install_modules
        ;;
    load)
        load_modules
        ;;
    clean)
        do_clean
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        echo -e "${RED}Unknown command: $COMMAND${NC}"
        show_usage
        exit 1
        ;;
esac
