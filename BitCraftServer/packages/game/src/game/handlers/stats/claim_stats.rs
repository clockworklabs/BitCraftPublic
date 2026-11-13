use crate::game::handlers::authentication::has_role;
use crate::messages::authentication::Role;
use crate::messages::components::claim_member_state;
use crate::{claim_state, claim_tech_state};
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::reducer]
pub fn log_claim_member_leaderboard(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let mut claims: Vec<(String, usize)> = ctx
        .db
        .claim_state()
        .iter()
        .map(|c| (c.name, ctx.db.claim_member_state().claim_entity_id().filter(c.entity_id).count()))
        .collect();
    claims.sort_by_key(|b| std::cmp::Reverse(b.1));

    log::info!("Claim Size Ranking");
    log::info!("|        Claim Name         |  Member Count  |");
    for (name, member_count) in claims {
        if member_count < 5 {
            continue;
        }

        if name.len() >= 26 {
            log::info!("| {}|{:16} |", &name[0..26], member_count);
        } else {
            log::info!("| {:26}|{:16} |", name, member_count);
        }
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn log_claim_tier_leaderboard(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let mut claim_tiers: Vec<(String, i32)> = ctx
        .db
        .claim_tech_state()
        .iter()
        .filter_map(|c| {
            let mut highest_tier = 1;
            for tech in c.learned {
                if tech == 600 || tech == 500 || tech == 400 || tech == 300 || tech == 200 || tech == 100 {
                    let tech = tech / 100;
                    if tech > highest_tier {
                        highest_tier = tech;
                    }
                }
            }
            if highest_tier > 3 {
                let claim = ctx.db.claim_state().entity_id().find(c.entity_id).unwrap();
                Some((claim.name, highest_tier))
            } else {
                None
            }
        })
        .collect();

    claim_tiers.sort_by_key(|b| std::cmp::Reverse(b.1));
    log::info!("Claim Tier Ranking");
    log::info!("|        Claim Name        |  Tier  |");
    for (name, tier) in claim_tiers {
        if name.len() >= 26 {
            log::info!("| {}|{:7} |", &name[0..26], tier);
        } else {
            log::info!("| {:26}|{:7} |", name, tier);
        }
    }

    Ok(())
}
