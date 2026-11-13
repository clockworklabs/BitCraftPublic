#!/bin/bash

set -euo pipefail

pause_if_needed() {
  if [[ -z "${NO_PAUSE:-}" && -t 0 ]]; then
    read -r -p "Press Enter to close this window..."
  fi
}
trap pause_if_needed EXIT

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

host="${1:-}"

if [[ -z "$host" ]]; then
  read -r -p "Enter the spacetime server name (e.g. bitcraft-staging): " host
fi

while true; do
  read -r -p "Enter the world to publish (ea2 or release): " world
  case "$world" in
    ea2) prefix="bitcraft-live"; break ;;
    release) prefix="idc"; break ;;
    *) echo "Invalid world: '$world'" >&2 ;;
  esac
done

read -r -p "You are about to publish to world '$world' (prefix: $prefix-) on server '$host'. Continue? (y/n): " confirm
if [[ "$confirm" != "y" ]]; then
  echo "Publish cancelled."
  exit 1
fi

echo "Building module to produce wasm artifact..."
spacetime build -p "$script_dir"

wasm_dir="$script_dir/target/wasm32-unknown-unknown/release"
wasm_path=""

if [[ -f "$wasm_dir/bitcraft_spacetimedb.opt.wasm" ]]; then
  wasm_path="$wasm_dir/bitcraft_spacetimedb.opt.wasm"
elif [[ -f "$wasm_dir/bitcraft_spacetimedb.wasm" ]]; then
  wasm_path="$wasm_dir/bitcraft_spacetimedb.wasm"
fi

if [[ -z "$wasm_path" || ! -f "$wasm_path" ]]; then
  echo "Error: unable to find built wasm under $wasm_dir" >&2
  exit 1
fi

echo "Publishing $prefix-1 to '$host' using: $wasm_path"
spacetime publish -s "$host" --bin-path "$wasm_path" "$prefix-1"
