# usb-ctl

A wrapper around [uhubctl](https://github.com/mvp/uhubctl) for controlling USB port power on macOS. Designed for automation scripts and programmatic use.

## Prerequisites

Install uhubctl via Homebrew:

```sh
brew install uhubctl
```

### Sudoers configuration

uhubctl requires root privileges. To allow passwordless execution, add a sudoers entry:

```sh
sudo visudo -f /etc/sudoers.d/uhubctl
```

Add the following line (replace `yourusername` with your macOS username):

```
yourusername ALL=(ALL) NOPASSWD: /opt/homebrew/bin/uhubctl
```

This grants sudo access **only** for uhubctl, without requiring a password.

## Installation

### Option A: Run directly

```sh
./usb_ctl.sh help
```

### Option B: Install via Makefile

```sh
make install
```

This creates a symlink at `/usr/local/bin/usb-ctl` pointing to the script.

To remove:

```sh
make uninstall
```

## Usage

```
usb-ctl list                                  Show all compatible hubs and ports
usb-ctl status [--hub <loc>] [--json]         Show port states (optionally as JSON)
usb-ctl on  <port> [--hub <loc>]              Power on a port
usb-ctl off <port> [--hub <loc>]              Power off a port
usb-ctl on  --all  [--hub <loc>]              Power on all ports
usb-ctl off --all  [--hub <loc>]              Power off all ports
usb-ctl cycle <port> [--hub <loc>] [--delay <s>]  Power cycle (default 2s delay)
usb-ctl version                               Print version
usb-ctl help                                  Show this help
```

### Options

| Flag | Description |
|------|-------------|
| `--hub <location>` | Target a specific hub (required when multiple hubs are present) |
| `--all` | Apply to all ports on the hub |
| `--delay <seconds>` | Delay between off and on during cycle (default: 2) |
| `--json` | Output machine-readable JSON (status command only) |

### Hub auto-selection

When only one compatible hub is connected, it is selected automatically. When multiple hubs are present and `--hub` is not specified, the command exits with an error listing available hubs.

## Examples

List all hubs and ports:

```sh
usb-ctl list
```

Get port status as JSON:

```sh
usb-ctl status --json
```

Power off port 2:

```sh
usb-ctl off 2
```

Power cycle port 1 on a specific hub with a 5-second delay:

```sh
usb-ctl cycle 1 --hub 1-1 --delay 5
```

Disable all ports:

```sh
usb-ctl off --all
```

## JSON output format

The `status --json` command outputs:

```json
[
    {
      "hub": "1-1",
      "description": "0bda:5411 Generic 4-Port USB 3.0 Hub, USB 3.00, 4 ports, ppps",
      "ports": [
        {"port": 1, "state": "on", "device": "0bda:8153 Realtek USB GbE Family Controller"},
        {"port": 2, "state": "on", "device": null},
        {"port": 3, "state": "off", "device": null}
      ]
    }
]
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Usage error |
| 2 | uhubctl not found |
| 3 | Permission denied (no sudo) |

## Environment variables

| Variable | Description |
|----------|-------------|
| `UHUBCTL_PATH` | Override the uhubctl binary path (default: `/opt/homebrew/bin/uhubctl`) |
