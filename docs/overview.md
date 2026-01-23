# BitCraft Server Overview

## Table of Contents

- [Introduction](#introduction)
- [Project Structure](#project-structure)
- [Key Concepts](#key-concepts)
- [Technology Stack](#technology-stack)
- [Development Workflow](#development-workflow)

## Introduction

BitCraft is a community-driven MMORPG where players collaborate to shape a procedurally generated world. The server implementation is built on SpacetimeDB, a real-time database platform specifically designed for multiplayer games.

### What is SpacetimeDB?

SpacetimeDB is a novel approach to game backend development where:
- **Data lives in tables** - Similar to a traditional database
- **Logic lives in reducers** - Functions that modify data (like API endpoints)
- **Clients subscribe to tables** - Automatic real-time synchronization
- **Everything is reactive** - Changes propagate automatically to subscribed clients

This eliminates the need for manual client-server synchronization code and enables a purely reactive programming model.

## Project Structure

```
BitCraftPublic/
├── BitCraftServer/
│   └── packages/
│       ├── game/              # Region server module
│       │   ├── src/
│       │   │   ├── agents/    # Background scheduled tasks
│       │   │   ├── game/      # Core game logic
│       │   │   │   ├── coordinates/    # Hex coordinate system
│       │   │   │   ├── entities/       # Entity components
│       │   │   │   ├── handlers/       # Reducer implementations
│       │   │   │   ├── world_gen/      # World generation
│       │   │   │   ├── static_data/    # Game configuration
│       │   │   │   └── game_state/     # State utilities
│       │   │   ├── messages/  # Table definitions
│       │   │   ├── inter_module/  # Cross-region communication
│       │   │   └── lib.rs     # Module entry point
│       │   ├── config/        # Environment configurations
│       │   └── Cargo.toml
│       │
│       └── global_module/     # Global server module
│           ├── src/
│           │   ├── agents/    # Global scheduled tasks
│           │   ├── game/      # Global game logic
│           │   │   └── handlers/   # Empire, admin handlers
│           │   ├── messages/  # Global table definitions
│           │   └── inter_module/  # Region message handlers
│           └── Cargo.toml
│
├── docs/                      # This documentation
├── images/                    # Assets for README
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

### Two-Module Architecture

BitCraft uses a **distributed multi-module architecture**:

#### 1. Game Module (Region Server)
- **Location**: `packages/game/`
- **Purpose**: Handles regional game logic and player interactions
- **Scope**:
  - Player movement, actions, and combat
  - Resource harvesting and world interactions
  - Building construction and management
  - Local claim management
  - Region-specific state

#### 2. Global Module
- **Location**: `packages/global_module/`
- **Purpose**: Manages cross-region state and coordination
- **Scope**:
  - Empire system (multi-region guilds)
  - Player account management
  - Global chat
  - Cross-region player transfers
  - Premium/hub items

### Package Details

#### Game Package (`bitcraft-spacetimedb`)

**Crate Type**: `cdylib` (C dynamic library - required by SpacetimeDB)

**Key Dependencies**:
```toml
spacetimedb = "=1.6.0"
probability = "0.20.3"  # Procedural generation
glam = "0.30.9"         # Vector mathematics
strum = "0.24"          # Enum utilities
queues = "1.0"          # Data structures
csv = "1.1"             # Static data import
```

**Module Statistics**:
- **Reducers**: 650+ functions
- **Tables**: ~180 tables
- **Agents**: 23 background tasks
- **Source Files**: 350+ Rust files
- **Lines of Code**: ~140,000

#### Global Module

**Crate Type**: `cdylib`

**Module Statistics**:
- **Reducers**: 25+ functions
- **Tables**: ~20 tables
- **Agents**: 3 background tasks
- **Primary Focus**: Empire management and cross-region coordination

## Key Concepts

### Entities

In BitCraft, everything in the game world is an **entity** identified by a unique `entity_id: u64`.

**Entity Types**:
- `Player` - Player characters
- `Enemy` - Hostile NPCs
- `Resource` - Harvestable resources (trees, rocks, etc.)
- `Building` - Constructed structures
- `Deployable` - Placed items (campfires, signs, etc.)
- `Npc` - Friendly NPCs

**Entity-Component Pattern**:
Entities are composed of multiple table rows sharing the same `entity_id`:

```rust
// Entity 12345 might have:
PlayerState { entity_id: 12345, ... }
MobileEntityState { entity_id: 12345, location_x: 100.0, ... }
HealthState { entity_id: 12345, health: 100.0, ... }
InventoryState { entity_id: 12345, pockets: [...], ... }
```

### Reducers

**Reducers** are the only way to modify data in SpacetimeDB. They function like API endpoints in traditional web services.

**Characteristics**:
- Defined with `#[spacetimedb::reducer]` attribute
- Automatically exposed to clients
- Run in database transactions (atomic)
- Can be scheduled to run at specific times

**Example**:
```rust
#[spacetimedb::reducer]
pub fn player_move(
    ctx: &ReducerContext,
    destination_x: f32,
    destination_z: f32,
    is_running: bool,
) -> Result<(), String> {
    // Validate movement
    // Update player position
    // Deduct stamina
    // Update exploration
    Ok(())
}
```

### Tables

**Tables** store all game state in SpacetimeDB. They are defined as Rust structs with the `#[spacetimedb::table]` attribute.

**Characteristics**:
- Automatic indexes on `#[primary_key]` fields
- Can be marked `public` for client visibility
- Support custom indexes with `#[index(btree)]`
- Automatically synchronized to subscribed clients

**Example**:
```rust
#[spacetimedb::table(name = player_state, public)]
pub struct PlayerState {
    #[primary_key]
    pub entity_id: u64,
    pub signed_in: bool,
    pub session_start_timestamp: u64,
    pub last_action_timestamp: u64,
    // ... more fields
}
```

### Agents

**Agents** are scheduled background tasks that run periodically.

**Common Uses**:
- Health/stamina regeneration
- Resource respawning
- Building decay
- NPC AI updates
- Auto-logout for inactive players

**Implementation**:
```rust
#[spacetimedb::reducer]
pub fn schedule_player_regen_agent(ctx: &ReducerContext, initial_delay: u64) {
    ctx.db.player_regen_schedule().insert(PlayerRegenSchedule {
        scheduled_id: ctx.timestamp.micros_since_epoch,
        scheduled_at: ctx.timestamp.add_duration(Duration::from_millis(initial_delay)),
    });
}

#[spacetimedb::table(scheduled(player_regen_agent, at = scheduled_at))]
pub struct PlayerRegenSchedule {
    scheduled_id: u64,
    scheduled_at: Timestamp,
}
```

### Coordinate System

BitCraft uses a **hexagonal tile-based coordinate system** for its world.

**Coordinate Types**:
- `SmallHexTile` - Axial coordinates (q, r, s) for individual tiles
- `LargeHexTile` - Chunk-level coordinates
- `FloatHexTile` - Precise floating-point positions for entities
- `OffsetCoordinates` - Row/column representation
- `ChunkCoordinates` - Indexing for terrain chunks

**Why Hexagons?**
- Each tile has 6 equidistant neighbors (better than square grids)
- More natural pathfinding
- Better visual aesthetics for organic worlds

### Static Data

**Static data** refers to game configuration that defines items, buildings, recipes, enemies, etc.

**Storage**:
- Imported from CSV files
- Stored in "Desc" tables (e.g., `ItemDesc`, `BuildingDesc`)
- Versioned using staging tables (`staged_static_data_v3`)
- Can be updated without redeploying code

**Categories**:
- Items (500+ items)
- Buildings (200+ types)
- Crafting recipes
- Enemy definitions
- Biome configurations
- Game balance parameters

### Inter-Module Communication

Since BitCraft runs multiple region servers and one global server, they need to communicate.

**Message System**:
- Messages stored in `InterModuleMessageV3` table
- Routed through global server
- Support for targeting: Global, AllOtherRegions, or specific Region

**Shared Table Reducers**:
Special reducers that automatically replicate table changes across modules:

```rust
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn empire_form(ctx: &ReducerContext, request: EmpireFormRequest) -> Result<(), String> {
    // Changes to empire tables automatically sync to all regions
    EmpireState::insert_shared(ctx, empire, InterModuleDestination::AllOtherRegions);
    Ok(())
}
```

## Technology Stack

### Core Technologies

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | 1.70+ | Primary programming language |
| SpacetimeDB | 1.6.0 | Database and backend platform |
| WASM | - | Compilation target for SpacetimeDB modules |

### Key Rust Libraries

| Library | Purpose |
|---------|---------|
| `glam` | 3D mathematics (vectors, transforms) |
| `probability` | Random number generation and distributions |
| `strum` | Enum utilities and string conversions |
| `csv` | Static data parsing |
| `regex` | Text validation and parsing |
| `queues` | Efficient queue data structures |

### Development Tools

- **SpacetimeDB CLI** - Build, publish, and manage modules
- **Cargo** - Rust package manager and build tool
- **rustfmt** - Code formatting (configured via `rustfmt.toml`)

## Development Workflow

### Building

```bash
# Build game module
cd BitCraftServer/packages/game
spacetime build

# Build global module
cd BitCraftServer/packages/global_module
spacetime build
```

### Local Testing

```bash
# Start local SpacetimeDB instance
spacetime start

# Publish module locally
spacetime publish bitcraft-test --project-path ./packages/game

# View logs
spacetime logs bitcraft-test
```

### Configuration

Edit configuration files in `packages/game/config/`:
- Start with `local.example.json`
- Copy to `local.json` and customize
- Set environment-specific values

### Static Data Import

1. Prepare CSV files with game data
2. Use `stage_static_data` reducer to import
3. Data loaded into staging tables
4. Activate with appropriate admin commands

### Code Organization Best Practices

**Handlers** (Reducers):
- Group related reducers in handler files
- One reducer per logical action
- Keep reducers focused and small
- Use helper functions for complex logic

**Tables**:
- Define in `messages/` directory
- Use descriptive names ending in `State` or `Desc`
- Mark `public` if clients need to read
- Add indexes for frequently queried fields

**Agents**:
- Keep agent logic lightweight
- Batch operations when possible
- Use appropriate scheduling intervals
- Handle errors gracefully (agents shouldn't panic)

## Next Steps

- **[Architecture](architecture.md)** - Deep dive into system architecture and design patterns
- **[Data Models](data-models.md)** - Complete database schema reference
- **[Reducers API](reducers-api.md)** - Full reducer documentation
- **[Game Systems](game-systems.md)** - Game mechanics and algorithms
- **[Deployment](deployment.md)** - Production deployment guide
