use std::collections::{BTreeSet, HashMap};

use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::{authentication::has_role, claim::claim_tech_unlock_tech::unlock_claim_tech},
    messages::{
        authentication::Role,
        components::*,
        static_data::{claim_tech_desc, ClaimTechType},
    },
};

#[spacetimedb::reducer]
pub fn migrate_claim_tech(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    crate::game::handlers::cheats::cheat_claim_totem::cheat_claims_complete_all_current_research(ctx)?;

    spacetimedb::log::info!("Migrating {} claim techs", ctx.db.claim_tech_state().count());
    for mut claim_tech in ctx.db.claim_tech_state().iter() {
        if claim_tech.learned.contains(&200) {
            //Tier 2
            unlock_claim_tech(ctx, &mut claim_tech, 748616905, false); //Town Bank
        }
        if claim_tech.learned.contains(&300) {
            //Tier 3
            unlock_claim_tech(ctx, &mut claim_tech, 1300921803, false); //Tier 3 Settlement
            unlock_claim_tech(ctx, &mut claim_tech, 611825682, false); //Large Houses
        }
        if claim_tech.learned.contains(&400) {
            //Tier 4
            unlock_claim_tech(ctx, &mut claim_tech, 1272742983, false); //Tier 4 Settlement
            unlock_claim_tech(ctx, &mut claim_tech, 2111626185, false); //Town Marketplace
        }
        if claim_tech.learned.contains(&500) {
            //Tier 5
            unlock_claim_tech(ctx, &mut claim_tech, 225112729, false); //Tier 5 Settlement
        }
        if claim_tech.learned.contains(&600) {
            //Tier 6
            unlock_claim_tech(ctx, &mut claim_tech, 1425977115, false); //Tier 6 Settlement
        }
        if claim_tech.learned.contains(&700) {
            //Tier 7
            unlock_claim_tech(ctx, &mut claim_tech, 315153795, false); //Tier 7 Settlement
        }
        if claim_tech.learned.contains(&800) {
            //Tier 8
            unlock_claim_tech(ctx, &mut claim_tech, 2029414118, false); //Tier 8 Settlement
        }
        if claim_tech.learned.contains(&900) {
            //Tier 9
            unlock_claim_tech(ctx, &mut claim_tech, 760073735, false); //Tier 9 Settlement
        }
        if claim_tech.learned.contains(&1000) {
            //Tier 10
            unlock_claim_tech(ctx, &mut claim_tech, 1091470477, false); //Tier 10 Settlement
        }

        ctx.db.claim_tech_state().entity_id().update(claim_tech);
    }
    spacetimedb::log::info!("Claim techs migrated");

    Ok(())
}

#[spacetimedb::reducer]
pub fn migrate_claim_tech_homestead_unlocks(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut homestead_tech_by_tier = HashMap::new();
    let mut profession_tier_by_tech = HashMap::new();
    for tech in ctx.db.claim_tech_desc().iter() {
        if tech.tech_type == ClaimTechType::Homestead {
            if let Some(existing_id) = homestead_tech_by_tier.insert(tech.tier, tech.id) {
                return Err(format!(
                    "Multiple homestead claim techs exist for tier {{0}}: {{1}} and {{2}}|~{}|~{}|~{}",
                    tech.tier, existing_id, tech.id
                ));
            }
        } else if is_profession_claim_tech(tech.tech_type) {
            profession_tier_by_tech.insert(tech.id, tech.tier);
        }
    }

    let mut updated_claims = 0;
    let mut unlocked_starter_claim_techs = 0;
    let mut unlocked_homestead_techs = 0;
    for mut claim_tech in ctx.db.claim_tech_state().iter() {
        let learned_profession_tiers: BTreeSet<i32> = claim_tech
            .learned
            .iter()
            .filter_map(|tech_id| profession_tier_by_tech.get(tech_id).copied())
            .collect();

        let learned_count_before = claim_tech.learned.len();

        if !learned_profession_tiers.is_empty() && !claim_tech.learned.contains(&254714033) {
            unlock_claim_tech(ctx, &mut claim_tech, 254714033, true); // Starter Claim
            unlocked_starter_claim_techs += 1;
        }

        for tier in learned_profession_tiers {
            let homestead_tech_id = homestead_tech_by_tier
                .get(&tier)
                .copied()
                .ok_or_else(|| format!("No homestead claim tech exists for tier {{0}}|~{tier}"))?;
            if claim_tech.learned.contains(&homestead_tech_id) {
                continue;
            }

            unlock_claim_tech(ctx, &mut claim_tech, homestead_tech_id, true);
            unlocked_homestead_techs += 1;
        }

        if claim_tech.learned.len() != learned_count_before {
            ctx.db.claim_tech_state().entity_id().update(claim_tech);
            updated_claims += 1;
        }
    }

    spacetimedb::log::info!(
        "Unlocked {unlocked_starter_claim_techs} Starter Claim techs and {unlocked_homestead_techs} homestead claim techs across {updated_claims} claims"
    );

    Ok(())
}

fn is_profession_claim_tech(tech_type: ClaimTechType) -> bool {
    matches!(
        tech_type,
        ClaimTechType::Forestry
            | ClaimTechType::Carpentry
            | ClaimTechType::Masonry
            | ClaimTechType::Mining
            | ClaimTechType::Smithing
            | ClaimTechType::Scholar
            | ClaimTechType::Leatherworking
            | ClaimTechType::Hunting
            | ClaimTechType::Tailoring
            | ClaimTechType::Farming
            | ClaimTechType::Fishing
            | ClaimTechType::Foraging
    )
}
