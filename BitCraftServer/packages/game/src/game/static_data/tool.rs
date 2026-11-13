use crate::messages::components::{InventoryState, PlayerState};
use spacetimedb::{log, ReducerContext, Table};

use crate::messages::game_util::ToolRequirement;
use crate::messages::static_data::*;

impl ToolDesc {
    pub fn get_required_tool(ctx: &ReducerContext, player_entity_id: u64, tool_requirement: &ToolRequirement) -> Result<ToolDesc, String> {
        let toolbelt_inventory = InventoryState::get_player_toolbelt(ctx, player_entity_id).unwrap();
        let toolbelt = &toolbelt_inventory.pockets;
        let toolbelt_pocket = ToolTypeDesc::get_slot_from_tool_type_id(ctx, tool_requirement.tool_type) as usize;

        if let Some(pocket) = toolbelt.get(toolbelt_pocket) {
            if let Some(contents) = pocket.contents {
                if let Some(equipment_info) = ctx.db.equipment_desc().item_id().find(contents.item_id) {
                    if let Some(level_req) = &equipment_info.level_requirement {
                        if PlayerState::meets_level_requirement(ctx, player_entity_id, level_req) {
                            let tool = ctx.db.tool_desc().item_id().filter(contents.item_id).next().unwrap();
                            if tool.level >= tool_requirement.level {
                                return Result::Ok(tool);
                            }
                        } else {
                            let tool_name = ctx.db.item_desc().id().find(contents.item_id).unwrap().name;
                            return Err(format!("You need a higher skill to use your {{0}}|~{}", tool_name));
                        }
                    }
                }
            }
        }

        let tool_name = ctx.db.tool_type_desc().id().find(tool_requirement.tool_type).unwrap().name;

        let article = if tool_name.chars().nth(0).unwrap() == 'A' { "an" } else { "a" };

        if tool_requirement.level <= 0 {
            return Result::Err(format!("You must have {{0}} {{1}} on your toolbelt|~{}|~{}", article, tool_name));
        }

        Err(format!(
            "You must have {{0}} {{1}} tier {{2}} or higher on your toolbelt|~{}|~{}|~{}",
            article, tool_name, tool_requirement.level
        ))
    }

    pub fn get_equipped_tool(ctx: &ReducerContext, player_entity_id: u64, tool_requirement: &ToolRequirement) -> Option<ToolDesc> {
        let toolbelt_inventory = InventoryState::get_player_toolbelt(ctx, player_entity_id).unwrap();
        let toolbelt = &toolbelt_inventory.pockets;
        let toolbelt_pocket = ToolTypeDesc::get_slot_from_tool_type_id(ctx, tool_requirement.tool_type) as usize;

        if let Some(pocket) = toolbelt.get(toolbelt_pocket) {
            if let Some(contents) = pocket.contents {
                if ctx.db.equipment_desc().item_id().find(contents.item_id).is_some() {
                    let tool = ctx.db.tool_desc().item_id().filter(contents.item_id).next().unwrap();
                    return Some(tool);
                }
            }
        }
        None
    }

    pub fn get_time_factor(ctx: &ReducerContext, tech_power: i32, desired_power: i32) -> f32 {
        // should be between range [-80, 80]
        let tech_power_delta = tech_power - desired_power;
        let max_tool_power = ctx.db.tool_desc().iter().map(|t| t.power).max().unwrap();
        let bound = max_tool_power - 1;
        if tech_power_delta.abs() > bound.abs() {
            log::warn!(
                "Delta out of bound [{},{}]. tech_power {} - desired_power: {}",
                -bound,
                bound,
                tech_power,
                desired_power
            );
            return 1.0;
        }

        // calculate time factor
        let log_f = ctx.db.parameters_desc().version().find(0).unwrap().tech_time_log_base.ln();
        // transform to float range of [0, 2]
        let modified_tech_power_delta = 1.0 + tech_power_delta as f32 / 100.0;
        let speed_factor = if tech_power_delta < 0 {
            modified_tech_power_delta.powf(ctx.db.parameters_desc().version().find(0).unwrap().tech_time_power_exponent)
        } else {
            modified_tech_power_delta.ln() / log_f + 1.0
        };
        1.0 / speed_factor
    }
}

impl ToolTypeDesc {
    pub fn get_combat_weapon_slot(ctx: &ReducerContext) -> i32 {
        // Using Slayer skill to recognize weapons from tool type even if it's not technically the ase
        ctx.db.tool_type_desc().skill_id().find(SkillType::Slayer as i32).unwrap().id - 1
    }

    pub fn get_slot_from_skill_id(ctx: &ReducerContext, skill_id: i32) -> i32 {
        // Using Slayer skill to recognize weapons from tool type even if it's not technically the ase
        ctx.db.tool_type_desc().skill_id().find(skill_id).unwrap().id - 1
    }

    pub fn get_slot_from_tool_type_id(ctx: &ReducerContext, tool_type_id: i32) -> i32 {
        ctx.db.tool_type_desc().id().find(tool_type_id).unwrap().id - 1
    }
}
