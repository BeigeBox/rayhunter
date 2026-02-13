#!/bin/sh
# Network diagnostics for WiFi client mode on the Orbic.
# Run via AT+SYSCMD. Output goes to /tmp/net-diag.txt.
exec > /tmp/net-diag.txt 2>&1

echo "=== interfaces ==="
ip addr show wlan1
ip addr show bridge0

echo "=== routing ==="
ip route show
ip rule show
ip route show table 100 2>/dev/null

echo "=== iptables filter INPUT ==="
iptables -L INPUT -n -v

echo "=== iptables filter FORWARD ==="
iptables -L FORWARD -n -v

echo "=== iptables mangle ==="
iptables -t mangle -L -n -v

echo "=== iptables raw ==="
iptables -t raw -L -n -v

echo "=== iptables nat ==="
iptables -t nat -L -n -v

echo "=== bridge ==="
brctl show
cat /proc/sys/net/bridge/bridge-nf-call-iptables

echo "=== rp_filter ==="
cat /proc/sys/net/ipv4/conf/all/rp_filter
cat /proc/sys/net/ipv4/conf/wlan1/rp_filter

echo "=== wpa_supplicant ==="
cat /tmp/wpa_sta.pid 2>/dev/null && echo "running" || echo "not running"

echo "=== connectivity ==="
wget -q -O /dev/null http://detectportal.firefox.com/success.txt && echo "internet: OK" || echo "internet: FAILED"
