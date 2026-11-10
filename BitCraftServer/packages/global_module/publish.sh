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

cd "$script_dir"

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

read -r -p "You are about to publish to world '$world' (prefix: $prefix-) on server '$host'. Continue? (yes/no): " confirm
if [[ "$confirm" != "yes" ]]; then
  echo "Publish cancelled."
  exit 1
fi

echo "Publishing $prefix-global to '$host'..."
spacetime publish -s "$host" "$prefix-global"
