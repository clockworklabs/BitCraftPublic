# BitCraft Server Architecture

## Table of Contents

- [System Architecture](#system-architecture)
- [Design Patterns](#design-patterns)
- [Coordinate Systems](#coordinate-systems)
- [Entity Management](#entity-management)
- [State Management](#state-management)
- [Performance Optimization](#performance-optimization)

## System Architecture

### Multi-Region Distributed Architecture

BitCraft employs a distributed architecture with multiple server instances handling different geographic regions of the game world.

```
                    ┌─────────────────┐
                    │  Global Server  │
                    │  (Global Module)│
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────▼─────┐       ┌────▼─────┐       ┌────▼─────┐
    │ Region 1 │       │ Region 2 │       │ Region 3 │
    │  Server  │       │  Server  │       │  Server  │
    └──────────┘       └──────────┘       └──────────┘
         │                   │                   │
    Players in         Players in         Players in
    Region 1          Region 2          Region 3
```

#### Global Server Responsibilities

- **Empire Management**: Multi-region guild system
- **Player Accounts**: Global player identity and metadata
- **Cross-Region Coordination**: Message routing between regions
- **Global Chat**: Server-wide communication
- **Premium Items**: Hub and cosmetic items
- **Player Transfers**: Moving players between regions

#### Region Server Responsibilities

- **World State**: Terrain, resources, buildings for assigned region
- **Player Interactions**: Movement, combat, crafting, building
- **Local Claims**: Territory ownership within region
- **Enemy AI**: Enemy spawning and behavior
- **Resource Spawning**: Dynamic resource generation
- **Local Events**: Region-specific gameplay events

#### Region Assignment

Regions are assigned based on world coordinates:

```rust
pub struct RegionCoordinates {
    pub region_index: u32,      // This region's index
    pub region_count_sqrt: u32, // Square root of total regions
}

// Example: 4 regions in 2x2 grid
// Region 0: Northwest quadrant
// Region 1: Northeast quadrant
// Region 2: Southwest quadrant
// Region 3: Southeast quadrant
```

### Inter-Module Communication

#### Message Protocol

Modules communicate via the `InterModuleMessageV3` table:

```rust
#[spacetimedb::table(name = inter_module_message_v3)]
pub struct InterModuleMessageV3 {
    #[primary_key]
    pub id: u64,
    pub destination: InterModuleDestination,
    pub sender_module_identity: Identity,
    pub contents: MessageContentsV3,
    pub timestamp: u64,
}

pub enum InterModuleDestination {
    Global,
    AllOtherRegions,
    Region(u32),
}
```

#### Message Types

**24 distinct message types** including:
- `PlayerCreate` - Create player on global server
- `TransferPlayer` - Move player between regions
- `UserUpdateRegion` - Update player's current region
- `OnPlayerJoinedEmpire` / `OnPlayerLeftEmpire` - Empire membership sync
- `EmpireCreateBuilding` - Sync empire buildings across regions
- `GrantHubItem` - Grant premium items to players

#### Shared Table Reducer Pattern

Automatically replicates table changes across modules:

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]  // Macro generates inter-module sync code
pub fn empire_form(ctx: &ReducerContext, request: EmpireFormRequest) -> Result<(), String> {
    // Create empire
    let empire = EmpireState {
        entity_id: create_entity(ctx),
        name: request.name,
        // ... more fields
    };

    // Insert with automatic replication
    EmpireState::insert_shared(
        ctx,
        empire,
        InterModuleDestination::AllOtherRegions
    );

    Ok(())
}
```

**How It Works**:
1. `SharedTransactionAccumulator` tracks all table operations
2. Operations batched into `InterModuleTableUpdates` message
3. Message sent to destination modules
4. Receiving modules apply updates atomically
5. All regions see consistent empire state

### Client-Server Communication

SpacetimeDB provides **automatic subscription-based synchronization**:

1. **Client Subscribes**: Requests specific tables or query results
2. **Initial State**: Receives current matching rows
3. **Automatic Updates**: Receives INSERT/UPDATE/DELETE as they occur
4. **Reducers**: Client calls reducers to trigger state changes

**No manual synchronization code needed** - the platform handles it.

## Design Patterns

### 1. Entity-Component System (ECS-Like)

BitCraft implements a table-based entity-component architecture:

**Entities** are identified by unique `entity_id: u64`

**Components** are table rows sharing the same `entity_id`:

```rust
// Creating an entity with components
let entity_id = create_entity(ctx); // Increments global counter

// Add components by inserting into tables
ctx.db.location_state().insert(LocationState {
    entity_id,
    location: SmallHexTile { q: 10, r: 20, s: -30 },
});

ctx.db.health_state().insert(HealthState {
    entity_id,
    health: 100.0,
    max_health: 100.0,
});

ctx.db.player_state().insert(PlayerState {
    entity_id,
    signed_in: true,
    // ... more fields
});
```

**Querying Entities**:

```rust
// Get all players with health
for player in ctx.db.player_state().iter() {
    if let Some(health) = ctx.db.health_state().entity_id().find(&player.entity_id) {
        // Process player with health component
    }
}
```

**80+ Entity Components** including:
- `LocationState`, `MobileEntityState` - Position and movement
- `HealthState`, `StaminaState` - Vital statistics
- `InventoryState`, `EquipmentState` - Items
- `PlayerState`, `EnemyState`, `BuildingState` - Entity type data
- `CombatState`, `BuffState` - Combat system
- `ClaimState`, `ClaimMemberState` - Territory ownership

### 2. Progressive Action Pattern

Used for **long-running actions** that can be paused/resumed:

```rust
#[spacetimedb::table(name = progressive_action_state, public)]
pub struct ProgressiveActionState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub recipe_id: i32,
    pub building_entity_id: u64,
    pub lock_expiration: u64,
    pub items_completed: i32,
    pub items_requested: i32,
    pub suspended_timestamp: u64,
    pub status: ProgressiveActionStatus,
}

pub enum ProgressiveActionStatus {
    InProgress,
    Suspended,
    Complete,
}
```

**Use Cases**:
- **Crafting**: Multi-item batch crafting
- **Construction**: Multi-stage building
- **Terraforming**: Large area modifications
- **Extraction**: Resource harvesting

**Flow**:
1. Player initiates action → Create `ProgressiveActionState`
2. Action proceeds in timed steps
3. Player can suspend (e.g., to move)
4. Resume later from same state
5. Complete after all items/stages finished

**Benefits**:
- Interruptible actions
- Resumable across sessions
- Batch operations without blocking
- Consistent progress tracking

### 3. Player Action Layer System

Allows **concurrent actions** without conflicts:

```rust
pub enum PlayerActionLayer {
    Base,       // Primary action: movement, crafting, combat
    Secondary,  // Simultaneous: emotes while moving
    Tertiary,   // Additional layer
}

#[spacetimedb::table(name = player_action_state, public)]
pub struct PlayerActionState {
    #[primary_key]
    pub entity_id: u64,
    pub player_action: PlayerAction,
    pub player_action_layer: PlayerActionLayer,
    pub timestamp: u64,
}
```

**Examples**:
- **Base Layer**: Crafting at a workshop
- **Secondary Layer**: Waving emote while crafting
- Walking (Base) + Pointing emote (Secondary)

### 4. Footprint Delta System

**Buildings and deployables** have complex shapes:

```rust
pub struct FootprintDelta {
    pub offset_q: i32,  // Hex tile offset from origin
    pub offset_r: i32,
    pub footprint_type: FootprintType,
}

pub enum FootprintType {
    Buildable,      // Can build on this tile
    NonBuildable,   // Blocks building placement
    Ground,         // Requires ground terrain
    Water,          // Requires water terrain
}
```

**Applications**:
- **Placement Validation**: Check all footprint tiles are valid
- **Collision Detection**: Test overlap with other entities
- **Visual Rendering**: Client displays building shape
- **Territory Claims**: Multi-tile building ownership

**Example** - Large building with entrance:
```rust
vec![
    FootprintDelta { offset_q: 0, offset_r: 0, footprint_type: Buildable },
    FootprintDelta { offset_q: 1, offset_r: 0, footprint_type: Buildable },
    FootprintDelta { offset_q: 0, offset_r: 1, footprint_type: Buildable },
    FootprintDelta { offset_q: 1, offset_r: 1, footprint_type: NonBuildable }, // Entrance
]
```

### 5. Discovery Lazy-Sync Pattern

**Problem**: Thousands of entities in world, can't send all to every client

**Solution**: Knowledge-based discovery system

```rust
#[spacetimedb::table(name = knowledge_item_state, public)]
pub struct KnowledgeItemState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub player_entity_id: u64,
    pub item_id: i32,
    pub knowledge_state: KnowledgeState,
}

pub enum KnowledgeState {
    Unknown,    // Player hasn't encountered
    Discovered, // Player has seen/heard about
    Acquired,   // Player has obtained/can use
}
```

**Separate Knowledge Tables**:
- `KnowledgeItemState` - Items
- `KnowledgeBuildingState` - Buildings
- `KnowledgeRecipeState` - Crafting recipes
- `KnowledgeEnemyState` - Enemies
- `KnowledgeResourceState` - Resources

**Discovery Flow**:
1. Player moves near entity
2. `discover_entities` reducer called with entity IDs
3. Knowledge table updated to `Discovered`
4. Client requests full entity data
5. Player interacts → Knowledge becomes `Acquired`

**Benefits**:
- Reduces initial data transfer
- Gradual world revelation (fog of war)
- Discovery progression system
- Network bandwidth optimization

### 6. Claim Permission System

**Centralized permission checking** for all claim-restricted actions:

```rust
impl ClaimState {
    pub fn can_access_inventory(
        ctx: &ReducerContext,
        claim_entity_id: u64,
        player_entity_id: u64
    ) -> bool {
        // Check if player owns claim
        if is_owner(ctx, claim_entity_id, player_entity_id) {
            return true;
        }

        // Check if player is member with permission
        if let Some(member) = get_member(ctx, claim_entity_id, player_entity_id) {
            return member.permissions.inventory;
        }

        false
    }

    pub fn can_build(...) -> bool { /* ... */ }
    pub fn can_use_buildings(...) -> bool { /* ... */ }
}
```

**Permission Types**:
- `Inventory` - Access claim storage
- `Build` - Place/remove buildings
- `Usage` - Use building functions
- `Recruit` - Invite new members
- `RemoveMember` - Kick members
- `EditPermissions` - Change member permissions
- `AddTile` - Expand territory
- `RemoveTile` - Shrink territory

**Applied In**:
- Building placement/removal
- Storage access
- Resource extraction on claim
- Territory modification
- Member management

### 7. Agent Scheduling Pattern

**Background tasks** run on schedules:

```rust
// Schedule an agent
#[spacetimedb::reducer]
pub fn schedule_player_regen_agent(ctx: &ReducerContext, delay_ms: u64) {
    ctx.db.player_regen_schedule().insert(PlayerRegenSchedule {
        scheduled_id: ctx.timestamp.micros_since_epoch,
        scheduled_at: ctx.timestamp.add_duration(Duration::from_millis(delay_ms)),
    });
}

// Agent table triggers execution
#[spacetimedb::table(scheduled(player_regen_agent, at = scheduled_at))]
pub struct PlayerRegenSchedule {
    scheduled_id: u64,
    scheduled_at: Timestamp,
}

// Agent reducer runs at scheduled time
#[spacetimedb::reducer]
pub fn player_regen_agent(ctx: &ReducerContext, arg: PlayerRegenSchedule) {
    // Regenerate health/stamina for all players
    for player in ctx.db.player_state().iter().filter(|p| p.signed_in) {
        regenerate_player(ctx, player.entity_id);
    }

    // Reschedule for next run
    schedule_player_regen_agent(ctx, 5000); // Run every 5 seconds
}
```

**23 Active Agents**:
- `player_regen_agent` - Health/stamina regeneration
- `enemy_regen_agent` - Enemy health recovery
- `resources_regen` - Resource respawning
- `building_decay_agent` - Building maintenance
- `auto_logout_agent` - Kick inactive players
- `starving_agent` - Hunger damage
- `environment_debuff_agent` - Environmental effects
- `npc_ai_agent` - NPC behavior
- More specialized agents...

### 8. Dimension System

**Multiple world spaces** for interiors and instances:

```rust
pub enum DimensionType {
    Overworld,  // Main open world
    Interior,   // Building interiors
    Housing,    // Player housing instances
}

#[spacetimedb::table(name = dimension_description_state, public)]
pub struct DimensionDescriptionState {
    #[primary_key]
    pub dimension: u32,
    pub dimension_type: DimensionType,
    pub parent_building_entity_id: u64,
    pub player_count: i32,
}

#[spacetimedb::table(name = mobile_entity_state, public)]
pub struct MobileEntityState {
    #[primary_key]
    pub entity_id: u64,
    pub location_x: f32,
    pub location_z: f32,
    pub dimension: u32,  // Which dimension entity is in
    // ...
}
```

**Portal System**:
```rust
pub struct PortalLocationDesc {
    pub dimension: u32,
    pub location_x: f32,
    pub location_z: f32,
}
```

**Use Cases**:
- Large building interiors (separate from overworld)
- Player housing instances (personalized spaces)
- Instanced dungeons or special areas
- Isolation for specific gameplay

## Coordinate Systems

BitCraft uses **hexagonal tiles** for its world grid.

### Coordinate Type Hierarchy

```
FloatHexTile (precise positions)
    ↕
SmallHexTile (individual tiles)
    ↕
LargeHexTile (chunks)
    ↕
ChunkCoordinates (chunk indexing)
    ↕
RegionCoordinates (multi-region)
```

### Axial Hex Coordinates

**Primary Representation**: `SmallHexTile`

```rust
pub struct SmallHexTile {
    pub q: i32,  // Column axis
    pub r: i32,  // Row axis
    pub s: i32,  // Diagonal axis (always q + r + s = 0)
}
```

**Why Three Coordinates?**
Hexagons require 3 axes for consistent distance calculations:
- `q + r + s = 0` (constraint)
- Distance = `(|q1-q2| + |r1-r2| + |s1-s2|) / 2`

### Coordinate Conversions

**Extensive conversion methods** (13 coordinate types):

```rust
impl SmallHexTile {
    pub fn from_float(float: FloatHexTile) -> Self { /* ... */ }
    pub fn to_float(&self) -> FloatHexTile { /* ... */ }
    pub fn to_large(&self) -> LargeHexTile { /* ... */ }
    pub fn to_offset(&self) -> OffsetCoordinates { /* ... */ }
    // ... many more
}
```

### Chunk System

**World is divided into chunks** for efficient loading:

```rust
pub const TERRAIN_CHUNK_WIDTH: i32 = 10;  // Tiles wide
pub const TERRAIN_CHUNK_HEIGHT: i32 = 10; // Tiles tall

pub struct ChunkCoordinates {
    pub chunk_column: i32,
    pub chunk_row: i32,
}
```

**Chunk Indexing**:
```rust
pub fn chunk_index(chunk_coords: ChunkCoordinates, region_coords: RegionCoordinates) -> u64 {
    // Unique index for each chunk in region
}
```

### Neighbor Calculations

**Hexagons have 6 neighbors** at consistent distance:

```rust
pub const HEX_DIRECTIONS: [SmallHexTile; 6] = [
    SmallHexTile { q: 1, r: 0, s: -1 },   // East
    SmallHexTile { q: 1, r: -1, s: 0 },   // Northeast
    SmallHexTile { q: 0, r: -1, s: 1 },   // Northwest
    SmallHexTile { q: -1, r: 0, s: 1 },   // West
    SmallHexTile { q: -1, r: 1, s: 0 },   // Southwest
    SmallHexTile { q: 0, r: 1, s: -1 },   // Southeast
];

impl SmallHexTile {
    pub fn get_neighbors(&self) -> Vec<SmallHexTile> {
        HEX_DIRECTIONS.iter().map(|dir| self.add(dir)).collect()
    }
}
```

## Entity Management

### Entity Creation

**Global counter** ensures unique IDs:

```rust
#[spacetimedb::table(name = globals, public)]
pub struct Globals {
    #[primary_key]
    pub version: u32,
    pub entity_pk_counter: u64,  // Increments for each entity
    // ... more globals
}

pub fn create_entity(ctx: &ReducerContext) -> u64 {
    let mut globals = ctx.db.globals().version().find(&0).unwrap();
    globals.entity_pk_counter += 1;
    ctx.db.globals().version().update(globals.clone());
    globals.entity_pk_counter
}
```

### Entity Types

```rust
pub enum EntityType {
    Player,
    Enemy,
    Resource,
    Building,
    Deployable,
    Npc,
}

pub fn get_entity_type(ctx: &ReducerContext, entity_id: u64) -> Option<EntityType> {
    if ctx.db.player_state().entity_id().find(&entity_id).is_some() {
        return Some(EntityType::Player);
    }
    if ctx.db.enemy_state().entity_id().find(&entity_id).is_some() {
        return Some(EntityType::Enemy);
    }
    // ... check other types
    None
}
```

### Entity Deletion

**Cascading deletion** removes all components:

```rust
pub fn delete_entity(ctx: &ReducerContext, entity_id: u64) {
    // Delete from all component tables
    ctx.db.location_state().entity_id().delete(&entity_id);
    ctx.db.mobile_entity_state().entity_id().delete(&entity_id);
    ctx.db.health_state().entity_id().delete(&entity_id);
    ctx.db.inventory_state().entity_id().delete(&entity_id);
    // ... delete from all relevant tables

    // Delete type-specific data
    ctx.db.player_state().entity_id().delete(&entity_id);
    ctx.db.enemy_state().entity_id().delete(&entity_id);
    ctx.db.building_state().entity_id().delete(&entity_id);
    // ...
}
```

## State Management

### Timestamp Consistency

**All timestamps use SpacetimeDB's `Timestamp`**:

```rust
pub fn current_time_ms(ctx: &ReducerContext) -> u64 {
    ctx.timestamp.micros_since_epoch / 1000
}
```

**Why Server Timestamps?**
- Prevents client-side time manipulation
- Consistent across all players
- Monotonic (never goes backwards)
- Synchronized with scheduled agents

### Error Handling

**Custom macros** for clean error handling:

```rust
// Return error if None
unwrap_or_err!(optional_value, "Error message")

// Return from function if None
unwrap_or_return!(optional_value)

// Continue loop if None
unwrap_or_continue!(optional_value)
```

**Usage**:
```rust
#[spacetimedb::reducer]
pub fn example_reducer(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    let player = unwrap_or_err!(
        ctx.db.player_state().entity_id().find(&entity_id),
        "Player not found"
    );

    // Continue with player
    Ok(())
}
```

### Cache Systems

**Spatial Indexing**:
```rust
pub struct LocationCache {
    // Maps chunk_index -> Vec<entity_id>
    entities_by_chunk: HashMap<u64, Vec<u64>>,
}
```

**Terrain Caching**:
```rust
pub struct TerrainChunkCache {
    chunks: HashMap<ChunkCoordinates, TerrainChunk>,
}
```

**Benefits**:
- Fast spatial queries (nearby entities)
- Reduced table scans
- Efficient collision detection
- Quick chunk loading

## Performance Optimization

### Database Indexes

**Strategic indexing** on frequently queried fields:

```rust
#[spacetimedb::table(name = player_state, public)]
pub struct PlayerState {
    #[primary_key]  // Automatic B-tree index
    pub entity_id: u64,

    #[index(btree)]  // Additional index
    pub signed_in: bool,

    // ...
}
```

**Indexed Fields**:
- All primary keys (automatic)
- `signed_in` flags (filter active players)
- `owner_entity_id` (query by owner)
- `claim_entity_id` (query by claim)
- Foreign key relationships

### Build Optimization

```toml
[profile.release]
opt-level = 's'    # Optimize for size (WASM constraint)
lto = true         # Link-time optimization
codegen-units = 1  # Single codegen unit for better optimization
```

**Why Size Over Speed?**
- SpacetimeDB modules compile to WASM
- Smaller WASM = faster loading
- Less memory usage in database engine

### Batching Operations

**Agents batch operations** for efficiency:

```rust
pub fn resources_regen(ctx: &ReducerContext, arg: ResourcesRegenSchedule) {
    // Collect all resources needing respawn
    let mut resources_to_spawn = Vec::new();

    for resource_type in all_resource_types() {
        let current_count = count_resources(ctx, resource_type);
        let target_count = get_target_count(resource_type);

        if current_count < target_count {
            resources_to_spawn.push((resource_type, target_count - current_count));
        }
    }

    // Batch spawn all at once
    for (resource_type, count) in resources_to_spawn {
        spawn_resources(ctx, resource_type, count);
    }
}
```

### Query Optimization

**Filter early, iterate minimal**:

```rust
// Good: Filter with index, then process
let signed_in_players: Vec<_> = ctx.db.player_state()
    .signed_in()  // Uses index
    .filter(|s| s == &true)
    .collect();

for player in signed_in_players {
    // Process only signed-in players
}

// Bad: Iterate all, then filter
for player in ctx.db.player_state().iter() {
    if player.signed_in {  // No index benefit
        // Process
    }
}
```

### Lazy Loading

**Discovery system** prevents loading all entity data:
- Client subscribes to knowledge tables (small)
- Requests full data only for discovered entities
- Incremental world revelation
- Reduces initial connection payload

## Next Steps

- **[Data Models](data-models.md)** - Complete table schemas and structures
- **[Reducers API](reducers-api.md)** - Full reducer reference
- **[Game Systems](game-systems.md)** - Game mechanics deep dive
- **[Deployment](deployment.md)** - Production deployment guide
