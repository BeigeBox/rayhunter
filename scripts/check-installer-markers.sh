#!/usr/bin/env bash
# Verify every marker string referenced by the installer GUI parser
# (devices.ts) and error classifier (error_classifier.ts) still appears
# somewhere in the installer Rust sources. Drift here means the GUI
# silently fails to advance steps or misclassifies errors.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$SCRIPT_DIR")"

DEVICES_FILE="$ROOT/installer-gui/src/lib/devices.ts"
CLASSIFIER_FILE="$ROOT/installer-gui/src/lib/error_classifier.ts"
INSTALLER_SRC="$ROOT/installer/src"

if [ ! -d "$INSTALLER_SRC" ]; then
    echo "installer source dir not found: $INSTALLER_SRC" >&2
    exit 2
fi

extract_quoted() {
    local file="$1" key="$2"
    grep -E "${key}:[[:space:]]*'" "$file" \
        | sed -E "s/.*${key}:[[:space:]]*'([^']*)'.*/\1/" \
        | grep -v '^null$' \
        | grep -v '^$' || true
}

# Strings that the installer Rust side returns but does not literally
# spell out in source (e.g. constructed via anyhow context chains, or
# matched against future Rust changes that have not landed yet).
is_exempt() {
    case "$1" in
        # Cancellation marker comes from a Display impl, not a literal.
        'Installation cancelled.') return 0 ;;
    esac
    return 1
}

missing=()

while IFS= read -r marker; do
    [ -z "$marker" ] && continue
    if is_exempt "$marker"; then continue; fi
    if ! grep -RF --include='*.rs' -q -- "$marker" "$INSTALLER_SRC"; then
        missing+=("devices.ts marker: $marker")
    fi
done < <(extract_quoted "$DEVICES_FILE" "marker")

while IFS= read -r m; do
    [ -z "$m" ] && continue
    if is_exempt "$m"; then continue; fi
    if ! grep -RF --include='*.rs' -q -- "$m" "$INSTALLER_SRC"; then
        missing+=("error_classifier.ts match: $m")
    fi
done < <(extract_quoted "$CLASSIFIER_FILE" "match")

if [ ${#missing[@]} -ne 0 ]; then
    echo "Markers referenced by the installer GUI that no longer appear in installer/src:" >&2
    for m in "${missing[@]}"; do
        echo "  $m" >&2
    done
    echo >&2
    echo "Either restore the original installer message, or update the GUI marker." >&2
    exit 1
fi

echo "All installer markers found."
