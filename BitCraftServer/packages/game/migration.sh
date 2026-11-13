#!/bin/bash

read -r -p "Enter the spacetime server name (e.g. bitcraft-staging): " host

while true; do
  read -r -p "Enter the world to migrate (ea2 or release): " world
  case "$world" in
    ea2) prefix="bitcraft-live"; break ;;
    release) prefix="idc"; break ;;
    *) echo "Invalid world: '$world'" >&2 ;;
  esac
done

read -r -p "You are about to migrate world '$world' (prefix: $prefix-) on server '$host'. Continue? (y/n): " confirm
if [[ "$confirm" != "y" ]]; then
  echo "Migration cancelled."
  exit 1
fi

for i in {1..25}; do
  spacetime call -s "$host" "$prefix-$i" #add reducer here
done
