#!/bin/sh
# Run on Mac with sudo to capture traffic to/from the Orbic's wlan1 IP.
# Usage: sudo sh mac-tcpdump.sh [orbic-ip] [interface]
#   orbic-ip  defaults to 192.168.1.208
#   interface defaults to en1
ORBIC_IP="${1:-192.168.1.208}"
IFACE="${2:-en1}"
SECONDS=15

echo "Capturing traffic to/from $ORBIC_IP on $IFACE for $SECONDS seconds..."
echo "Run 'ping $ORBIC_IP' from another terminal during capture."
tcpdump -i "$IFACE" -n host "$ORBIC_IP" &
PID=$!
sleep "$SECONDS"
kill $PID 2>/dev/null
echo "Done."
