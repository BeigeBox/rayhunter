#!/usr/bin/env bash
#
# usb-ctl — a wrapper around uhubctl for controlling USB port power on macOS
#
# Exit codes:
#   0 - success
#   1 - usage error
#   2 - uhubctl not found
#   3 - permission denied (no sudo)

set -euo pipefail

readonly VERSION="1.0.0"
readonly UHUBCTL="${UHUBCTL_PATH:-/opt/homebrew/bin/uhubctl}"
readonly DEFAULT_DELAY=2

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

die() {
    echo "error: $1" >&2
    exit "${2:-1}"
}

check_uhubctl() {
    if [[ ! -x "$UHUBCTL" ]]; then
        die "uhubctl not found at $UHUBCTL — install with: brew install uhubctl" 2
    fi
}

check_sudo() {
    if ! sudo -n true 2>/dev/null; then
        die "sudo required — run with sudo or configure NOPASSWD for uhubctl" 3
    fi
}

run_uhubctl() {
    sudo "$UHUBCTL" "$@" 2>&1
}

# ---------------------------------------------------------------------------
# Parsing uhubctl output
#
# uhubctl output looks like:
#
#   Current status for hub 1-1 [0bda:5411 Generic 4-Port USB 3.0 Hub, USB 3.00, 4 ports, ppps]
#     Port 1: 0503 power highspeed enable connect [0bda:8153 Realtek USB GbE Family Controller]
#     Port 2: 0100 power
#     Port 3: 0100 power
#     Port 4: 0100 power
#   Current status for hub 1-1.1 [...]
#     ...
# ---------------------------------------------------------------------------

# Parse uhubctl status output into a structured format.
# Outputs lines of: HUB_LOCATION<TAB>HUB_DESCRIPTION<TAB>PORT<TAB>STATE<TAB>DEVICE
parse_uhubctl_output() {
    local hub_loc="" hub_desc=""
    while IFS= read -r line; do
        if [[ "$line" =~ ^Current\ status\ for\ hub\ ([^ ]+)\ \[(.+)\]$ ]]; then
            hub_loc="${BASH_REMATCH[1]}"
            hub_desc="${BASH_REMATCH[2]}"
        elif [[ "$line" =~ ^[[:space:]]+Port\ ([0-9]+):\ ([0-9a-f]+)\ (.+)$ ]]; then
            local port="${BASH_REMATCH[1]}"
            local raw="${BASH_REMATCH[3]}"
            local state="off"
            local device=""
            if [[ "$raw" == *"power"* ]]; then
                state="on"
            fi
            if [[ "$raw" =~ \[(.+)\]$ ]]; then
                device="${BASH_REMATCH[1]}"
            fi
            printf '%s\t%s\t%s\t%s\t%s\n' "$hub_loc" "$hub_desc" "$port" "$state" "$device"
        fi
    done
}

# Get list of unique hub locations from parsed output.
get_hub_locations() {
    cut -f1 | sort -u
}

# Resolve which hub to target. If --hub given, use it. If only one hub exists,
# auto-select. If multiple exist and no --hub, error with list.
resolve_hub() {
    local requested_hub="$1"
    local parsed="$2"

    if [[ -n "$requested_hub" ]]; then
        # Verify the requested hub exists
        if ! echo "$parsed" | cut -f1 | sort -u | grep -qx "$requested_hub"; then
            local available
            available=$(echo "$parsed" | cut -f1 | sort -u | tr '\n' ', ' | sed 's/,$//')
            die "hub '$requested_hub' not found. Available hubs: $available" 1
        fi
        echo "$requested_hub"
        return
    fi

    local hubs
    hubs=$(echo "$parsed" | get_hub_locations)
    local count
    count=$(echo "$hubs" | wc -l | tr -d ' ')

    if [[ "$count" -eq 0 ]]; then
        die "no compatible USB hubs found" 1
    elif [[ "$count" -eq 1 ]]; then
        echo "$hubs"
    else
        echo "error: multiple hubs detected — specify one with --hub <location>:" >&2
        echo "$hubs" | while IFS= read -r h; do
            local desc
            desc=$(echo "$parsed" | awk -F'\t' -v hub="$h" '$1 == hub { print $2; exit }')
            echo "  $h  [$desc]" >&2
        done
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Output formatters
# ---------------------------------------------------------------------------

# Print parsed data as a human-readable table.
format_table() {
    local parsed="$1"
    local current_hub=""
    while IFS=$'\t' read -r hub_loc hub_desc port state device; do
        if [[ "$hub_loc" != "$current_hub" ]]; then
            [[ -n "$current_hub" ]] && echo
            echo "Hub $hub_loc [$hub_desc]"
            current_hub="$hub_loc"
        fi
        local dev_info=""
        [[ -n "$device" ]] && dev_info="  ($device)"
        printf "  Port %s: %-3s%s\n" "$port" "$state" "$dev_info"
    done <<< "$parsed"
}

# Print parsed data as JSON.
format_json() {
    local parsed="$1"
    local first_hub=true
    local current_hub=""
    local first_port=true

    echo "["
    while IFS=$'\t' read -r hub_loc hub_desc port state device; do
        if [[ "$hub_loc" != "$current_hub" ]]; then
            if [[ -n "$current_hub" ]]; then
                # close previous hub's ports array and object
                echo ""
                echo "      ]"
                echo "    },"
            fi
            current_hub="$hub_loc"
            first_port=true
            echo "    {"
            echo "      \"hub\": \"$hub_loc\","
            echo "      \"description\": \"$hub_desc\","
            echo "      \"ports\": ["
        fi
        if [[ "$first_port" == true ]]; then
            first_port=false
        else
            echo ","
        fi
        local dev_json="null"
        [[ -n "$device" ]] && dev_json="\"$device\""
        printf '        {"port": %s, "state": "%s", "device": %s}' "$port" "$state" "$dev_json"
    done <<< "$parsed"
    # close last hub
    if [[ -n "$current_hub" ]]; then
        echo ""
        echo "      ]"
        echo "    }"
    fi
    echo "]"
}

# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

cmd_list() {
    local raw
    raw=$(run_uhubctl)
    local parsed
    parsed=$(echo "$raw" | parse_uhubctl_output)

    if [[ -z "$parsed" ]]; then
        echo "No compatible USB hubs found."
        return
    fi

    format_table "$parsed"
}

cmd_status() {
    local hub="" json=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --hub)  hub="$2"; shift 2 ;;
            --json) json=true; shift ;;
            *)      die "unknown option: $1" 1 ;;
        esac
    done

    local raw
    raw=$(run_uhubctl)
    local parsed
    parsed=$(echo "$raw" | parse_uhubctl_output)

    if [[ -z "$parsed" ]]; then
        if [[ "$json" == true ]]; then
            echo "[]"
        else
            echo "No compatible USB hubs found."
        fi
        return
    fi

    if [[ -n "$hub" ]]; then
        parsed=$(echo "$parsed" | awk -F'\t' -v h="$hub" '$1 == h')
        if [[ -z "$parsed" ]]; then
            die "hub '$hub' not found" 1
        fi
    fi

    if [[ "$json" == true ]]; then
        format_json "$parsed"
    else
        format_table "$parsed"
    fi
}

cmd_on() {
    local port="" hub="" all=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --hub)  hub="$2"; shift 2 ;;
            --all)  all=true; shift ;;
            *)
                if [[ -z "$port" && "$1" =~ ^[0-9]+$ ]]; then
                    port="$1"; shift
                else
                    die "unknown option: $1" 1
                fi
                ;;
        esac
    done

    if [[ "$all" == false && -z "$port" ]]; then
        die "usage: usb-ctl on <port> [--hub <location>]  or  usb-ctl on --all [--hub <location>]" 1
    fi

    local raw parsed target_hub
    raw=$(run_uhubctl)
    parsed=$(echo "$raw" | parse_uhubctl_output)
    target_hub=$(resolve_hub "$hub" "$parsed")

    local args=(-l "$target_hub" -a on)
    if [[ "$all" == false ]]; then
        args+=(-p "$port")
    fi

    run_uhubctl "${args[@]}" >/dev/null
    echo "ok"
}

cmd_off() {
    local port="" hub="" all=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --hub)  hub="$2"; shift 2 ;;
            --all)  all=true; shift ;;
            *)
                if [[ -z "$port" && "$1" =~ ^[0-9]+$ ]]; then
                    port="$1"; shift
                else
                    die "unknown option: $1" 1
                fi
                ;;
        esac
    done

    if [[ "$all" == false && -z "$port" ]]; then
        die "usage: usb-ctl off <port> [--hub <location>]  or  usb-ctl off --all [--hub <location>]" 1
    fi

    local raw parsed target_hub
    raw=$(run_uhubctl)
    parsed=$(echo "$raw" | parse_uhubctl_output)
    target_hub=$(resolve_hub "$hub" "$parsed")

    local args=(-l "$target_hub" -a off)
    if [[ "$all" == false ]]; then
        args+=(-p "$port")
    fi

    run_uhubctl "${args[@]}" >/dev/null
    echo "ok"
}

cmd_cycle() {
    local port="" hub="" delay="$DEFAULT_DELAY"
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --hub)   hub="$2"; shift 2 ;;
            --delay) delay="$2"; shift 2 ;;
            *)
                if [[ -z "$port" && "$1" =~ ^[0-9]+$ ]]; then
                    port="$1"; shift
                else
                    die "unknown option: $1" 1
                fi
                ;;
        esac
    done

    if [[ -z "$port" ]]; then
        die "usage: usb-ctl cycle <port> [--hub <location>] [--delay <seconds>]" 1
    fi

    if ! [[ "$delay" =~ ^[0-9]+\.?[0-9]*$ ]]; then
        die "invalid delay: $delay" 1
    fi

    local raw parsed target_hub
    raw=$(run_uhubctl)
    parsed=$(echo "$raw" | parse_uhubctl_output)
    target_hub=$(resolve_hub "$hub" "$parsed")

    run_uhubctl -l "$target_hub" -a off -p "$port" >/dev/null
    sleep "$delay"
    run_uhubctl -l "$target_hub" -a on -p "$port" >/dev/null
    echo "ok"
}

cmd_help() {
    cat <<'EOF'
usb-ctl — control USB port power via uhubctl

Usage:
  usb-ctl list                                  Show all compatible hubs and ports
  usb-ctl status [--hub <loc>] [--json]         Show port states (optionally as JSON)
  usb-ctl on  <port> [--hub <loc>]              Power on a port
  usb-ctl off <port> [--hub <loc>]              Power off a port
  usb-ctl on  --all  [--hub <loc>]              Power on all ports
  usb-ctl off --all  [--hub <loc>]              Power off all ports
  usb-ctl cycle <port> [--hub <loc>] [--delay <s>]  Power cycle (default 2s delay)
  usb-ctl version                               Print version
  usb-ctl help                                  Show this help

Options:
  --hub <location>   Target a specific hub (required when multiple hubs present)
  --all              Apply to all ports on the hub
  --delay <seconds>  Delay between off and on during cycle (default: 2)
  --json             Output machine-readable JSON (status command)

Exit codes:
  0  success
  1  usage error
  2  uhubctl not found
  3  permission denied

Environment:
  UHUBCTL_PATH       Override uhubctl binary path (default: /opt/homebrew/bin/uhubctl)
EOF
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    if [[ $# -eq 0 ]]; then
        cmd_help
        exit 1
    fi

    local cmd="$1"
    shift

    case "$cmd" in
        help|--help|-h)
            cmd_help
            exit 0
            ;;
        version|--version|-V)
            echo "usb-ctl $VERSION"
            exit 0
            ;;
    esac

    check_uhubctl
    check_sudo

    case "$cmd" in
        list)   cmd_list "$@" ;;
        status) cmd_status "$@" ;;
        on)     cmd_on "$@" ;;
        off)    cmd_off "$@" ;;
        cycle)  cmd_cycle "$@" ;;
        *)      die "unknown command: $cmd — run 'usb-ctl help' for usage" 1 ;;
    esac
}

main "$@"
