#!/usr/bin/env bash
# Regenerate homepage/public/fresh-demo.cast.
#
# Prerequisites:
#   - A built fresh binary (target/debug/fresh or target/release/fresh),
#     or $FRESH pointing at one, or 'fresh' on $PATH.
#   - Python 3 (stdlib only).
#   - git, bash.
#
# Usage: scripts/record-asciinema/regenerate.sh
#
# The resulting .cast is written to homepage/public/fresh-demo.cast, which
# is what the hero asciinema-player on index.html loads.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT_DIR="$REPO_ROOT/scripts/record-asciinema"
DEMO_DIR="${DEMO_DIR:-/tmp/fresh-demo-workspace}"
OUTPUT="${OUTPUT:-$REPO_ROOT/homepage/public/fresh-demo.cast}"

echo "[1/3] Preparing demo workspace at $DEMO_DIR"
bash "$SCRIPT_DIR/setup-demo.sh" "$DEMO_DIR"

# If fresh isn't already built, try to build it.
if [ -z "${FRESH:-}" ] \
   && [ ! -x "$REPO_ROOT/target/release/fresh" ] \
   && [ ! -x "$REPO_ROOT/target/debug/fresh" ] \
   && ! command -v fresh >/dev/null 2>&1; then
    echo "[2/3] Building fresh (debug)"
    (cd "$REPO_ROOT" && cargo build --bin fresh)
else
    echo "[2/3] Using existing fresh binary"
fi

echo "[3/3] Recording demo"
python3 "$SCRIPT_DIR/record.py" "$OUTPUT" --demo "$DEMO_DIR"

echo "done — $OUTPUT"
