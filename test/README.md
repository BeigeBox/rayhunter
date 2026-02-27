# Rayhunter Acceptance Tests

Acceptance test suite that exercises a real running Rayhunter daemon via its HTTP API. Talks to a live device not mocks.

## Quick Start

```bash
cargo build -p rayhunter-test

# List all tests (no device needed)
cargo run -p rayhunter-test -- --list

# Run all tests against a device
cargo run -p rayhunter-test -- --host <ADDRESS:PORT>

# Run one test group
cargo run -p rayhunter-test -- --host <ADDRESS:PORT> config

# Run a single test
cargo run -p rayhunter-test -- --host <ADDRESS:PORT> --exact config::get_returns_valid_json

# Include shell-level tests via ADB
cargo run -p rayhunter-test -- --host <ADDRESS:PORT> --shell adb
```

`--host` is required. The address depends on your device and connection method.

## Test Groups

| Group | Tests | What it covers |
|-------|-------|---------------|
| `config` | 5 | GET shape, SET+restore, restart detection, invalid JSON rejection, SSID stripping |
| `system` | 5 | system-stats fields, time, invalid time-offset JSON, log endpoint |
| `recording` | 10 | start/stop, manifest, delete, delete-all, double-start, debug_mode guard, stop idempotency, low disk 507, nonexistent delete, delete while recording |
| `download` | 4 | QMDL + PCAP + ZIP validation (single recording), 404 for nonexistent downloads |
| `analysis` | 6 | queue status, polling, report retrieval, live report 503, nonexistent report 404, nonexistent name queuing |
| `wifi` | 6 | status, scan, rate limit, disable/enable, wrong SSID, missing password |
| `shell` | 5 | process running, config on disk, DNS, wpa creds, log file |
| `security` | 4 | password redaction (GET, POST, log, config.toml) |

Tests that require capabilities not available on the device (e.g. WiFi disabled, no shell access) are automatically marked `ignored`.

This crate is in the workspace `members` but not `default-members`, so `cargo build` / `cargo test` won't pick it up. Run explicitly with `-p rayhunter-test`.
