# BitCraft Game Systems

## Table of Contents

- [Player System](#player-system)
- [Movement and Navigation](#movement-and-navigation)
- [Inventory System](#inventory-system)
- [Crafting System](#crafting-system)
- [Building System](#building-system)
- [Claim System](#claim-system)
- [Empire System](#empire-system)
- [Combat System](#combat-system)
- [Resource System](#resource-system)
- [World Generation](#world-generation)
- [NPC and Enemy AI](#npc-and-enemy-ai)
- [Progression System](#progression-system)

## Player System

### Overview

The player system manages player lifecycle, session state, and core player attributes.

### Sign In/Out Flow

**Sign In Process**:
1. Client calls `sign_in` reducer
2. System validates user exists and can sign in (queue check)
3. Check if player entity exists, create if first time
4. Load player state from database
5. Set `signed_in = true`
6. Add to `SignedInPlayerState` table
7. Schedule auto-logout agent
8. Trigger discovery sync
9. Return success to client

**Sign Out Process**:
1. Client calls `sign_out` or disconnect triggers grace period
2. Update total play time statistics
3. Set `signed_in = false`
4. Remove from `SignedInPlayerState`
5. Clean up temporary state
6. Persist data changes

**Grace Period**:
- After disconnect, player has grace period to reconnect
- Prevents queue re-entry on accidental disconnects
- Grace period configurable (typically 60-120 seconds)
- If reconnect within grace period, resume session without sign-in

### Queue System

**Purpose**: Manage server capacity and prevent overload

**Implementation**:
- `UserState.can_sign_in` flag controls access
- `UserState.queue_position` tracks position
- Admin role bypasses queue
- SkipQueue role bypasses queue

**Queue Management**:
- Admin reducers can modify queue state
- Population monitoring agent tracks player count
- Dynamic queue enabling based on server load

### Death and Respawn

**Death Triggers**:
- Health reaches 0 from combat
- Environmental damage (starvation, etc.)
- Fall damage (if enabled)

**Death Effects**:
1. Set `is_incapacitated = true`
2. Create dropped inventory with death flag
3. Increment `death_count`
4. Record `time_since_last_death_in_ms`
5. Disable movement and actions
6. Show respawn UI to client

**Respawn Options**:
1. **Home Location**: Teleport to set home
2. **Waystone**: Teleport to discovered waystone
3. **Birth Location**: Default spawn point

**Death Penalties**:
- Drop items (configurable percentage)
- Experience penalty (configurable)
- Temporary debuff after respawn

### Incapacitation

**Temporary State**: Player is down but can be revived

**Mechanics**:
- Health at 0 but not dead yet
- Countdown timer for death
- Other players can revive
- Self-revive with items

## Movement and Navigation

### Hexagonal Grid Movement

**Coordinate System**: BitCraft uses axial hex coordinates (q, r, s)

**Movement Validation**:
1. Calculate distance from current to destination
2. Check maximum movement range (based on stamina)
3. Validate terrain traversability at destination
4. Check for obstacles (buildings, resources)
5. Validate claim permissions if entering claimed land
6. Check for debuffs affecting movement

**Movement Costs**:
```rust
base_stamina_cost = distance * base_cost_per_tile
if is_running {
    stamina_cost = base_stamina_cost * running_multiplier // typically 2x
}
if terrain_type == Water {
    stamina_cost *= water_penalty // typically 1.5x
}
if elevation_change > threshold {
    stamina_cost *= climbing_penalty // typically 2x
}
```

### Pathfinding

**Algorithm**: A* pathfinding on hex grid

**Cost Function**:
- Base cost: 1.0 per tile
- Terrain multipliers:
  - Water: 1.5x
  - Steep elevation: 2.0x
  - Obstacles: Infinite (impassable)
- Claim boundaries: Requires permission check

**Path Caching**:
- NPCs cache paths for efficiency
- Paths invalidated on terrain/building changes
- Maximum cache size per NPC

### Exploration and Discovery

**Fog of War System**:

**Chunk-Based Discovery**:
- World divided into exploration chunks
- Player has `explored_chunks: Vec<u64>`
- Discovering new chunk reveals entities within
- Permanent discovery (persists across sessions)

**Entity Discovery**:
```rust
pub fn discover_entities(ctx: &ReducerContext, entity_ids: Vec<u64>) -> Result<(), String> {
    for entity_id in entity_ids {
        // Determine entity type
        let entity_type = get_entity_type(ctx, entity_id);

        // Update appropriate knowledge table
        match entity_type {
            EntityType::Item => update_item_knowledge(ctx, entity_id, KnowledgeState::Discovered),
            EntityType::Building => update_building_knowledge(ctx, entity_id, KnowledgeState::Discovered),
            // ... other types
        }

        // Grant discovery experience
        grant_discovery_experience(ctx, entity_type);
    }
}
```

**Discovery Ranges**:
- Buildings: 50 tiles
- Resources: 30 tiles
- Enemies: 40 tiles
- Items: 10 tiles

### Teleportation

**Types of Teleportation**:

1. **Home Teleport**:
   - Cost: Teleport energy (regenerates over time)
   - Cooldown: Configurable (e.g., 5 minutes)
   - Blocked in combat

2. **Waystone Network**:
   - Requires waystone discovery
   - Cost: Energy + currency
   - Instant travel between discovered waystones
   - Must be at a waystone to teleport

3. **Portal System**:
   - Building-based teleportation
   - Can link dimensions
   - Configurable destinations
   - Can be private or public

**Teleportation Energy**:
- Max energy: Configurable per player
- Regen rate: Configurable (e.g., 1 energy per 60 seconds)
- Costs:
  - Home teleport: 50 energy
  - Waystone: 30 energy + currency
  - Portal: 0 energy (building provides)

## Inventory System

### Multi-Pocket Inventory

**Structure**:
```rust
pub struct InventoryState {
    pub pockets: Vec<Pocket>,
    pub cargo_index: i32,      // Which pocket is cargo
    pub inventory_index: i32,  // Which pocket is main inventory
}

pub struct Pocket {
    pub item_stacks: Vec<ItemStack>,
    pub volume: i32,
    pub locked: bool,
}
```

**Pocket Types**:
1. **Main Inventory**: General storage
2. **Cargo**: Large volume, slower access
3. **Quick Slots**: Fast-access hotbar
4. **Equipment**: Worn items
5. **Crafting**: Recipe materials

### Item Stacking

**Stacking Rules**:
- Items stack if `item_id` matches
- Max stack size defined in `ItemDesc.max_stack_size`
- Damaged tools don't stack
- Unique items (durability) don't stack

**Stack Operations**:
- **Merge**: Combine partial stacks
- **Split**: Divide stack into two
- **Move**: Transfer between pockets/inventories
- **Swap**: Exchange positions

### Volume System

**Volume Mechanics**:
- Each item has `ItemDesc.volume`
- Each pocket has `Pocket.volume` capacity
- Stack volume = `item.volume * quantity`
- Total pocket volume = sum of all stack volumes
- Movement fails if destination exceeds volume

**Volume Expansion**:
- Unlock additional pockets via progression
- Increase pocket volume with upgrades
- Cargo pockets have large volume

### Item Durability

**Durability System**:
```rust
pub struct ItemStack {
    pub item_id: i32,
    pub quantity: i32,
    pub durability: Option<f32>,  // For tools/equipment
}
```

**Durability Loss**:
- Tools lose durability on use
- Equipment loses durability in combat
- Repair at buildings with materials
- Broken items remain in inventory (0 durability)

**Repair**:
- Requires repair materials (fraction of crafting cost)
- Specialized buildings (forges, workshops)
- NPC repair services

### Dropped Items

**Drop Mechanics**:
1. Player drops item → Create `DroppedInventoryState` entity
2. Place at player location
3. Set protection timer (30-60 seconds)
4. During protection, only dropper can pick up
5. After protection, anyone can pick up
6. Despawn timer (5-10 minutes)

**Death Drops**:
- Special category (`is_death_drop = true`)
- Longer protection timer
- Lost items recovery system
- May have partial item loss

## Crafting System

### Active Crafting

**Progressive Action System**:

**Flow**:
1. Player calls `craft_initiate(recipe_id, building_id, quantity)`
2. Validate requirements (materials, tool, building, skill)
3. Consume materials (if config enables)
4. Create `ProgressiveActionState` entity
5. Set player action to `Craft`
6. Client shows crafting progress bar
7. Timed action completes → produce items
8. Call `craft_collect` to receive items

**Timing Windows**:
```rust
base_time = recipe.craft_time_ms
skill_modifier = player_skill_level / 100.0  // e.g., level 50 = 0.5x reduction
building_modifier = building.craft_speed_bonus  // e.g., 1.2x faster

actual_time = base_time * (1.0 - skill_modifier) * building_modifier
```

**Batch Crafting**:
- Request multiple items in single operation
- Craft items sequentially
- Can suspend/resume
- Track `items_completed` and `items_requested`

**Suspend/Resume**:
- Player can suspend to move/fight
- `ProgressiveActionState.status = Suspended`
- Resume at same building later
- Preserves progress and materials

### Passive Crafting

**Queue-Based System**:

**Mechanics**:
1. Player queues craft at building
2. Materials consumed immediately
3. Crafting proceeds in background
4. No player presence required
5. Collect finished items later

**Queue Processing**:
```rust
// Passive craft agent
pub fn passive_craft_process_agent(ctx: &ReducerContext, arg: PassiveCraftProcessSchedule) {
    for craft in ctx.db.passive_craft_state().iter() {
        let elapsed = current_time - craft.start_timestamp;
        let time_per_item = recipe.craft_time_ms;

        let items_completed = (elapsed / time_per_item) as i32;

        if items_completed >= craft.items_requested {
            // Mark as complete, ready to collect
            mark_craft_complete(ctx, craft.id);
        } else {
            // Update progress
            update_craft_progress(ctx, craft.id, items_completed);
        }
    }

    // Reschedule agent
    schedule_passive_craft_process_agent(ctx, 10000); // Every 10 seconds
}
```

**Building Queues**:
- Each building has independent queue
- Queue position per player
- First-in-first-out processing
- Public buildings can have shared queues

### Recipe System

**Recipe Types**:

1. **Crafting**: Produce items at buildings/by hand
2. **Construction**: Build structures
3. **Extraction**: Harvest resources
4. **Item Conversion**: Transform items instantly
5. **Terraform**: Modify terrain
6. **Growth**: Plant/grow resources

**Recipe Requirements**:
```rust
pub struct CraftingRecipeDesc {
    pub inputs: Vec<ItemStack>,               // Materials needed
    pub outputs: Vec<ItemStack>,              // Items produced
    pub tool_requirement: Option<i32>,        // Required equipped tool
    pub building_requirement: Option<i32>,    // Required building
    pub skill_requirement: Option<SkillRequirement>,  // Skill level
    pub tech_requirement: Option<i32>,        // Tech unlock
    pub craft_time_ms: u64,                   // Base time to craft
}
```

**Recipe Discovery**:
- Start with basic recipes
- Unlock via:
  - Skill progression
  - Tech research
  - Recipe discovery (find in world)
  - Quest rewards

### Crafting Experience

**Experience Grants**:
```rust
fn grant_crafting_experience(ctx: &ReducerContext, player_id: u64, recipe_id: i32) {
    let recipe = get_recipe(ctx, recipe_id);
    let skill_id = recipe.primary_skill;

    // Experience based on recipe complexity
    let base_exp = recipe.experience_value;

    // Bonus for first-time craft
    let first_time_bonus = if is_first_craft(ctx, player_id, recipe_id) {
        base_exp * 0.5
    } else {
        0
    };

    let total_exp = base_exp + first_time_bonus;

    add_experience(ctx, player_id, skill_id, total_exp);
}
```

## Building System

### Construction Process

**Three-Stage Construction**:

**Stage 1: Project Site Placement**
```rust
project_site_place(building_id, location, direction)
```
1. Validate building type and player knowledge
2. Check terrain compatibility
3. Validate footprint (all tiles valid)
4. Check for overlapping entities
5. Verify claim permissions
6. Create `ProjectSiteState` entity
7. Reserve footprint tiles

**Stage 2: Material Contribution**
```rust
project_site_add_materials(project_site_id)
```
1. Check what materials are still needed
2. Transfer materials from player inventory
3. Update `materials_contributed`
4. Calculate `construction_progress` percentage
5. Allow multiple players to contribute (if public)

**Stage 3: Construction Work**
```rust
project_site_advance_project(project_site_id)
```
1. Verify all materials present
2. Start progressive action (timed construction)
3. On completion:
   - Delete `ProjectSiteState`
   - Create `BuildingState` entity
   - Create all building components (inventory, functions, etc.)
   - Grant building experience

### Building Functions

**Multi-Function Buildings**:

Buildings can have multiple functions:
```rust
pub enum BuildingFunctionDesc {
    Storage { volume: i32 },
    Crafting { recipes: Vec<i32> },
    Teleportation,
    Bank { vault_tabs: i32 },
    Spawn { resource_id: i32, rate_ms: u64 },
    Portal { destination: PortalLocationDesc },
    Waystone,
    Marketplace,
    Housing { capacity: i32 },
    // ... 20+ function types
}
```

**Function Examples**:

**Storage Function**:
- Building has `InventoryState` component
- Configurable volume
- Permission-based access
- Can link to storage networks

**Crafting Function**:
- Unlocks specific recipes
- May provide speed bonuses
- Can be public or private access
- Supports passive crafting queues

**Spawn Function**:
- Periodically generates resources
- Resource type and rate configurable
- Outputs to building inventory
- Requires building health > 0

**Portal Function**:
- Links to specific dimension and location
- Can be bidirectional
- Permission-controlled
- Supports interior instances

### Building Maintenance and Decay

**Supply-Based Upkeep**:

**Mechanics**:
```rust
// Building decay agent runs periodically
pub fn building_decay_agent(ctx: &ReducerContext, arg: BuildingDecaySchedule) {
    for claim in ctx.db.claim_state().iter() {
        // Calculate supply decay rate
        let decay_rate = calculate_decay_rate(ctx, claim.entity_id);

        // Deduct supplies
        claim.supplies -= decay_rate;

        if claim.supplies <= 0 {
            // Buildings start taking damage
            damage_claim_buildings(ctx, claim.entity_id);
        }

        // Update claim
        ctx.db.claim_state().entity_id().update(claim);
    }

    // Reschedule
    schedule_building_decay_agent(ctx, 3600000); // Every hour
}
```

**Decay Formula**:
```rust
total_maintenance = sum(building.maintenance for all buildings in claim)
decay_rate = total_maintenance / decay_interval  // e.g., per hour
```

**Resupply**:
- Players contribute items to claim supplies
- Items converted to supply points
- Supply conversion rates in `ParametersDesc`
- Prevents decay when supplies > 0

**Building Damage**:
- When supplies depleted, buildings lose health
- Damage rate configurable
- At 0 health: Building becomes "ruin"
- Ruined buildings lose functionality
- Can be repaired with materials

### Building Networks

**Storage Networks**:

**Purpose**: Link multiple storage buildings for shared inventory

**Implementation**:
```rust
pub struct StorageNetworkState {
    pub entity_id: u64,
    pub building_entity_ids: Vec<u64>,
    pub total_volume: i32,
}
```

**Mechanics**:
- Buildings in same claim can join network
- Shared inventory across all buildings
- Total volume = sum of individual volumes
- Access any networked storage from any building

## Claim System

### Claim Ownership

**Claim Creation**:
1. Player builds claimable building (e.g., Totem)
2. Call `claim_take_ownership(building_id)`
3. System creates `ClaimState` entity
4. Adds building's tile to claim territory
5. Player becomes owner

**Claim Types**:
- **Personal**: Individual player claim
- **Clan**: Small group claim
- **Settlement**: Large community claim
- **Outpost**: Empire-linked claim

### Territory Management

**Territory Expansion**:

**Adding Tiles**:
```rust
claim_add_tile(claim_id, location)
```

**Requirements**:
- Tile must be adjacent to current territory
- Tile not claimed by others
- Sufficient claim supplies
- Player has `add_tile` permission

**Cost**:
```rust
expansion_cost = base_cost + (current_tile_count * scaling_factor)
```

**Removing Tiles**:
- Can only remove edges (maintain connectivity)
- Cannot remove tiles with buildings
- Refunds partial supplies

### Permission System

**Granular Permissions**:
```rust
pub struct ClaimPermissions {
    pub inventory: bool,        // Access claim storage
    pub build: bool,            // Place/remove buildings
    pub usage: bool,            // Use building functions
    pub recruit: bool,          // Invite members
    pub remove_member: bool,    // Kick members
    pub edit_permissions: bool, // Change member permissions
    pub add_tile: bool,         // Expand territory
    pub remove_tile: bool,      // Shrink territory
}
```

**Permission Checks**:
```rust
fn can_access_building(ctx: &ReducerContext, building_id: u64, player_id: u64) -> bool {
    let building = get_building(ctx, building_id);
    let claim_id = building.claim_entity_id;

    // Owner has all permissions
    if is_claim_owner(ctx, claim_id, player_id) {
        return true;
    }

    // Check member permissions
    if let Some(member) = get_claim_member(ctx, claim_id, player_id) {
        return member.permissions.usage;
    }

    false
}
```

### Claim Technology

**Tech Tree System**:

**Tech Unlocks**:
- New building types
- Advanced recipes
- Territory expansion limits
- Empire formation

**Research Process**:
1. Select tech to research
2. Contribute research materials
3. Accumulate research points
4. Tech unlocks when threshold reached

**Tech Requirements**:
- Prerequisites (other techs)
- Resources
- Claim level/size
- Time investment

### Claim Supplies

**Supply System**:

**Sources**:
- Player contributions (items → supplies)
- Claim income buildings
- Empire support
- Quest rewards

**Uses**:
- Territory expansion
- Building maintenance
- Tech research
- Defense/protection

**Protection Threshold**:
```rust
if claim.supplies >= claim.protection_threshold {
    // Claim is fully protected
    // Buildings can't be damaged by enemies
    // Territory secure
} else {
    // Reduced protection
    // Vulnerable to decay and raids
}
```

## Empire System

### Empire Formation

**Requirements**:
- Claim must have unlocked empire tech
- Pay formation cost (shards + resources)
- Have qualifying settlement (size/buildings)
- Choose unique empire name

**Formation Process**:
1. Call `empire_form(name, settlement_id, emblem)`
2. Create `EmpireState` on global server
3. Set founder as emperor
4. Add settlement to empire
5. Create default rank structure
6. Replicate to all regions

### Rank System

**Rank Hierarchy**:

**Default Ranks**:
1. **Emperor** (rank 0): Highest authority
2. **Council** (rank 1): Senior leadership
3. **Officer** (rank 2): Management
4. **Member** (rank 3): Regular members
5. **Recruit** (rank 4): New members

**Custom Ranks**:
- Empire can create custom ranks
- Set rank titles
- Configure rank permissions
- Assign members to ranks

**Rank Permissions**:
```rust
pub struct EmpirePermissions {
    pub invite: bool,              // Invite new members
    pub expel: bool,               // Remove members
    pub manage_ranks: bool,        // Create/modify ranks
    pub manage_settlements: bool,  // Add/remove settlements
    pub declare_war: bool,         // Start conflicts
    pub manage_treasury: bool,     // Access empire funds
    pub mark_expansion: bool,      // Mark territory for expansion
    pub start_siege: bool,         // Initiate sieges
    pub manage_diplomacy: bool,    // Set alliances
    pub modify_tax_rate: bool,     // Set member tax
    pub access_foundry: bool,      // Use empire foundry
    pub manage_officers: bool,     // Promote/demote below own rank
    pub broadcast: bool,           // Empire announcements
    pub modify_emblem: bool,       // Change empire emblem
}
```

### Territory and Expansion

**Empire Territory**:

**Territory Types**:
1. **Settlement Territory**: Land owned by empire settlements
2. **Expansion Territory**: Marked for future conquest
3. **Contested Territory**: Under siege

**Expansion Process**:
1. Officer marks territory for expansion
2. Empire members contribute resources
3. Once funded, expansion activates
4. Territory becomes empire-controlled

**Territory Benefits**:
- Resource bonuses in empire land
- Building bonuses
- Fast travel between settlements
- Shared storage/resources

### Siege System

**PvP Territory Conquest**:

**Siege Initiation**:
1. Declare target settlement
2. Mark settlement for siege (requires resources)
3. Siege preparation period (defenders can prepare)
4. Siege begins

**Siege Mechanics**:
```rust
pub struct EmpireSiegeState {
    pub attacker_supplies: f32,    // Attacker siege supplies
    pub defender_supplies: f32,    // Defender defense supplies
    pub siege_start_timestamp: u64,
    pub siege_end_timestamp: u64,
    pub status: SiegeStatus,
}
```

**Siege Progress**:
- Both sides contribute supplies
- Siege resolves after time period
- Winner = side with more supplies
- Victory grants territory control

**Siege Outcomes**:
- **Attacker Victory**: Settlement transfers to attacking empire
- **Defender Victory**: Settlement remains, attackers lose resources
- **Stalemate**: Settlement remains, both sides lose resources

### Empire Economy

**Empire Treasury**:
- Centralized empire funds
- Member contributions (taxes, donations)
- Territory income
- Siege rewards

**Foundry System**:
- Empire-level crafting
- Hexite capsule production
- Requires specific buildings
- Produces empire-exclusive items

**Supply Management**:
- Empire-wide supply pool
- Supports settlements
- Funds expansions and sieges
- Distributed to needy settlements

## Combat System

### Damage Calculation

**Base Damage Formula**:
```rust
fn calculate_damage(
    attacker: &CombatStats,
    defender: &CombatStats,
    combat_action: &CombatActionDesc,
) -> f32 {
    // Base damage from action
    let base_damage = combat_action.damage;

    // Weapon multiplier
    let weapon_multiplier = attacker.weapon_damage_multiplier;

    // Stat multipliers (strength, etc.)
    let stat_multiplier = attacker.get_damage_stat_multiplier();

    // Pre-mitigation damage
    let pre_mitigation = base_damage * weapon_multiplier * stat_multiplier;

    // Armor reduction
    let armor_reduction = defender.armor / (defender.armor + 100.0);
    let post_armor = pre_mitigation * (1.0 - armor_reduction);

    // Critical hit check
    if random() < attacker.critical_chance {
        post_armor * attacker.critical_multiplier
    } else {
        post_armor
    }
}
```

### Threat System

**Enemy Aggro**:

**Threat Generation**:
- Damage dealt: `threat += damage * threat_multiplier`
- Healing allies: `threat += healing * 0.5`
- Proximity: `threat += proximity_threat_per_second`
- First strike: `threat += initial_aggro`

**Threat Decay**:
```rust
fn update_threat(ctx: &ReducerContext) {
    for threat in ctx.db.threat_state().iter() {
        let time_since_update = current_time - threat.last_update_timestamp;

        // Decay over time
        threat.threat_level *= 0.99f32.powf(time_since_update / 1000.0);

        // If out of range, decay faster
        if distance_to_player(threat.enemy_id, threat.player_id) > aggro_range {
            threat.threat_level *= 0.5;
        }

        // Remove if too low
        if threat.threat_level < 1.0 {
            delete_threat(ctx, threat.id);
        } else {
            update_threat_entry(ctx, threat);
        }
    }
}
```

**Target Selection**:
- Enemy targets highest threat player within range
- If current target out of range, switch to next highest
- New players start with 0 threat

### Combat Actions

**Action Types**:
- **Basic Attack**: Default weapon attack
- **Abilities**: Special attacks with cooldowns
- **Combo Attacks**: Multi-hit sequences
- **Dodge**: Evasion move

**Cooldown System**:
```rust
pub struct CombatActionState {
    pub player_entity_id: u64,
    pub action_id: i32,
    pub last_use_timestamp: u64,
}

fn is_on_cooldown(ctx: &ReducerContext, player_id: u64, action_id: i32) -> bool {
    if let Some(state) = get_action_state(ctx, player_id, action_id) {
        let action = get_combat_action(ctx, action_id);
        let elapsed = current_time(ctx) - state.last_use_timestamp;
        elapsed < action.cooldown_ms
    } else {
        false
    }
}
```

### Buff and Debuff System

**Status Effects**:
```rust
pub struct BuffState {
    pub entity_id: u64,
    pub buff_id: i32,
    pub stacks: i32,
    pub applied_timestamp: u64,
    pub expires_at: u64,
}
```

**Buff Types**:
- **Stat Modifiers**: +damage, +armor, +speed
- **Damage Over Time**: Poison, bleed
- **Healing Over Time**: Regeneration
- **Crowd Control**: Stun, slow, root
- **Immunity**: Shield, invulnerability

**Stack Mechanics**:
- Some buffs stack (multiple applications)
- Stack limit per buff type
- Refreshing duration vs adding stacks

### Auto-Attack

**Auto-Attack Flow**:
1. Player enables auto-attack on target
2. System schedules periodic attacks
3. Each attack:
   - Check target still valid and in range
   - Execute attack with weapon
   - Schedule next attack based on weapon speed
4. Continues until:
   - Target dies
   - Player cancels
   - Target out of range
   - Player starts different action

**Attack Speed**:
```rust
attack_interval_ms = weapon.base_attack_speed * (1.0 / attack_speed_multiplier)
```

## Resource System

### Resource Spawning

**Initial Placement**:
- During world generation
- Biome-specific resource types
- Density maps determine spawn rates
- Noise-based natural distribution

**Resource Regeneration**:

**Agent-Based Respawning**:
```rust
pub fn resources_regen_agent(ctx: &ReducerContext, arg: ResourcesRegenSchedule) {
    // For each resource type
    for resource_type in all_resource_types() {
        let current_count = count_resources(ctx, resource_type);
        let target_count = get_target_density(ctx, resource_type);

        if current_count < target_count {
            let to_spawn = target_count - current_count;

            // Find valid spawn locations
            let spawn_locations = find_spawn_locations(
                ctx,
                resource_type,
                to_spawn
            );

            // Spawn resources
            for location in spawn_locations {
                spawn_resource(ctx, resource_type, location);
            }
        }
    }

    // Reschedule for next run
    schedule_resources_regen_agent(ctx, 300000); // Every 5 minutes
}
```

**Spawn Location Selection**:
1. Filter by biome (resource must match biome)
2. Check terrain type (ground, water depth)
3. Avoid overlap with buildings/claims
4. Respect chunk limits (max resources per chunk)
5. Use noise function for natural clustering

### Resource Extraction

**Harvesting Process**:
1. Player initiates extract action on resource
2. Validate tool requirement
3. Start progressive action (timed)
4. On completion:
   - Deal damage to resource health
   - Grant extracted items
   - Grant extraction experience
5. If resource health reaches 0:
   - Resource depletes
   - Schedule respawn

**Tool Requirements**:
- Different resources require different tools
- Tool level affects extraction speed
- Higher level tools extract more efficiently

**Resource Health**:
- Resources have health pool
- Each extraction reduces health
- Multiple extractions before depletion
- Health regenerates slowly or on respawn

### Resource Growth

**Planted Resources**:

Some resources can be planted and grown:
1. Use growth recipe (plant seed)
2. Resource enters growth state
3. Growth agent processes over time
4. Matures into harvestable resource

**Growth Mechanics**:
```rust
pub fn growth_agent(ctx: &ReducerContext, arg: GrowthSchedule) {
    for growing_resource in ctx.db.growing_resource_state().iter() {
        let elapsed = current_time - growing_resource.plant_timestamp;
        let growth_time = get_growth_time(growing_resource.resource_id);

        if elapsed >= growth_time {
            // Convert to mature resource
            complete_growth(ctx, growing_resource);
        } else {
            // Update growth progress
            let progress = elapsed as f32 / growth_time as f32;
            update_growth_progress(ctx, growing_resource.id, progress);
        }
    }

    schedule_growth_agent(ctx, 60000); // Check every minute
}
```

**Growth Factors**:
- Base growth time
- Biome suitability
- Nearby buildings (fertilizer, etc.)
- Player care actions

## World Generation

### Procedural Generation

**Generation Pipeline**:

**Phase 1: Noise Maps**
```rust
pub fn generate_noise_maps(seed: u64, size: u32) -> NoiseMaps {
    NoiseMaps {
        elevation: generate_perlin_noise(seed, size, octaves: 6),
        moisture: generate_perlin_noise(seed + 1, size, octaves: 4),
        temperature: generate_perlin_noise(seed + 2, size, octaves: 4),
        biome_variation: generate_perlin_noise(seed + 3, size, octaves: 3),
    }
}
```

**Phase 2: Water Level Calculation**
```rust
pub fn calculate_water_levels(elevation_map: &NoiseMap) -> WaterMap {
    let sea_level = calculate_sea_level(elevation_map);

    for tile in all_tiles() {
        if tile.elevation < sea_level {
            tile.water_level = sea_level - tile.elevation;
            tile.is_water = true;
        }
    }

    generate_rivers(&mut water_map);
    generate_lakes(&mut water_map);

    water_map
}
```

**Phase 3: Biome Assignment**
```rust
pub fn assign_biomes(
    elevation_map: &NoiseMap,
    moisture_map: &NoiseMap,
    temperature_map: &NoiseMap,
) -> BiomeMap {
    for tile in all_tiles() {
        let elevation = elevation_map[tile];
        let moisture = moisture_map[tile];
        let temperature = temperature_map[tile];

        tile.biome = match (elevation, moisture, temperature) {
            (e, _, _) if e < sea_level => Biome::Ocean,
            (e, m, t) if e > high && t < cold => Biome::SnowyPeaks,
            (e, m, t) if m > wet && e < mid => Biome::Swamp,
            (e, m, t) if m < dry && t > hot => Biome::Desert,
            // ... 14 biome rules
            _ => Biome::BreezyPlains,
        };
    }

    biome_map
}
```

**Phase 4: Feature Placement**
1. **Player Spawns**: Safe, flat areas near water
2. **Building Placement**: Strategic locations, biome-appropriate
3. **Resource Distribution**: Density maps per biome
4. **Enemy Spawns**: Herd locations

### River Generation

**River Algorithm**:
```rust
pub fn generate_rivers(water_map: &mut WaterMap, elevation_map: &ElevationMap) {
    // Find high elevation source points
    let sources = find_river_sources(elevation_map);

    for source in sources {
        let mut current = source;
        let mut path = vec![current];

        // Flow downhill until reaching ocean or lake
        loop {
            let neighbors = get_neighbors(current);
            let lowest = neighbors.iter()
                .min_by(|a, b| elevation_map[a].cmp(&elevation_map[b]))
                .unwrap();

            if elevation_map[lowest] >= elevation_map[current] {
                // Create lake
                create_lake(water_map, current);
                break;
            }

            if elevation_map[lowest] < sea_level {
                // Reached ocean
                break;
            }

            current = *lowest;
            path.push(current);
        }

        // Mark river tiles
        for tile in path {
            water_map[tile].is_river = true;
            water_map[tile].water_level = river_depth;
        }
    }
}
```

### Chunk System

**Chunk Upload**:

World generation happens offline, chunks uploaded via:
```rust
pub fn insert_terrain_chunk(
    ctx: &ReducerContext,
    chunk_index: u64,
    terrain_nodes: Vec<TerrainNode>,
) -> Result<(), String>
```

**Terrain Node**:
```rust
pub struct TerrainNode {
    pub elevation: f32,
    pub water_level: f32,
    pub biome: Biome,
    pub is_passable: bool,
}
```

**Chunk Loading**:
- Clients subscribe to chunks in view range
- Server sends chunk data on subscription
- Lazy loading reduces initial payload

## NPC and Enemy AI

### Enemy AI States

**State Machine**:
```rust
pub enum EnemyAIState {
    Idle,        // Standing still, scanning for targets
    Wandering,   // Roaming within herd radius
    Fighting,    // Engaged in combat
    Fleeing,     // Running from high-level threats
    Retreating,  // Returning to home location
    Dead,        // Waiting for respawn
}
```

**State Transitions**:
- **Idle → Wandering**: Random timer triggers
- **Idle → Fighting**: Player within aggro range
- **Fighting → Retreating**: Target out of range or leash reached
- **Fighting → Fleeing**: Health below threshold and outmatched
- **Retreating → Idle**: Back at home location, no threats

### Herd Behavior

**Herd System**:
```rust
pub struct HerdState {
    pub herd_leader_entity_id: u64,
    pub member_entity_ids: Vec<u64>,
    pub center_location: SmallHexTile,
    pub wander_radius: f32,
}
```

**Herd Mechanics**:
- Leader designated (typically strongest enemy)
- Members follow leader during wander
- Members stay within radius of center
- Combat behavior coordinated (assist allies)
- Herd respawns together

**AI Agent**:
```rust
pub fn npc_ai_agent(ctx: &ReducerContext, arg: NpcAiSchedule) {
    for enemy in ctx.db.enemy_state().iter().filter(|e| e.ai_state != Dead) {
        match enemy.ai_state {
            Idle => process_idle_ai(ctx, enemy),
            Wandering => process_wander_ai(ctx, enemy),
            Fighting => process_combat_ai(ctx, enemy),
            Fleeing => process_flee_ai(ctx, enemy),
            Retreating => process_retreat_ai(ctx, enemy),
            Dead => {}, // Handled by regen agent
        }
    }

    schedule_npc_ai_agent(ctx, 2000); // Every 2 seconds
}
```

### Enemy Scaling

**Level-Based Scaling**:
```rust
fn calculate_enemy_stats(base_stats: &EnemyDesc, level: i32) -> CombatStats {
    let level_mult = 1.0 + (level as f32 * 0.1); // 10% per level

    CombatStats {
        health: base_stats.base_health * level_mult,
        damage: base_stats.base_damage * level_mult,
        armor: base_stats.base_armor * level_mult,
        // ... other stats
    }
}
```

**Area-Based Scaling**:
- Enemies near spawn points: Low level
- Enemies in dangerous biomes: Higher level
- Dungeon/special area enemies: Scaled to area

### Loot System

**Loot Drops**:
```rust
fn generate_loot(ctx: &ReducerContext, enemy: &EnemyState) -> Vec<ItemStack> {
    let loot_table = get_loot_table(ctx, enemy.enemy_description_id);
    let mut drops = Vec::new();

    for entry in loot_table.entries {
        if random() < entry.drop_chance {
            let quantity = random_range(entry.min_quantity, entry.max_quantity);
            drops.push(ItemStack {
                item_id: entry.item_id,
                quantity,
            });
        }
    }

    // Bonus drops for high-level enemies
    if enemy.level > 10 {
        add_rare_loot(&mut drops, enemy.level);
    }

    drops
}
```

**Loot Rarity**:
- Common: High drop chance, low value
- Uncommon: Medium drop chance
- Rare: Low drop chance, valuable
- Legendary: Very low drop chance, unique items

## Progression System

### Experience and Leveling

**Skill-Based Experience**:

**Skill Types**:
- Combat: From fighting enemies
- Mining: From extracting ores
- Woodcutting: From harvesting trees
- Crafting: From crafting items
- Building: From construction
- Cooking: From food preparation
- Farming: From planting/harvesting
- Exploration: From discovering new areas

**Experience Formula**:
```rust
pub fn level_for_experience(experience: i32) -> i32 {
    // Quadratic formula: level = sqrt(xp / base)
    let base_xp_per_level = 100.0;
    (((experience as f32) / base_xp_per_level).sqrt()) as i32
}

pub fn experience_for_level(level: i32) -> i32 {
    let base_xp_per_level = 100.0;
    (level * level) as f32 * base_xp_per_level) as i32
}
```

**Level Benefits**:
- Unlock new recipes
- Improved crafting speed
- Better resource yields
- Stat bonuses
- Ability unlocks

### Achievement System

**Achievement Tracking**:
```rust
pub struct AchievementState {
    pub player_entity_id: u64,
    pub achievement_id: i32,
    pub progress: i32,
    pub completed: bool,
    pub completed_timestamp: u64,
}
```

**Achievement Types**:
- Craft X items
- Defeat X enemies
- Explore X chunks
- Build X buildings
- Reach level X in skill
- Complete quests
- Join empire
- Social achievements

**Rewards**:
- Experience bonuses
- Unique items/cosmetics
- Titles
- Skill unlocks

### Quest System

**Quest Structure**:
```rust
pub struct QuestState {
    pub quest_id: i32,
    pub status: QuestStatus,
    pub objectives_completed: Vec<bool>,
    pub accepted_timestamp: u64,
}

pub enum QuestStatus {
    Available,   // Can be accepted
    Active,      // In progress
    Completed,   // Finished, ready for reward
    Failed,      // Failed (time limit, etc.)
}
```

**Quest Types**:
- Story quests: Narrative progression
- Side quests: Optional content
- Repeatable quests: Daily/weekly tasks
- Tutorial quests: Onboarding
- Traveler tasks: NPC requests

**Quest Rewards**:
- Experience
- Items
- Currency
- Recipe unlocks
- Reputation

## Summary

BitCraft features **10+ major interconnected systems**:
1. Player lifecycle and session management
2. Movement on hexagonal grid with pathfinding
3. Multi-pocket inventory with volume/stacking
4. Active and passive crafting with progression
5. Three-stage building construction
6. Territory-based claim system with permissions
7. Multi-region empire system with siege warfare
8. Threat-based combat with buffs/debuffs
9. Procedurally generated world with biomes
10. Skill-based progression with achievements

These systems work together to create a comprehensive sandbox MMO experience.

## Next Steps

- **[Reducers API](reducers-api.md)** - API reference for all systems
- **[Data Models](data-models.md)** - Database schemas
- **[Architecture](architecture.md)** - Technical implementation
- **[Deployment](deployment.md)** - Running the server
