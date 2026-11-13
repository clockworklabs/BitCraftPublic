use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct Pocket {
    pub volume: i32,
    pub contents: Option<ItemStack>,
    pub locked: bool,
}

#[derive(Debug, SpacetimeType, Clone, PartialEq, Copy)]
#[repr(i32)]
pub enum ItemType {
    Item = 0,
    Cargo,
}

#[derive(Debug, SpacetimeType, Clone, PartialEq)]
pub struct ItemStack {
    pub item_id: i32,
    pub quantity: i32,
    pub item_type: ItemType,
    pub durability: Option<i32>,
}

#[derive(Debug, SpacetimeType, Clone, PartialEq)]
pub struct ItemListPossibility {
    pub probability: f32,
    pub items: Vec<ItemStack>,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct InputItemStack {
    pub item_id: i32,
    pub quantity: i32,
    pub item_type: ItemType,
    pub discovery_score: i32,
    pub consumption_chance: f32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct TradePocket {
    pub inventory_pocket_index: i32,
    pub inventory_index: i32,
    pub contents: ItemStack,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct ProbabilisticItemStack {
    pub item_stack: Option<ItemStack>,
    pub probability: f32,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct BuildingRequirement {
    pub building_type: i32,
    pub tier: i32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ToolRequirement {
    pub tool_type: i32,
    pub level: i32,
    pub power: i32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct LevelRequirement {
    pub skill_id: i32,
    pub level: i32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct CappedLevelRequirement {
    pub skill_id: i32,
    pub min_level: i32,
    pub max_level: i32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ExperienceStack {
    pub skill_id: i32,
    pub quantity: i32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ExperienceStackF32 {
    pub skill_id: i32,
    pub quantity: f32,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ActiveBuff {
    pub buff_id: i32,
    pub buff_start_timestamp: OnlineTimestamp,
    pub buff_duration: i32,
    pub values: Vec<f32>,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct OnlineTimestamp {
    pub value: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[repr(i32)]
#[derive(Default)]
pub enum DimensionType {
    #[default]
    Unknown,
    Overworld,
    AncientRuin,
    BuildingInterior,
    Dungeon,
}


#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct PocketKey {
    pub inventory_entity_id: u64,
    pub pocket_index: i32,
}
