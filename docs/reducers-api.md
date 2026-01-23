# BitCraft Reducers API Reference

## Table of Contents

- [Overview](#overview)
- [Core Lifecycle](#core-lifecycle)
- [Player Actions](#player-actions)
- [Movement and Teleportation](#movement-and-teleportation)
- [Inventory Management](#inventory-management)
- [Crafting](#crafting)
- [Building and Construction](#building-and-construction)
- [Claim Management](#claim-management)
- [Empire System](#empire-system)
- [Combat](#combat)
- [Trading](#trading)
- [Social and Communication](#social-and-communication)
- [Admin Commands](#admin-commands)
- [Cheat Commands](#cheat-commands)

## Overview

**Reducers** are the API endpoints of BitCraft. They are the only way to modify game state in SpacetimeDB.

### Key Characteristics

- **675+ reducers** across the codebase
- **Transactional**: Each reducer runs in an atomic database transaction
- **Type-safe**: Strongly typed parameters and return values
- **Authenticated**: Access to caller's `Identity` via `ReducerContext`
- **Error handling**: Return `Result<(), String>` for proper error messages

### Calling Reducers

**From Client** (TypeScript/JavaScript example):
```typescript
// Sign in
await BitCraft.player_move(100.5, 200.3, true);

// Craft item
await BitCraft.craft_initiate(recipeId, buildingId, quantity);
```

**From Another Reducer** (Rust):
```rust
#[spacetimedb::reducer]
pub fn my_reducer(ctx: &ReducerContext) {
    // Reducers can call other reducers
    player_move(ctx, 100.0, 200.0, false)?;
}
```

### Reducer Context

Every reducer receives a `ReducerContext`:

```rust
pub struct ReducerContext {
    pub db: Database,        // Table access
    pub sender: Identity,    // Calling client
    pub timestamp: Timestamp, // Current server time
    pub address: Address,    // Caller address (for auth)
}
```

## Core Lifecycle

### initialize

**File**: `lib.rs`

Initializes the database with default values.

```rust
#[spacetimedb::reducer(init)]
pub fn initialize(ctx: &ReducerContext)
```

**Called**: Automatically when module is first published

**Actions**:
- Creates `Globals` entry
- Sets up default parameters
- Initializes system state

### identity_connected

**File**: `lib.rs`

Handles client connection events.

```rust
#[spacetimedb::reducer]
pub fn identity_connected(ctx: &ReducerContext)
```

**Called**: Automatically when client connects

**Actions**:
- Logs connection
- Updates user state
- Prepares session

### identity_disconnected

**File**: `lib.rs`

Handles client disconnection events.

```rust
#[spacetimedb::reducer]
pub fn identity_disconnected(ctx: &ReducerContext)
```

**Called**: Automatically when client disconnects

**Actions**:
- Starts grace period for reconnection
- Schedules logout if not reconnected

## Player Actions

### sign_in

**File**: `handlers/player/sign_in.rs`

Logs a player into the game world.

```rust
#[spacetimedb::reducer]
pub fn sign_in(ctx: &ReducerContext) -> Result<(), String>
```

**Parameters**: None (uses caller's `Identity`)

**Returns**: `Ok(())` on success, error message on failure

**Validation**:
- User exists and has entity
- `can_sign_in` flag is true (queue)
- Not already signed in
- Player entity exists

**Effects**:
- Sets `signed_in = true`
- Updates session timestamps
- Adds to `SignedInPlayerState`
- Schedules auto-logout agent

**Errors**:
- "User not found"
- "Cannot sign in (queue)"
- "Already signed in"

### sign_out

**File**: `handlers/player/sign_out.rs`

Logs a player out of the game world.

```rust
#[spacetimedb::reducer]
pub fn sign_out(ctx: &ReducerContext) -> Result<(), String>
```

**Parameters**: None

**Effects**:
- Sets `signed_in = false`
- Updates play time statistics
- Removes from `SignedInPlayerState`
- Cleans up temporary state

**Errors**:
- "Not signed in"

## Movement and Teleportation

### player_move

**File**: `handlers/player/player_move.rs`

Moves player to a new location.

```rust
#[spacetimedb::reducer]
pub fn player_move(
    ctx: &ReducerContext,
    destination_x: f32,
    destination_z: f32,
    is_running: bool,
) -> Result<(), String>
```

**Parameters**:
- `destination_x`: Target X coordinate
- `destination_z`: Target Z coordinate
- `is_running`: Whether player is running (affects stamina)

**Validation**:
- Player is signed in
- Destination is within movement range
- Terrain is traversable
- Sufficient stamina for running
- Not in restricted area without permission

**Effects**:
- Updates `MobileEntityState` with new destination
- Deducts stamina if running
- Updates exploration chunks (fog of war)
- Sets player action to `Move`

**Errors**:
- "Not signed in"
- "Destination too far"
- "Terrain not traversable"
- "Insufficient stamina"
- "Cannot enter claimed area"

### player_teleport_home

**File**: `handlers/player/player_teleport_home.rs`

Teleports player to their home location.

```rust
#[spacetimedb::reducer]
pub fn player_teleport_home(ctx: &ReducerContext) -> Result<(), String>
```

**Requirements**:
- Home location must be set
- Sufficient teleport energy
- Not in combat

**Effects**:
- Instantly moves player to home
- Deducts teleport energy
- Updates timestamp

**Errors**:
- "Home not set"
- "Insufficient teleport energy"
- "Cannot teleport while in combat"

### set_home

**File**: `handlers/player/set_home.rs`

Sets player's home location for teleportation.

```rust
#[spacetimedb::reducer]
pub fn set_home(ctx: &ReducerContext) -> Result<(), String>
```

**Requirements**:
- Must be at a valid home-setting location (bed, claim center)
- Not in combat

**Effects**:
- Updates `home_location` in `PlayerState`
- Sets `home_dimension`

**Errors**:
- "Not at valid location to set home"
- "Cannot set home while in combat"

### player_teleport_waystone

**File**: `handlers/player/player_teleport_waystone.rs`

Teleports player to a waystone.

```rust
#[spacetimedb::reducer]
pub fn player_teleport_waystone(
    ctx: &ReducerContext,
    waystone_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `waystone_entity_id`: Target waystone building

**Requirements**:
- Waystone exists and is active
- Player has discovered waystone
- Sufficient teleport energy
- Not in combat

**Effects**:
- Teleports to waystone location
- Deducts teleport energy

**Errors**:
- "Waystone not found"
- "Waystone not discovered"
- "Insufficient teleport energy"

### portal_enter

**File**: `handlers/player/portal_enter.rs`

Enters a portal to travel to another dimension.

```rust
#[spacetimedb::reducer]
pub fn portal_enter(
    ctx: &ReducerContext,
    portal_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `portal_entity_id`: Portal building to enter

**Validation**:
- Portal exists
- Portal has valid destination
- Player is near portal

**Effects**:
- Changes player's dimension
- Updates location to portal destination
- Triggers dimension change events

**Errors**:
- "Portal not found"
- "Not near portal"
- "Portal destination invalid"

### player_climb

**File**: `handlers/player/player_climb.rs`

Climbs vertical surfaces.

```rust
#[spacetimedb::reducer]
pub fn player_climb(
    ctx: &ReducerContext,
    destination_x: f32,
    destination_z: f32,
) -> Result<(), String>
```

**Parameters**:
- `destination_x`: Target X coordinate
- `destination_z`: Target Z coordinate

**Validation**:
- Adjacent tile has climbable elevation difference
- Sufficient stamina

**Effects**:
- Moves to higher/lower elevation
- Deducts stamina
- Updates action state

## Inventory Management

### item_stack_move

**File**: `handlers/inventory/item_stack_move.rs`

Moves items between inventory pockets.

```rust
#[spacetimedb::reducer]
pub fn item_stack_move(
    ctx: &ReducerContext,
    from_entity_id: u64,
    from_pocket_index: i32,
    from_stack_index: i32,
    to_entity_id: u64,
    to_pocket_index: i32,
    to_stack_index: i32,
    quantity: i32,
) -> Result<(), String>
```

**Parameters**:
- `from_*`: Source inventory, pocket, and stack
- `to_*`: Destination inventory, pocket, and stack
- `quantity`: Number of items to move

**Validation**:
- Source and destination inventories exist
- Player has permission to access both inventories
- Pockets are not locked
- Sufficient quantity in source
- Destination has volume space

**Effects**:
- Removes items from source stack
- Adds items to destination stack (stacks if same item)
- Updates inventory states

**Errors**:
- "Insufficient quantity"
- "No permission"
- "Pocket locked"
- "Insufficient volume"

### item_stack_split

**File**: `handlers/inventory/item_stack_split.rs`

Splits an item stack into two stacks.

```rust
#[spacetimedb::reducer]
pub fn item_stack_split(
    ctx: &ReducerContext,
    entity_id: u64,
    pocket_index: i32,
    stack_index: i32,
    split_quantity: i32,
) -> Result<(), String>
```

**Parameters**:
- `entity_id`: Inventory entity
- `pocket_index`: Pocket containing stack
- `stack_index`: Stack to split
- `split_quantity`: Amount for new stack

**Effects**:
- Reduces original stack by `split_quantity`
- Creates new stack with `split_quantity`

**Errors**:
- "Insufficient quantity"
- "No empty slot for new stack"

### item_drop

**File**: `handlers/inventory/item_drop.rs`

Drops items on the ground.

```rust
#[spacetimedb::reducer]
pub fn item_drop(
    ctx: &ReducerContext,
    entity_id: u64,
    pocket_index: i32,
    stack_index: i32,
    quantity: i32,
) -> Result<(), String>
```

**Parameters**:
- `entity_id`: Player's inventory entity
- `pocket_index`: Source pocket
- `stack_index`: Source stack
- `quantity`: Amount to drop

**Effects**:
- Removes items from inventory
- Creates `DroppedInventoryState` entity at player location
- Sets pickup protection timer
- Schedules despawn

**Errors**:
- "Insufficient quantity"
- "Cannot drop equipped items"

### item_pick_up

**File**: `handlers/inventory/item_pick_up.rs`

Picks up dropped items.

```rust
#[spacetimedb::reducer]
pub fn item_pick_up(
    ctx: &ReducerContext,
    dropped_inventory_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `dropped_inventory_entity_id`: Dropped item entity

**Validation**:
- Dropped inventory exists
- Player is near the items
- Protection timer expired (or player is owner)
- Player has inventory space

**Effects**:
- Adds items to player inventory
- Deletes dropped inventory entity

**Errors**:
- "Not near items"
- "Items protected (not your drop)"
- "Insufficient inventory space"

### inventory_sort

**File**: `handlers/inventory/inventory_sort.rs`

Sorts inventory by item type.

```rust
#[spacetimedb::reducer]
pub fn inventory_sort(
    ctx: &ReducerContext,
    entity_id: u64,
    pocket_index: i32,
) -> Result<(), String>
```

**Parameters**:
- `entity_id`: Inventory entity
- `pocket_index`: Pocket to sort

**Effects**:
- Rearranges items by item ID
- Stacks identical items
- Optimizes space usage

## Crafting

### craft_initiate

**File**: `handlers/player_craft/craft_initiate.rs`

Starts active crafting.

```rust
#[spacetimedb::reducer]
pub fn craft_initiate(
    ctx: &ReducerContext,
    recipe_id: i32,
    building_entity_id: u64,
    items_requested: i32,
) -> Result<(), String>
```

**Parameters**:
- `recipe_id`: Recipe to craft
- `building_entity_id`: Crafting station (0 for no building)
- `items_requested`: Number of items to craft

**Validation**:
- Recipe exists and player knows it
- Has required building (if needed)
- Has required tool equipped (if needed)
- Has required skill level
- Has required materials in inventory
- Building is accessible

**Effects**:
- Consumes materials (if config enabled)
- Creates `ProgressiveActionState`
- Sets player action to `Craft`
- Starts crafting timer

**Errors**:
- "Recipe not found"
- "Recipe not unlocked"
- "Missing required building"
- "Missing required tool"
- "Insufficient skill level"
- "Insufficient materials"

### craft_continue_start

**File**: `handlers/player_craft/craft_continue_start.rs`

Resumes suspended crafting.

```rust
#[spacetimedb::reducer]
pub fn craft_continue_start(
    ctx: &ReducerContext,
    progressive_action_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `progressive_action_entity_id`: Suspended progressive action

**Validation**:
- Progressive action exists and is owned by player
- Status is `Suspended`
- Player is at correct building

**Effects**:
- Changes status to `InProgress`
- Updates timestamps
- Resumes crafting

### craft_collect

**File**: `handlers/player_craft/craft_collect.rs`

Collects finished crafted items.

```rust
#[spacetimedb::reducer]
pub fn craft_collect(
    ctx: &ReducerContext,
    progressive_action_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `progressive_action_entity_id`: Completed progressive action

**Validation**:
- Progressive action is complete
- Player has inventory space

**Effects**:
- Adds crafted items to inventory
- Grants crafting experience
- Deletes progressive action entity

**Errors**:
- "Not complete"
- "Insufficient inventory space"

### craft_cancel

**File**: `handlers/player_craft/craft_cancel.rs`

Cancels active crafting.

```rust
#[spacetimedb::reducer]
pub fn craft_cancel(
    ctx: &ReducerContext,
    progressive_action_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `progressive_action_entity_id`: Progressive action to cancel

**Effects**:
- Deletes progressive action
- Returns partial materials (if applicable)
- Clears player action

### passive_craft_queue

**File**: `handlers/player_craft/passive_craft_queue.rs`

Queues passive crafting.

```rust
#[spacetimedb::reducer]
pub fn passive_craft_queue(
    ctx: &ReducerContext,
    recipe_id: i32,
    building_entity_id: u64,
    items_requested: i32,
) -> Result<(), String>
```

**Parameters**:
- `recipe_id`: Recipe to craft
- `building_entity_id`: Crafting building
- `items_requested`: Quantity to craft

**Validation**:
- Similar to `craft_initiate`
- Building supports passive crafting

**Effects**:
- Consumes materials immediately
- Creates `PassiveCraftState` entry
- Queues in building's craft queue
- Starts background processing

### passive_craft_collect

**File**: `handlers/player_craft/passive_craft_collect.rs`

Collects finished passive crafting.

```rust
#[spacetimedb::reducer]
pub fn passive_craft_collect(
    ctx: &ReducerContext,
    passive_craft_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `passive_craft_id`: Passive craft entry ID

**Validation**:
- Passive craft is complete
- Player has inventory space

**Effects**:
- Adds items to inventory
- Grants experience
- Deletes passive craft entry

### item_convert

**File**: `handlers/player_craft/item_convert.rs`

Converts items using conversion recipes.

```rust
#[spacetimedb::reducer]
pub fn item_convert(
    ctx: &ReducerContext,
    recipe_id: i32,
    building_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `recipe_id`: Conversion recipe
- `building_entity_id`: Conversion building

**Effects**:
- Instant conversion (no crafting time)
- Consumes input items
- Produces output items

**Use Cases**:
- Smelting ores to bars
- Processing raw materials
- Refining resources

## Building and Construction

### project_site_place

**File**: `handlers/buildings/project_site_place.rs`

Places a construction project site.

```rust
#[spacetimedb::reducer]
pub fn project_site_place(
    ctx: &ReducerContext,
    building_description_id: i32,
    location: SmallHexTile,
    direction_index: u8,
) -> Result<(), String>
```

**Parameters**:
- `building_description_id`: Building type to construct
- `location`: Placement location
- `direction_index`: Rotation (0-5)

**Validation**:
- Building type exists
- Player knows the building (has recipe/tech)
- Location is valid terrain for building
- No overlapping buildings/resources
- Claim permission (if on claimed land)
- Footprint tiles are valid

**Effects**:
- Creates `ProjectSiteState` entity
- Creates placeholder visual entity
- Reserves footprint tiles

**Errors**:
- "Building not found"
- "Building not unlocked"
- "Invalid placement location"
- "Overlapping entity"
- "No build permission on claim"

### project_site_add_materials

**File**: `handlers/buildings/project_site_add_materials.rs`

Adds materials to construction project.

```rust
#[spacetimedb::reducer]
pub fn project_site_add_materials(
    ctx: &ReducerContext,
    project_site_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `project_site_entity_id`: Project site to contribute to

**Validation**:
- Project site exists
- Player has required materials
- Project is not complete
- Project is public OR player is owner/member

**Effects**:
- Transfers materials from inventory to project
- Updates `materials_contributed`
- Updates `construction_progress`

**Errors**:
- "No materials to contribute"
- "Project complete"
- "No permission (private project)"

### project_site_advance_project

**File**: `handlers/buildings/project_site_advance_project.rs`

Advances construction progress (timed action).

```rust
#[spacetimedb::reducer]
pub fn project_site_advance_project(
    ctx: &ReducerContext,
    project_site_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `project_site_entity_id`: Project to work on

**Validation**:
- Project has all required materials
- Player is near project
- Not already being worked on

**Effects**:
- Starts timed construction action
- On completion: Converts project site to finished building
- Grants building experience

### project_site_cancel

**File**: `handlers/buildings/project_site_cancel.rs`

Cancels construction and refunds materials.

```rust
#[spacetimedb::reducer]
pub fn project_site_cancel(
    ctx: &ReducerContext,
    project_site_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `project_site_entity_id`: Project to cancel

**Validation**:
- Project exists
- Player is owner

**Effects**:
- Returns contributed materials to owner
- Deletes project site entity

### building_repair

**File**: `handlers/buildings/building_repair.rs`

Repairs damaged buildings.

```rust
#[spacetimedb::reducer]
pub fn building_repair(
    ctx: &ReducerContext,
    building_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `building_entity_id`: Building to repair

**Validation**:
- Building is damaged
- Player has repair materials
- Player has build permission on claim

**Effects**:
- Consumes repair materials
- Restores building health
- Grants experience

### building_deconstruct

**File**: `handlers/buildings/building_deconstruct.rs`

Deconstructs a building.

```rust
#[spacetimedb::reducer]
pub fn building_deconstruct(
    ctx: &ReducerContext,
    building_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `building_entity_id`: Building to remove

**Validation**:
- Player has build permission on claim
- Building is not critical (e.g., claim totem)

**Effects**:
- Returns partial materials
- Deletes building entity
- Frees footprint tiles

### building_set_nickname

**File**: `handlers/buildings/building_set_nickname.rs`

Sets custom building name.

```rust
#[spacetimedb::reducer]
pub fn building_set_nickname(
    ctx: &ReducerContext,
    building_entity_id: u64,
    nickname: String,
) -> Result<(), String>
```

**Parameters**:
- `building_entity_id`: Building to rename
- `nickname`: New name

**Validation**:
- Player has usage permission
- Nickname is valid (length, characters)

**Effects**:
- Updates `BuildingNicknameState`

### building_set_sign_text

**File**: `handlers/buildings/building_set_sign_text.rs`

Sets sign text on building.

```rust
#[spacetimedb::reducer]
pub fn building_set_sign_text(
    ctx: &ReducerContext,
    building_entity_id: u64,
    sign_text: String,
) -> Result<(), String>
```

**Parameters**:
- `building_entity_id`: Building with sign
- `sign_text`: Text to display

**Validation**:
- Building supports signs
- Text is valid

**Effects**:
- Updates sign text in `BuildingNicknameState`

## Claim Management

### claim_take_ownership

**File**: `handlers/claim/claim_take_ownership.rs`

Claims an ownerless building.

```rust
#[spacetimedb::reducer]
pub fn claim_take_ownership(
    ctx: &ReducerContext,
    building_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `building_entity_id`: Ownerless building to claim

**Validation**:
- Building exists and is claimable type
- Building has no current claim
- Player doesn't exceed claim limit

**Effects**:
- Creates `ClaimState` entity
- Sets player as owner
- Adds initial claim tile at building location

**Errors**:
- "Building not claimable"
- "Already claimed"
- "Claim limit reached"

### claim_transfer_ownership

**File**: `handlers/claim/claim_transfer_ownership.rs`

Transfers claim ownership to another player.

```rust
#[spacetimedb::reducer]
pub fn claim_transfer_ownership(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    new_owner_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `claim_entity_id`: Claim to transfer
- `new_owner_entity_id`: New owner player

**Validation**:
- Caller is current owner
- New owner exists

**Effects**:
- Updates claim owner
- Transfers permissions

### claim_add_member

**File**: `handlers/claim/claim_add_member.rs`

Adds a member to the claim.

```rust
#[spacetimedb::reducer]
pub fn claim_add_member(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    player_entity_id: u64,
    permissions: ClaimPermissions,
) -> Result<(), String>
```

**Parameters**:
- `claim_entity_id`: Claim to add member to
- `player_entity_id`: Player to add
- `permissions`: Permissions to grant

**Validation**:
- Caller has recruit permission
- Target player exists
- Not already a member

**Effects**:
- Creates `ClaimMemberState` entry
- Grants specified permissions

### claim_remove_member

**File**: `handlers/claim/claim_remove_member.rs`

Removes a member from the claim.

```rust
#[spacetimedb::reducer]
pub fn claim_remove_member(
    ctx: &ReducerContext,
    claim_member_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `claim_member_id`: Membership ID to remove

**Validation**:
- Caller has remove_member permission
- Cannot remove owner

**Effects**:
- Deletes `ClaimMemberState` entry

### claim_set_member_permissions

**File**: `handlers/claim/claim_set_member_permissions.rs`

Updates member permissions.

```rust
#[spacetimedb::reducer]
pub fn claim_set_member_permissions(
    ctx: &ReducerContext,
    claim_member_id: u64,
    permissions: ClaimPermissions,
) -> Result<(), String>
```

**Parameters**:
- `claim_member_id`: Membership to update
- `permissions`: New permissions

**Validation**:
- Caller has edit_permissions permission

**Effects**:
- Updates permissions in `ClaimMemberState`

### claim_add_tile

**File**: `handlers/claim/claim_add_tile.rs`

Expands claim territory.

```rust
#[spacetimedb::reducer]
pub fn claim_add_tile(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    location: SmallHexTile,
) -> Result<(), String>
```

**Parameters**:
- `claim_entity_id`: Claim to expand
- `location`: Tile to add

**Validation**:
- Caller has add_tile permission
- Tile is adjacent to current territory
- Tile is not already claimed
- Sufficient claim supplies

**Effects**:
- Creates `ClaimTileState` entry
- Consumes supplies

### claim_remove_tile

**File**: `handlers/claim/claim_remove_tile.rs`

Shrinks claim territory.

```rust
#[spacetimedb::reducer]
pub fn claim_remove_tile(
    ctx: &ReducerContext,
    claim_tile_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `claim_tile_id`: Tile to remove

**Validation**:
- Caller has remove_tile permission
- No buildings on tile

**Effects**:
- Deletes `ClaimTileState` entry
- Refunds partial supplies

### claim_resupply

**File**: `handlers/claim/claim_resupply.rs`

Adds supplies to claim.

```rust
#[spacetimedb::reducer]
pub fn claim_resupply(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    supply_item_stacks: Vec<ItemStack>,
) -> Result<(), String>
```

**Parameters**:
- `claim_entity_id`: Claim to resupply
- `supply_item_stacks`: Items to convert to supplies

**Validation**:
- Player has items
- Items are valid supply items

**Effects**:
- Consumes items from inventory
- Adds to claim supplies
- Prevents decay

### claim_tech_learn

**File**: `handlers/claim/claim_tech_learn.rs`

Unlocks claim technology.

```rust
#[spacetimedb::reducer]
pub fn claim_tech_learn(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    tech_id: i32,
) -> Result<(), String>
```

**Parameters**:
- `claim_entity_id`: Claim to research tech
- `tech_id`: Technology to unlock

**Validation**:
- Prerequisites unlocked
- Sufficient resources

**Effects**:
- Unlocks tech in `ClaimTechState`
- Consumes research materials
- Unlocks new buildings/recipes

## Empire System

*These reducers are in the Global Module*

### empire_form

**File**: `global_module/handlers/empires/empire_form.rs`

Creates a new empire.

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_form(
    ctx: &ReducerContext,
    request: EmpireFormRequest,
) -> Result<(), String>
```

**Parameters**:
- `request.name`: Empire name
- `request.initial_settlement_id`: Starting claim
- `request.emblem`: Empire emblem design

**Validation**:
- Player has required tech
- Name is unique and valid
- Has formation cost (shards/items)
- Settlement exists and is eligible

**Effects**:
- Creates `EmpireState` entity
- Sets player as emperor
- Adds settlement to empire
- Replicates to all regions

**Errors**:
- "Insufficient resources"
- "Name already taken"
- "Settlement ineligible"

### empire_player_join

**File**: `global_module/handlers/empires/empire_player_join.rs`

Player joins an empire.

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_player_join(
    ctx: &ReducerContext,
    empire_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `empire_entity_id`: Empire to join

**Validation**:
- Empire exists
- Player has invitation OR empire is open
- Player not already in empire

**Effects**:
- Creates `EmpirePlayerDataState` entry
- Assigns default rank
- Syncs to all regions

### empire_leave

**File**: `global_module/handlers/empires/empire_leave.rs`

Player leaves their empire.

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_leave(ctx: &ReducerContext) -> Result<(), String>
```

**Validation**:
- Player is in empire
- Player is not emperor (must transfer first)

**Effects**:
- Removes `EmpirePlayerDataState` entry
- Updates member count
- Syncs to all regions

### empire_set_player_rank

**File**: `global_module/handlers/empires/empire_set_player_rank.rs`

Changes a member's rank.

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_set_player_rank(
    ctx: &ReducerContext,
    player_entity_id: u64,
    rank_index: i32,
) -> Result<(), String>
```

**Parameters**:
- `player_entity_id`: Member to promote/demote
- `rank_index`: New rank

**Validation**:
- Caller has manage_ranks permission
- Target is empire member
- Rank exists

**Effects**:
- Updates rank in `EmpirePlayerDataState`

### empire_claim_join

**File**: `global_module/handlers/empires/empire_claim_join.rs`

Adds a settlement to empire.

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_claim_join(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    region_index: u32,
) -> Result<(), String>
```

**Parameters**:
- `claim_entity_id`: Claim to add
- `region_index`: Region containing claim

**Validation**:
- Claim meets settlement requirements
- Player is claim owner
- Player is in empire
- Empire has settlement slots available

**Effects**:
- Creates `EmpireSettlementState` entry
- Updates claim's empire_id
- Syncs to regions

### empire_start_siege

**File**: `global_module/handlers/empires/empire_start_siege.rs`

Initiates siege on enemy settlement.

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_start_siege(
    ctx: &ReducerContext,
    target_settlement_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `target_settlement_id`: Settlement to siege

**Validation**:
- Caller's empire has declare_war permission
- Target settlement marked for siege
- Not already under siege
- Sufficient siege preparation

**Effects**:
- Creates `EmpireSiegeState` entry
- Starts siege timer
- Enables siege mechanics

## Combat

### attack

**File**: `handlers/attack.rs`

Performs an attack.

```rust
#[spacetimedb::reducer]
pub fn attack(
    ctx: &ReducerContext,
    target_entity_id: u64,
    combat_action_id: i32,
) -> Result<(), String>
```

**Parameters**:
- `target_entity_id`: Entity to attack
- `combat_action_id`: Attack type/ability

**Validation**:
- Target is valid and attackable
- Player has weapon/ability
- Target in range
- Sufficient stamina
- Not on cooldown

**Effects**:
- Calculates damage
- Creates `AttackOutcomeState` entry
- Applies damage to target
- Updates combat state
- Triggers threat/aggro

**Errors**:
- "Target not found"
- "Out of range"
- "Insufficient stamina"
- "On cooldown"

### attack_start

**File**: `handlers/attack.rs`

Initiates attack sequence.

```rust
#[spacetimedb::reducer]
pub fn attack_start(
    ctx: &ReducerContext,
    target_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `target_entity_id`: Entity to attack

**Effects**:
- Sets player action to Attack
- Enters combat state
- Schedules attack execution

### auto_attack_start

**File**: `handlers/attack.rs`

Enables auto-attack mode.

```rust
#[spacetimedb::reducer]
pub fn auto_attack_start(
    ctx: &ReducerContext,
    target_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `target_entity_id`: Entity to auto-attack

**Effects**:
- Enables auto-attack
- Continuously attacks until stopped or target dies

### auto_attack_stop

**File**: `handlers/attack.rs`

Disables auto-attack mode.

```rust
#[spacetimedb::reducer]
pub fn auto_attack_stop(ctx: &ReducerContext) -> Result<(), String>
```

**Effects**:
- Stops auto-attack
- Remains in combat

### target_update

**File**: `handlers/attack.rs`

Changes combat target.

```rust
#[spacetimedb::reducer]
pub fn target_update(
    ctx: &ReducerContext,
    new_target_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `new_target_entity_id`: New target

**Effects**:
- Updates current target
- Redirects auto-attack

## Trading

### trade_initiate_session

**File**: `handlers/player_trade/trade_initiate_session.rs`

Starts a trade with another player.

```rust
#[spacetimedb::reducer]
pub fn trade_initiate_session(
    ctx: &ReducerContext,
    target_player_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `target_player_entity_id`: Player to trade with

**Validation**:
- Target player exists and is online
- Both players are near each other
- Neither player is busy

**Effects**:
- Creates `TradeSessionState` entry
- Sends trade request to target

### trade_accept_session

**File**: `handlers/player_trade/trade_accept_session.rs`

Accepts a trade request.

```rust
#[spacetimedb::reducer]
pub fn trade_accept_session(
    ctx: &ReducerContext,
    trade_session_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `trade_session_id`: Trade session to accept

**Effects**:
- Activates trade session
- Opens trade window for both players

### trade_add_item

**File**: `handlers/player_trade/trade_add_item.rs`

Adds item to trade offer.

```rust
#[spacetimedb::reducer]
pub fn trade_add_item(
    ctx: &ReducerContext,
    trade_session_id: u64,
    pocket_index: i32,
    stack_index: i32,
    quantity: i32,
) -> Result<(), String>
```

**Parameters**:
- `trade_session_id`: Active trade session
- `pocket_index`, `stack_index`: Item location
- `quantity`: Amount to trade

**Effects**:
- Adds item to player's trade offer
- Resets both players' acceptance

### trade_accept

**File**: `handlers/player_trade/trade_accept.rs`

Accepts current trade offer.

```rust
#[spacetimedb::reducer]
pub fn trade_accept(
    ctx: &ReducerContext,
    trade_session_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `trade_session_id`: Trade session to accept

**Validation**:
- Both players have accepted
- Both have inventory space

**Effects**:
- Exchanges items between players
- Closes trade session
- Updates inventories

### order_post_sell_order

**File**: `handlers/player_trade/order_post_sell_order.rs`

Posts a sell order on marketplace.

```rust
#[spacetimedb::reducer]
pub fn order_post_sell_order(
    ctx: &ReducerContext,
    marketplace_entity_id: u64,
    item_id: i32,
    quantity: i32,
    price_per_unit: i32,
) -> Result<(), String>
```

**Parameters**:
- `marketplace_entity_id`: Marketplace building
- `item_id`: Item to sell
- `quantity`: Amount to sell
- `price_per_unit`: Price per item

**Validation**:
- Player has items
- Marketplace exists
- Price is valid

**Effects**:
- Transfers items to marketplace
- Creates `SellOrderState` entry
- Makes visible to buyers

### order_post_buy_order

**File**: `handlers/player_trade/order_post_buy_order.rs`

Posts a buy order on marketplace.

```rust
#[spacetimedb::reducer]
pub fn order_post_buy_order(
    ctx: &ReducerContext,
    marketplace_entity_id: u64,
    item_id: i32,
    quantity: i32,
    price_per_unit: i32,
) -> Result<(), String>
```

**Parameters**:
- `marketplace_entity_id`: Marketplace building
- `item_id`: Item to buy
- `quantity`: Amount to buy
- `price_per_unit`: Price willing to pay

**Validation**:
- Player has currency

**Effects**:
- Locks currency
- Creates `BuyOrderState` entry
- Matches with sell orders

## Social and Communication

### chat_post_message

**File**: `handlers/chat_post_message.rs`

Sends a chat message.

```rust
#[spacetimedb::reducer]
pub fn chat_post_message(
    ctx: &ReducerContext,
    channel: ChatChannel,
    message: String,
) -> Result<(), String>
```

**Parameters**:
- `channel`: Chat channel (Local, Claim, Empire, Global)
- `message`: Message text

**Validation**:
- Message length valid
- Player has permission for channel
- Not rate limited
- Not chat banned

**Effects**:
- Creates `ChatMessageState` entry
- Broadcasts to channel subscribers

**Errors**:
- "Message too long"
- "Rate limited"
- "Chat banned"

### emote

**File**: `handlers/player/emote.rs`

Performs an emote.

```rust
#[spacetimedb::reducer]
pub fn emote(
    ctx: &ReducerContext,
    emote_id: i32,
) -> Result<(), String>
```

**Parameters**:
- `emote_id`: Emote to perform

**Effects**:
- Sets secondary action to emote
- Triggers animation
- Visible to nearby players

## Admin Commands

*Requires Admin or GM role*

### admin_sign_out

**File**: `handlers/admin/admin_sign_out.rs`

Force signs out a player.

```rust
#[spacetimedb::reducer]
pub fn admin_sign_out(
    ctx: &ReducerContext,
    player_entity_id: u64,
) -> Result<(), String>
```

**Parameters**:
- `player_entity_id`: Player to sign out

**Authorization**: Admin or GM

**Effects**:
- Force signs out player
- Logs action

### admin_broadcast

**File**: `handlers/admin/admin_broadcast.rs`

Sends server announcement.

```rust
#[spacetimedb::reducer]
pub fn admin_broadcast(
    ctx: &ReducerContext,
    message: String,
) -> Result<(), String>
```

**Parameters**:
- `message`: Announcement text

**Authorization**: Admin or GM

**Effects**:
- Sends message to all players
- Creates system chat entry

### admin_grant_shards

**File**: `handlers/admin/admin_grant_shards.rs`

Grants premium currency to player.

```rust
#[spacetimedb::reducer]
pub fn admin_grant_shards(
    ctx: &ReducerContext,
    player_entity_id: u64,
    amount: i32,
) -> Result<(), String>
```

**Parameters**:
- `player_entity_id`: Recipient player
- `amount`: Shards to grant

**Authorization**: Admin

**Effects**:
- Adds shards to player
- Logs transaction

## Cheat Commands

*Requires Developer role or dev password*

### cheat_item_stack_grant

**File**: `handlers/cheats/cheat_item_stack_grant.rs`

Spawns items in inventory.

```rust
#[spacetimedb::reducer]
pub fn cheat_item_stack_grant(
    ctx: &ReducerContext,
    dev_pw: String,
    item_id: i32,
    quantity: i32,
) -> Result<(), String>
```

**Parameters**:
- `dev_pw`: Developer password (from config)
- `item_id`: Item to grant
- `quantity`: Amount

**Authorization**: Developer role OR valid dev password

**Effects**:
- Adds items to player inventory

### cheat_teleport_float

**File**: `handlers/cheats/cheat_teleport_float.rs`

Instant teleport to coordinates.

```rust
#[spacetimedb::reducer]
pub fn cheat_teleport_float(
    ctx: &ReducerContext,
    dev_pw: String,
    x: f32,
    z: f32,
) -> Result<(), String>
```

**Parameters**:
- `dev_pw`: Developer password
- `x`, `z`: Destination coordinates

**Effects**:
- Instantly teleports player

### cheat_experience_grant

**File**: `handlers/cheats/cheat_experience_grant.rs`

Grants experience in a skill.

```rust
#[spacetimedb::reducer]
pub fn cheat_experience_grant(
    ctx: &ReducerContext,
    dev_pw: String,
    skill_id: i32,
    amount: i32,
) -> Result<(), String>
```

**Parameters**:
- `dev_pw`: Developer password
- `skill_id`: Skill to level
- `amount`: XP to grant

**Effects**:
- Adds experience
- Levels up if threshold reached

**42 total cheat reducers** for development and testing

## Summary

- **675+ reducers** across all categories
- **Strongly typed** parameters and returns
- **Permission-based** access control
- **Transactional** execution
- **Client-callable** via SpacetimeDB SDK

For implementation details, see source files in:
- `BitCraftServer/packages/game/src/game/handlers/`
- `BitCraftServer/packages/global_module/src/game/handlers/`

## Next Steps

- **[Game Systems](game-systems.md)** - How systems use these reducers
- **[Data Models](data-models.md)** - Tables manipulated by reducers
- **[Architecture](architecture.md)** - Patterns and design decisions
