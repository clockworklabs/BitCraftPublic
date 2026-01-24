#!/bin/bash

read -p "Enter the spacetime server name (e.g. bitcraft-staging): " host

for i in {1..9}; do
  spacetime call -s "$host" "bitcraft-$i" migrate_onboarding
  #spacetime call -s "$host" "bitcraft-$i" admin_set_resource_world_target 1073998942 0
  #spacetime call -s "$host" "bitcraft-$i" admin_create_building_spawns 1842388176 false
done
