#!/bin/bash
HOOK_TYPE="${1:-unknown}"
TIMESTAMP=$(date +%s)
OUTPUT_DIR="/tmp/plate-spinner-discovery"
mkdir -p "$OUTPUT_DIR"

# Dump environment variables
env | sort > "$OUTPUT_DIR/env-${HOOK_TYPE}-${TIMESTAMP}.txt"

# Capture stdin (hooks may receive data via stdin)
cat > "$OUTPUT_DIR/stdin-${HOOK_TYPE}-${TIMESTAMP}.txt"

echo "Dumped to $OUTPUT_DIR/*-${HOOK_TYPE}-${TIMESTAMP}.txt"
