#!/bin/bash
# Build and deploy rayhunter + kernel modules to Orbic via SSH
#
# Usage: ./scripts/deploy-meshtastic.sh [build|push|both]
#
# Builds the daemon for ARM, then pushes daemon binary + kernel modules
# to the device over SSH, loads modules, and restarts rayhunter.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

ORBIC_IP="${ORBIC_IP:-192.168.1.208}"
ORBIC_SSH="root@$ORBIC_IP"
MODULE_PATH="/data/rayhunter/modules"
DAEMON_PATH="/data/rayhunter"
DAEMON_BIN="$PROJECT_DIR/target/armv7-unknown-linux-musleabihf/firmware-devel/rayhunter-daemon"
MODULE_DIR="$PROJECT_DIR/target/kernel-modules/out"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

build_daemon() {
    echo -e "${YELLOW}Building daemon for ARM...${NC}"
    cd "$PROJECT_DIR"
    cargo build-daemon-firmware-devel
    echo -e "${GREEN}Daemon built: $(ls -lh "$DAEMON_BIN" | awk '{print $5}')${NC}"
}

build_modules() {
    if [ -d "$MODULE_DIR" ] && ls "$MODULE_DIR"/*.ko &>/dev/null; then
        echo -e "${YELLOW}Kernel modules already built, skipping (run 'scripts/build-kernel-modules.sh clean' to rebuild)${NC}"
        return
    fi
    echo -e "${YELLOW}Building kernel modules...${NC}"
    "$SCRIPT_DIR/build-kernel-modules.sh" build
}

push_to_device() {
    echo -e "${YELLOW}Checking SSH connection to $ORBIC_IP...${NC}"
    if ! ssh -o ConnectTimeout=3 "$ORBIC_SSH" true 2>/dev/null; then
        echo -e "${RED}Cannot reach $ORBIC_IP via SSH${NC}"
        echo "Set ORBIC_IP if using a different address"
        exit 1
    fi

    echo -e "${YELLOW}Stopping rayhunter...${NC}"
    ssh "$ORBIC_SSH" "/etc/init.d/rayhunter_daemon stop 2>/dev/null || true"
    sleep 1

    echo -e "${YELLOW}Pushing daemon binary...${NC}"
    base64 -i "$DAEMON_BIN" | ssh "$ORBIC_SSH" "base64 -d > $DAEMON_PATH/rayhunter-daemon && chmod +x $DAEMON_PATH/rayhunter-daemon"
    echo -e "${GREEN}  rayhunter-daemon pushed${NC}"

    if [ -d "$MODULE_DIR" ] && ls "$MODULE_DIR"/*.ko &>/dev/null; then
        echo -e "${YELLOW}Pushing kernel modules...${NC}"
        ssh "$ORBIC_SSH" "mkdir -p $MODULE_PATH"
        for mod in "$MODULE_DIR"/*.ko; do
            NAME=$(basename "$mod")
            echo -n "  $NAME... "
            base64 -i "$mod" | ssh "$ORBIC_SSH" "base64 -d > $MODULE_PATH/$NAME"
            echo "ok"
        done
    fi

    echo -e "${YELLOW}Loading kernel modules...${NC}"
    ssh "$ORBIC_SSH" << 'LOADSCRIPT'
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

lsmod 2>/dev/null | grep -E "ehci|bluetooth|btusb|ath3k|usbserial|cp210x|cdc.acm|usb_host" || true
LOADSCRIPT

    echo -e "${YELLOW}Switching to USB host mode...${NC}"
    OTG_MODE=$(ssh "$ORBIC_SSH" "cat /sys/kernel/debug/msm_otg/mode 2>/dev/null || echo unknown")
    if [ "$OTG_MODE" = "host" ]; then
        echo -e "  ${GREEN}Already in host mode${NC}"
    else
        ssh "$ORBIC_SSH" "echo none > /sys/kernel/debug/msm_otg/mode; sleep 1; echo host > /sys/kernel/debug/msm_otg/mode; sleep 1"
        OTG_MODE=$(ssh "$ORBIC_SSH" "cat /sys/kernel/debug/msm_otg/mode 2>/dev/null || echo unknown")
        if [ "$OTG_MODE" = "host" ]; then
            echo -e "  ${GREEN}OTG mode: host${NC}"
        else
            echo -e "  ${RED}OTG mode: $OTG_MODE (expected 'host')${NC}"
            echo ""
            echo -e "  ${YELLOW}If a powered USB hub is connected, it may be backfeeding VBUS${NC}"
            echo -e "  ${YELLOW}which prevents host mode. Try:${NC}"
            echo "    1. Unplug the USB hub from the Orbic"
            echo "    2. Run: scripts/build-kernel-modules.sh load"
            echo "    3. Plug the hub back in"
        fi
    fi

    echo -e "${YELLOW}Starting rayhunter...${NC}"
    ssh "$ORBIC_SSH" "/etc/init.d/rayhunter_daemon start"

    echo ""
    echo -e "${GREEN}Done! Rayhunter deployed with Meshtastic support.${NC}"
    echo ""
    echo -e "${YELLOW}Note: Kernel modules must be reloaded after every reboot.${NC}"
    echo -e "${YELLOW}Run: scripts/build-kernel-modules.sh load${NC}"
    echo ""
    echo -e "${YELLOW}Make sure config.toml has: meshtastic_serial_port = \"/dev/ttyACM0\"${NC}"
    echo -e "${YELLOW}(Use /dev/ttyUSB0 for CP210x-based devices like Heltec V3)${NC}"
}

show_usage() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  build   Build daemon + kernel modules only"
    echo "  push    Push to device and restart (requires prior build)"
    echo "  both    Build and push (default)"
    echo ""
    echo "Environment:"
    echo "  ORBIC_IP  Device IP (default: 192.168.225.1)"
}

COMMAND="${1:-both}"

case "$COMMAND" in
    build)
        build_modules
        build_daemon
        ;;
    push)
        push_to_device
        ;;
    both)
        build_modules
        build_daemon
        push_to_device
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
