#!/bin/sh
# Relocate an existing jazyk out dir into the target/ layout.
#
# Older builds wrote the source-mirrored stage files (docs/<file>.md/*.yaml) and the link cache
# (link/*.yaml) at the out-dir root. The current layout nests that whole mirror under target/, leaving
# only the finals (linked.yaml, reviewed.yaml, diagnostics.yaml) at the root. This moves the mirror in
# place so a rebuild stays cached instead of recompiling. The move is lossless (no file is rewritten).
#
# Usage: migrate-out-to-target.sh <out-dir> [<out-dir> ...]
# Default out dir is <project>/jazyk-out.
set -eu

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <out-dir> [<out-dir> ...]" >&2
  exit 2
fi

for OUT in "$@"; do
  if [ ! -d "$OUT" ]; then
    echo "skip (not a dir): $OUT" >&2
    continue
  fi
  mkdir -p "$OUT/target"
  moved=0
  for entry in "$OUT"/*; do
    [ -e "$entry" ] || continue
    base=$(basename "$entry")
    case "$base" in
      # Keep the finals at the root; leave target/ and the abandoned legacy cache/ alone.
      target | cache | linked.yaml | reviewed.yaml | diagnostics.yaml) continue ;;
    esac
    mv "$entry" "$OUT/target/"
    moved=$((moved + 1))
  done
  echo "migrated $moved entries into $OUT/target/"
done
