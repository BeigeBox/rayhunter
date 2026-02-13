# WiFi Client Mode for Rayhunter (Orbic RC400L)

Connect the Orbic to an existing WiFi network while keeping its AP running.
This enables internet access (for ntfy notifications, etc.) and allows
accessing the Rayhunter web UI from any device on your network without
connecting to the Orbic's AP directly.

## How It Works

The Orbic's WiFi chip (QCA6174) supports concurrent AP + station mode.
`wlan0` runs the AP (via hostapd/QCMAP), and `wlan1` is a spare interface
we configure as a station (client) using a cross-compiled `wpa_supplicant`.

## Prerequisites

- Rayhunter installed on an Orbic RC400L
- ADB access to the device
- wpa_supplicant built for ARMv7 (see [Building wpa_supplicant](#building-wpa_supplicant))

## Quick Start

1. Build wpa_supplicant (one-time):
   ```
   cd tools/build-wpa-supplicant
   docker build --platform linux/amd64 --target export --output type=local,dest=./out .
   arm-linux-gnueabihf-strip out/wpa_supplicant out/wpa_cli
   ```

2. Push files to device:
   ```
   sh client-mode/scripts/setup-device.sh
   ```

3. Create credentials on the device:
   ```
   adb shell
   cat > /data/rayhunter/wifi-creds.conf <<EOF
   ssid=YourNetworkName
   password=YourPassword
   EOF
   ```

4. Reboot the device. WiFi client starts automatically on boot (see
   [Auto-Start on Boot](#auto-start-on-boot)). Wait ~60 seconds for it
   to connect, then check the log:
   ```
   adb shell cat /tmp/wifi-client.log
   ```

## File Layout on Device

```
/data/rayhunter/
  bin/wpa_supplicant        # Static ARMv7 binary (1.0MB stripped)
  bin/wpa_cli               # Static ARMv7 binary (450KB stripped)
  scripts/wifi-client.sh    # Main script (start/stop/status)
  scripts/run-wifi.sh       # Short wrapper for AT+SYSCMD
  wifi-creds.conf           # Your WiFi credentials (user-created)
```

## Building wpa_supplicant

Built from official w1.fi source (v2.11), statically linked for ARMv7:

```
cd tools/build-wpa-supplicant
docker build --platform linux/amd64 --target export --output type=local,dest=./out .
arm-linux-gnueabihf-strip out/wpa_supplicant out/wpa_cli
```

Produces `out/wpa_supplicant` (~1.0MB) and `out/wpa_cli` (~450KB).

## Script Management

```sh
# Start WiFi client (run via AT+SYSCMD after each boot)
sh /data/rayhunter/scripts/wifi-client.sh start

# Stop and disconnect
sh /data/rayhunter/scripts/wifi-client.sh stop

# Check status (prints IP or "disconnected")
sh /data/rayhunter/scripts/wifi-client.sh status
```

## AT+SYSCMD Details

Commands requiring `CAP_NET_ADMIN` (iw, iptables, ip rule/route) cannot run
through rootshell (ADB's capability bounding set is too restrictive). Instead,
we send them through the modem's AT command interface which runs with full
capabilities.

**Critical details:**
- AT+SYSCMD via `/dev/smd8` is **one-shot per boot** -- only the first
  command executes; subsequent writes are silently ignored. There is no
  error or feedback when a second command is sent; it just does nothing.
- The command **must** be framed with `\r\n` (the modem's AT parser needs
  carriage returns, not just newlines). Using `echo` only sends `\n` and
  the command will be silently ignored. Always use `printf`.
- The correct invocation from ADB:
  ```
  adb shell "rootshell -c 'printf \"\r\nAT+SYSCMD=sh /data/rayhunter/scripts/run-wifi.sh\r\n\" > /dev/smd8'"
  ```
- The Rayhunter installer uses USB bulk transfers (not smd8) and can send
  multiple AT commands per boot via proper USB control message handshaking.
  The installer sends `\r\n{command}\r\n` with a USB control message
  (0x22, 3, 1) to set up the serial port before each write.

### rootshell Quoting

rootshell is a setuid binary that execs `/bin/bash` with all remaining
arguments. This creates a quoting minefield:

- **Correct:** `rootshell -c 'your command here'`
  This runs `bash -c 'your command here'`.
- **Wrong:** `rootshell sh -c 'your command here'`
  This runs `bash sh -c '...'`, which makes bash try to execute a
  **file** named `sh` as a script. It fails silently with no output.
- When combining with `adb shell`, you get three layers of quoting
  (local shell -> adb shell -> bash via rootshell). Use double quotes
  for the outer adb layer and single quotes for the inner rootshell
  layer:
  ```
  adb shell "rootshell -c 'command with args'"
  ```
- For commands with special characters (redirects, pipes), the inner
  single quotes protect them from the adb shell layer:
  ```
  adb shell "rootshell -c 'echo something > /dev/smd8'"
  ```
- For printf with escape sequences, you need escaped inner double quotes:
  ```
  adb shell "rootshell -c 'printf \"\r\nAT+SYSCMD=...\r\n\" > /dev/smd8'"
  ```

### Capability Limitations

ADB shell has a capability bounding set of 0xc0 (only `CAP_SETUID` and
`CAP_SETGID`). rootshell inherits this limit -- it gives you uid 0 but
**not** full root capabilities. Commands that need `CAP_NET_ADMIN`
(iw, iptables, ip rule/route) will fail with EPERM even as uid 0.

AT+SYSCMD runs through the modem daemon which has the full capability set
(0x3fffffffff), so it can run anything. This is why the wifi-client.sh
script must be triggered via AT+SYSCMD rather than rootshell directly.

### Why a Wrapper Script

AT+SYSCMD has a command length limit and the path to wifi-client.sh with
its argument can be too long. The run-wifi.sh wrapper (just one line:
`sh /data/rayhunter/scripts/wifi-client.sh start`) keeps the AT command
short and reliable.

## What the Script Does

1. Sets wlan1 to managed (station) mode via `iw`
2. Starts wpa_supplicant with WPA2-PSK credentials
3. Obtains an IP via DHCP (udhcpc)
4. Fixes routing:
   - Replaces bridge0's default route with one through wlan1
   - Adds **policy routing** (table 100) so replies from wlan1's IP always
     go back out wlan1 (required because bridge0 shares the same subnet)
5. Sets DNS to 8.8.8.8 (device default is 127.0.0.1 which doesn't resolve)
6. Opens iptables INPUT and FORWARD for wlan1 (QCMAP's default policy is DROP)
7. Disables bridge-nf-call-iptables

## Key Technical Notes

### Concurrent AP+STA

`iw phy phy0 info` shows the chip supports `#{ managed } <= 2, #{ AP } <= 2,
total <= 4, #channels <= 2`. QCMAP does not interfere with wlan1 type changes.

### Duplicate Subnet Problem

bridge0 (AP) defaults to 192.168.1.1/24. If your home network is also
192.168.1.0/24, routing becomes ambiguous. The script handles this with:

- **Default route replacement**: Outbound internet goes through wlan1
- **Policy routing (table 100)**: Ensures reply packets (e.g. ICMP echo
  replies, TCP SYN-ACK) are sourced from wlan1's IP and routed out wlan1,
  not bridge0. Without this, the kernel picks bridge0's route (added first)
  and replies never reach the sender.

### iptables

QCMAP configures iptables with INPUT policy DROP, accepting only on bridge0
and established connections. The script inserts ACCEPT rules for wlan1 at the
top of INPUT and FORWARD chains.

### Outbound Firewall

The script blocks all outbound traffic on wlan1 except:
- **ESTABLISHED/RELATED**: Replies to incoming connections (so the
  Rayhunter web UI works from your LAN)
- **DHCP** (UDP 67-68): wlan1 lease renewal
- **DNS** (UDP/TCP 53): Hostname resolution

This prevents stock Orbic daemons from phoning home:
- `dmclient` - OMA-DM device management, polls Verizon every ~30s
  sending the device's IMSI
- `upgrade` - FOTA firmware update checks
- `sntp` - NTP time sync
- Any other stock service with outbound network access

**ntfy notifications**: If you configure `ntfy_url` in Rayhunter's
`config.toml`, you must uncomment the HTTPS rule in wifi-client.sh
(or add it manually) to allow outbound port 443:
```sh
iptables -A OUTPUT -o wlan1 -p tcp --dport 443 -j ACCEPT
```
This line must go before the DROP rule. Note that this also allows
`dmclient` to reach Verizon via HTTPS -- there is no way to
distinguish Rayhunter's traffic from other root processes via iptables
alone.

### Auto-Start on Boot

The rayhunter init script (`/etc/init.d/rayhunter_daemon`) launches
wifi-client.sh automatically on boot if `/data/rayhunter/wifi-creds.conf`
exists. It runs in the background so Rayhunter itself starts immediately.

The init script waits up to 30 seconds for wlan1 to appear (the WiFi
driver loads late in the boot sequence), then runs wifi-client.sh.
Total time from power-on to WiFi connected is roughly 60 seconds.

**Disabling auto-start:** Delete or rename `wifi-creds.conf`:
```
adb shell "mv /data/rayhunter/wifi-creds.conf /data/rayhunter/wifi-creds.conf.disabled"
```
Then reboot. The script checks for this file and skips WiFi setup if
it's missing. This is the safety valve if WiFi setup ever causes issues.

**Re-enabling:** Rename it back:
```
adb shell "mv /data/rayhunter/wifi-creds.conf.disabled /data/rayhunter/wifi-creds.conf"
```

**Manual trigger (alternative):** If auto-start is disabled or you need
to re-run the script without rebooting, use AT+SYSCMD (one-shot per boot):
```
adb shell "rootshell -c 'printf \"\r\nAT+SYSCMD=sh /data/rayhunter/scripts/run-wifi.sh\r\n\" > /dev/smd8'"
```

### Runtime-Only Changes

All network changes (routes, iptables rules, wpa_supplicant) are
runtime-only. A reboot restores the default network state, then
auto-start re-applies them. This means a power cycle always gives you
a clean slate if anything goes wrong.

### SIM + WiFi Coexistence

If the device has an active SIM, there will be additional routing complexity.
The cellular connection adds its own default route and DNS. This has not been
tested yet.

## Troubleshooting

### Script doesn't run after AT+SYSCMD

- AT+SYSCMD is one-shot per boot. If anything else wrote to `/dev/smd8`
  first, the command is silently ignored. Power cycle and try again.
- Make sure you use `rootshell -c '...'` not `rootshell sh -c '...'`.
- Make sure the AT command has `\r\n` framing (use `printf`, not `echo`).

### wpa_supplicant connects but no IP

- Check that udhcpc uses `-s /etc/udhcpc.d/50default` (the default script
  path varies between systems).
- Check `/tmp/wifi-client.log` for DHCP discover/select messages.

### Outbound internet works but can't ping from LAN

This was the main debugging challenge. Symptoms: packets arrive at wlan1
(confirmed via `/proc/net/dev` RX counters) but iptables INPUT counter
stays at 0.

**Root cause**: Reply routing. With bridge0 and wlan1 both on 192.168.1.0/24,
the kernel routes replies through bridge0 (its route was added first).
Replies go out the AP interface, which isn't on your home network.

**Fix**: Policy routing (table 100) forces all traffic sourced from wlan1's
IP to use wlan1. This is already in the script.

### adb reboot doesn't work

`adb reboot` doesn't actually reboot the Orbic. `rootshell -c 'reboot'`
powers it off but does NOT power it back on -- you must press the power
button manually:
```
adb shell "rootshell -c 'reboot'"
# Then unplug USB, press power button, wait for boot, reconnect USB
```

**Note:** The Orbic may fail to boot with the USB cable connected. If it
won't power on, unplug the cable first, then power on, then reconnect.

Alternatively, use AT+SYSCMD (but this consumes the one-shot):
```
AT+SYSCMD=shutdown -r -t 1 now
```

### Checking packet flow

Use the diagnostic scripts in `client-mode/diagnostics/`:
- `mac-tcpdump.sh` - capture traffic on your Mac (needs sudo)
- `packet-counters.sh` - read interface RX/TX counters (no root needed)
- `net-diag.sh` - full network dump (needs AT+SYSCMD)

### Reading the log

```
adb shell cat /tmp/wifi-client.log
```

The log shows each step: interface setup, wpa_supplicant status, DHCP lease,
routing configuration, iptables rules, and a connectivity test.
