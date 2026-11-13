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
bin_path="${2:-}"
wasm_dir="$script_dir/target/wasm32-unknown-unknown/release"

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

if [[ -z "$bin_path" ]]; then
  if [[ -f "$wasm_dir/bitcraft_spacetimedb.opt.wasm" ]]; then
    bin_path="$wasm_dir/bitcraft_spacetimedb.opt.wasm"
  elif [[ -f "$wasm_dir/bitcraft_spacetimedb.wasm" ]]; then
    bin_path="$wasm_dir/bitcraft_spacetimedb.wasm"
  else
    echo "No built wasm found under $wasm_dir. Run ./publish-region1.sh first, or pass --bin-path as arg 2." >&2
    read -r -p "Enter path to the built wasm assembly (--bin-path): " bin_path
  fi
fi

if [[ ! -f "$bin_path" ]]; then
  echo "Error: --bin-path file not found: $bin_path" >&2
  exit 1
fi

declare -a pids=()
declare -a modules=()

echo "Publishing $prefix-2..25 to '$host' in parallel using: $bin_path"

for i in {2..25}; do
  module="$prefix-$i"
  (
    spacetime publish -s "$host" --bin-path "$bin_path" "$module" -y
  ) &
  pids+=("$!")
  modules+=("$module")
done

fail_count=0
for idx in "${!pids[@]}"; do
  pid="${pids[$idx]}"
  module="${modules[$idx]}"
  if ! wait "$pid"; then
    echo "Publish failed for $module" >&2
    fail_count=$((fail_count + 1))
  fi
done

if (( fail_count > 0 )); then
  echo "$fail_count publish(es) failed." >&2
  exit 1
fi

echo "All publishes succeeded."
