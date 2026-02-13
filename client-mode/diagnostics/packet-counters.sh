#!/bin/sh
# Quick check of packet counters on the Orbic.
# Run via: adb shell sh /path/to/packet-counters.sh
# Then ping from your Mac, then run again to compare.
echo "=== /proc/net/dev (wlan1, bridge0) ==="
echo "iface     RX_bytes RX_pkts RX_errs RX_drop ... TX_bytes TX_pkts TX_errs TX_drop"
cat /proc/net/dev | grep -E 'wlan1|bridge0'
