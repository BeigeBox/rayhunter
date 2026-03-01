#!/bin/bash
# Check if a Meshtastic device is detected on the Orbic via USB
#
# Usage: ./scripts/check-meshtastic.sh

set -e

ORBIC_IP="${ORBIC_IP:-192.168.1.208}"
ORBIC_SSH="root@$ORBIC_IP"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}Checking SSH connection to $ORBIC_IP...${NC}"
if ! ssh -o ConnectTimeout=3 "$ORBIC_SSH" true 2>/dev/null; then
    echo -e "${RED}Cannot reach $ORBIC_IP via SSH${NC}"
    exit 1
fi

echo -e "${YELLOW}Checking kernel modules...${NC}"
MODS=$(ssh "$ORBIC_SSH" "lsmod 2>/dev/null")
MISSING=""
for mod in ehci_hcd ehci_msm usb_host_enable cdc_acm; do
    if echo "$MODS" | grep -q "$mod"; then
        echo -e "  ${GREEN}$mod loaded${NC}"
    else
        echo -e "  ${RED}$mod NOT loaded${NC}"
        MISSING="$MISSING $mod"
    fi
done

if [ -n "$MISSING" ]; then
    echo ""
    echo -e "${RED}Missing modules:$MISSING${NC}"
    echo "Run: scripts/build-kernel-modules.sh load"
    echo "(Modules must be reloaded after every reboot)"
    exit 1
fi

echo ""
echo -e "${YELLOW}Checking OTG mode...${NC}"
MODE=$(ssh "$ORBIC_SSH" "cat /sys/kernel/debug/msm_otg/mode 2>/dev/null")
if [ "$MODE" = "host" ]; then
    echo -e "  ${GREEN}OTG mode: host${NC}"
else
    echo -e "  ${RED}OTG mode: $MODE (expected 'host')${NC}"
    echo ""
    echo "  The Orbic is not in USB host mode. This usually means a powered"
    echo "  USB hub is backfeeding VBUS, preventing the mode switch."
    echo ""
    echo "  Fix:"
    echo "    1. Unplug the USB hub/docking station from the Orbic"
    echo "    2. Run: scripts/build-kernel-modules.sh load"
    echo "    3. Plug the hub back in"
    echo "    4. Re-run this script"
    exit 1
fi

echo ""
echo -e "${YELLOW}Checking for USB devices...${NC}"
USB_DEVICES=$(ssh "$ORBIC_SSH" "cat /sys/kernel/debug/usb/devices 2>/dev/null")
DEVICE_COUNT=$(echo "$USB_DEVICES" | grep -c "^T:" || true)
if [ "$DEVICE_COUNT" -gt 1 ]; then
    echo -e "  ${GREEN}Found $((DEVICE_COUNT - 1)) USB device(s) (plus root hub)${NC}"
    echo "$USB_DEVICES" | grep -E "^[TSPCI]:" | grep -v "Bus=01 Lev=00"
else
    echo -e "  ${RED}No USB devices detected (only root hub)${NC}"
    echo ""
    echo "  Check:"
    echo "    - MeshPocket is powered on"
    echo "    - Using a powered USB hub between Orbic and MeshPocket"
    echo "    - USB cables are data-capable (not charge-only)"
    exit 1
fi

echo ""
echo -e "${YELLOW}Checking for serial devices...${NC}"
SERIAL=$(ssh "$ORBIC_SSH" "ls /dev/ttyACM* /dev/ttyUSB* 2>/dev/null || true")
if [ -n "$SERIAL" ]; then
    echo -e "  ${GREEN}Found: $SERIAL${NC}"
else
    echo -e "  ${RED}No serial devices found (/dev/ttyACM* or /dev/ttyUSB*)${NC}"
    echo "  The USB device was detected but no serial port was created."
    echo "  Check dmesg: ssh $ORBIC_SSH dmesg | tail -20"
    exit 1
fi

echo ""
echo -e "${YELLOW}Checking rayhunter config...${NC}"
SERIAL_PORT=$(ssh "$ORBIC_SSH" "grep meshtastic_serial_port /data/rayhunter/config.toml 2>/dev/null | grep -v '^#' || true")
if [ -n "$SERIAL_PORT" ]; then
    echo -e "  ${GREEN}$SERIAL_PORT${NC}"
else
    echo -e "  ${YELLOW}meshtastic_serial_port not set in config${NC}"
    echo "  Add to /data/rayhunter/config.toml:"
    echo "    meshtastic_serial_port = \"$SERIAL\""
fi

echo ""
echo -e "${GREEN}Meshtastic device is ready!${NC}"
