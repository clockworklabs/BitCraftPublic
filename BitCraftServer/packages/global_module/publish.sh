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

echo "Publishing bitcraft-live-global to '$host'..."
spacetime publish -s "$host" bitcraft-live-global
