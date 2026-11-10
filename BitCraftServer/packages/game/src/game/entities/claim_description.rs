use std::time::Duration;

use spacetimedb::{ReducerContext, Table, TimeDuration};

use crate::{
    alert_state, claim_state, claim_tech_state, claim_tile_cost,
    game::{discovery::Discovery, game_state},
    inter_module::on_claim_members_changed,
    messages::{
        components::{
            claim_local_state, claim_local_supply_security_threshold_state, claim_member_state, AlertState, ClaimLocalState,
            ClaimMemberState, ClaimState,
        },
        static_data::{AlertType, ClaimTileCost},
    },
    parameters_desc, params, rent_state, unwrap_or_err, PlayerState,
};

const DEFAULT_TILE_COST: f32 = 0.1;

fn marginal_tile_maintenance(tile_count: i32, mut tile_costs: Vec<ClaimTileCost>) -> f32 {
    if tile_count <= 0 {
        return 0.0;
    }

    tile_costs.sort_unstable_by_key(|tile_cost| tile_cost.tile_count);

    let mut maintenance = 0.0;
    let mut bracket_start = 1;
    let mut cost_per_tile = DEFAULT_TILE_COST;

    for tile_cost in tile_costs {
        let next_bracket_start = tile_cost.tile_count.max(1);
        if next_bracket_start > tile_count {
            break;
        }

        maintenance += (next_bracket_start - bracket_start) as f32 * cost_per_tile;
        bracket_start = next_bracket_start;
        cost_per_tile = tile_cost.cost_per_tile;
    }

    maintenance + (tile_count - bracket_start + 1) as f32 * cost_per_tile
}

impl ClaimState {
    pub fn clear_notifications(ctx: &ReducerContext, claim_entity_id: u64) {
        if let Some(claim) = ctx.db.claim_state().entity_id().find(&claim_entity_id) {
            for member in claim.members(ctx) {
                // DAB Note: It could be nice to add a "your claim was destroyed because you didn't take care of your claim totem" alert.
                AlertState::delete(ctx, AlertType::OutOfSupplies, member.player_entity_id, claim.entity_id);
                AlertState::delete(ctx, AlertType::OutOfSuppliesInTwelveTicks, member.player_entity_id, claim.entity_id);
                AlertState::delete(ctx, AlertType::OutOfSuppliesInOneTick, member.player_entity_id, claim.entity_id);
                AlertState::delete(
                    ctx,
                    AlertType::CoOwnerClaimOwnershipTransferIn24h,
                    member.player_entity_id,
                    claim.entity_id,
                );
                AlertState::delete(
                    ctx,
                    AlertType::CoOwnerClaimOwnershipTransfer,
                    member.player_entity_id,
                    claim.entity_id,
                );
                AlertState::delete(
                    ctx,
                    AlertType::OfficerClaimOwnershipTransfer,
                    member.player_entity_id,
                    claim.entity_id,
                );
                AlertState::delete(
                    ctx,
                    AlertType::MemberClaimOwnershipTransfer,
                    member.player_entity_id,
                    claim.entity_id,
                );
            }

            // it could also be nice to add a "your rent was cancelled because your landlord is incompetent" alert.
            for rent in ctx.db.rent_state().claim_entity_id().filter(claim_entity_id) {
                if !rent.white_list.is_empty() {
                    AlertState::delete(ctx, AlertType::EvictionWarning, rent.white_list[0], rent.entity_id);
                    AlertState::delete(ctx, AlertType::EvictionStatement, rent.white_list[0], rent.entity_id);
                }
            }
        }
    }

    pub fn local_state(&self, ctx: &ReducerContext) -> ClaimLocalState {
        return ctx.db.claim_local_state().entity_id().find(self.entity_id).unwrap();
    }

    pub fn get_member(&self, ctx: &ReducerContext, entity_id: u64) -> Option<ClaimMemberState> {
        return ctx
            .db
            .claim_member_state()
            .player_claim()
            .filter((entity_id, self.entity_id))
            .next();
    }

    pub fn is_member(ctx: &ReducerContext, actor_id: u64, claim_entity_id: u64) -> bool {
        return ctx
            .db
            .claim_member_state()
            .player_claim()
            .filter((actor_id, claim_entity_id))
            .next()
            .is_some();
    }

    pub fn has_owner_permissions(&self, entity_id: u64) -> bool {
        self.owner_player_entity_id == entity_id
    }

    pub fn has_co_owner_permissions(&self, ctx: &ReducerContext, entity_id: u64) -> bool {
        self.has_owner_permissions(entity_id)
            || match self.get_member(ctx, entity_id) {
                Some(m) => m.co_owner_permission,
                None => false,
            }
    }

    pub fn has_officer_permissions(&self, ctx: &ReducerContext, entity_id: u64) -> bool {
        self.has_owner_permissions(entity_id)
            || match self.get_member(ctx, entity_id) {
                Some(m) => m.co_owner_permission || m.officer_permission,
                None => false,
            }
    }

    pub fn has_build_permissions(&self, ctx: &ReducerContext, entity_id: u64) -> bool {
        self.has_owner_permissions(entity_id)
            || match self.get_member(ctx, entity_id) {
                Some(m) => m.co_owner_permission || m.officer_permission || m.build_permission,
                None => false,
            }
    }

    pub fn has_inventory_permissions(&self, ctx: &ReducerContext, entity_id: u64) -> bool {
        self.has_owner_permissions(entity_id)
            || match self.get_member(ctx, entity_id) {
                Some(m) => m.co_owner_permission || m.officer_permission || m.inventory_permission,
                None => false,
            }
    }

    pub fn score_permissions(&self, member: &ClaimMemberState) -> i32 {
        let mut score = 0;
        if member.officer_permission {
            score += 1;
        }
        if member.co_owner_permission {
            score += 1;
        }
        if self.has_owner_permissions(member.player_entity_id) {
            score += 1;
        }
        score
    }

    fn refresh_end_timestamp(ctx: &ReducerContext, mut alert: AlertState, supplies: f32, decay: f32) {
        let tick_length = ctx.db.parameters_desc().version().find(&0).unwrap().building_decay_tick_millis as u64;
        let milliseconds_elapsed = game_state::unix_ms(ctx.timestamp);
        let next_tick = ((tick_length - (milliseconds_elapsed % tick_length)) / 1000) as f32;
        let duration = Duration::from_secs_f32(next_tick + supplies / decay * ((tick_length / 1000) as f32));
        alert.end_timestamp = ctx.timestamp + TimeDuration::from(duration);
        ctx.db.alert_state().entity_id().update(alert);
    }

    pub fn add_member(
        &self,
        ctx: &ReducerContext,
        player_entity_id: u64,
        build_permission: bool,
        inventory_permission: bool,
        officer_permission: bool,
        co_owner_permission: bool,
    ) -> Result<(), String> {
        if self.neutral {
            return Err("Cannot be a member of a neutral claim.".into());
        }

        let claim_member = ClaimMemberState {
            entity_id: game_state::create_entity(ctx),
            claim_entity_id: self.entity_id,
            player_entity_id,
            user_name: unwrap_or_err!(PlayerState::username_by_id(ctx, player_entity_id), "Player doesn't exist"),
            inventory_permission,
            build_permission,
            officer_permission,
            co_owner_permission,
        };

        ClaimMemberState::insert_shared(ctx, claim_member, crate::inter_module::InterModuleDestination::Global);
        on_claim_members_changed::send_message(ctx, self.entity_id);

        // Grant "Claim" secondary knowledge to added player
        let mut discovery = Discovery::new(player_entity_id);
        discovery.acquire_secondary(ctx, 100001); // 100001 is Claim knowledge
        discovery.commit(ctx);

        Ok(())
    }

    pub fn members(&self, ctx: &ReducerContext) -> impl Iterator<Item = ClaimMemberState> {
        return ctx.db.claim_member_state().claim_entity_id().filter(self.entity_id);
    }

    pub fn num_tiles_in_radius(radius: i32) -> i32 {
        return 1 + 3 * radius * (radius + 1);
    }

    fn num_tiles_in_ring(radius: i32) -> i32 {
        return radius * 6;
    }

    pub fn num_neighbors_in_radius(radius: i32) -> i32 {
        let num_tiles_inner_radius = Self::num_tiles_in_radius(radius - 1);
        let num_tiles_outer_ring = Self::num_tiles_in_ring(radius);
        let num_neighbors_inner_radius = num_tiles_inner_radius * 6; //Tiles in the inner radius have 6 neighbors each
        let num_neighbors_outer_ring = 6 * 3 + (num_tiles_outer_ring - 6) * 4; //Outer ring has 6 corners with 3 neighbors each, the other tiles have 4 neighbors

        return num_neighbors_inner_radius + num_neighbors_outer_ring;
    }

    pub fn get_upkeep_multiplier(&self, ctx: &ReducerContext) -> f32 {
        let local = self.local_state(ctx);
        return local.get_upkeep_multiplier(ctx);
    }
}

impl ClaimMemberState {
    pub fn set_permissions(
        mut self,
        ctx: &ReducerContext,
        build_permission: bool,
        inventory_permission: bool,
        officer_permission: bool,
        co_owner_permission: bool,
    ) {
        self.build_permission = build_permission;
        self.inventory_permission = inventory_permission;
        self.officer_permission = officer_permission;
        self.co_owner_permission = co_owner_permission;
        ctx.db.claim_member_state().entity_id().update(self);
    }
}

impl ClaimLocalState {
    pub fn members(&self, ctx: &ReducerContext) -> impl Iterator<Item = ClaimMemberState> {
        return ctx.db.claim_member_state().claim_entity_id().filter(self.entity_id);
    }

    pub fn update_supplies_and_commit(mut self, ctx: &ReducerContext, supplies_delta: f32, check_threshold: bool) -> Result<(), String> {
        let one_tick_threshold = self.building_maintenance + self.tiles_maintenance(ctx) * self.get_upkeep_multiplier(ctx);
        let twelve_ticks_threhsold = one_tick_threshold * 12.0;

        let previous_supplies = self.supplies as f32;
        let mut next_supplies = (self.supplies as f32 + supplies_delta).max(0.0);
        if supplies_delta > 0.0 {
            let max_supplies = ctx
                .db
                .claim_tech_state()
                .entity_id()
                .find(&self.entity_id)
                .unwrap()
                .max_supplies(ctx);
            next_supplies = next_supplies.min(max_supplies);
        }

        if supplies_delta == 0.0 {
            return Ok(());
        }

        //round to nearest integer
        next_supplies = next_supplies.round();
        let members: Vec<ClaimMemberState> = self.members(ctx).collect();

        let co_owner_warn_secs = params!(ctx).co_owner_take_ownership_supply_time;

        let co_owner_pre_warning_threshold = self.get_required_supplies_for_seconds(ctx, co_owner_warn_secs + 24 * 60 * 60) as f32; // one day more than co-owner warning
        let co_owner_warning_threshold = self.get_required_supplies_for_seconds(ctx, co_owner_warn_secs) as f32;
        let officer_warning_threshold = self.get_required_supplies_for_seconds(ctx, params!(ctx).officer_take_ownership_supply_time) as f32;
        let member_warning_threshold = self.get_required_supplies_for_seconds(ctx, params!(ctx).member_take_ownership_supply_time) as f32;

        /*
        log::info!("supplies: {previous_supplies} -> {next_supplies}");
        log::info!("co_owner_pre_warning_threshold: {co_owner_pre_warning_threshold}");
        log::info!("co_owner_warning_threshold: {co_owner_warning_threshold}");
        log::info!("officer_warning_threshold: {officer_warning_threshold}");
        log::info!("member_warning_threshold: {member_warning_threshold}");
        */

        if previous_supplies > next_supplies {
            // Only check threshold if supplies actually decreased
            if check_threshold {
                let threshold = match ctx
                    .db
                    .claim_local_supply_security_threshold_state()
                    .entity_id()
                    .find(self.entity_id)
                {
                    Some(t) => self.get_required_supplies_for_seconds(ctx, t.supply_security_threshold_hours * 3600) as f32,
                    None => co_owner_warning_threshold, // default - co-owner ownership transfer value
                };
                if next_supplies < threshold {
                    return Err("Cannot lower the supplies below the security threshold".into());
                }
            }

            if previous_supplies > 0.0 && next_supplies <= 0.0 {
                for member in &members {
                    // Out of supplies alert to ALL members
                    AlertState::new(ctx, AlertType::OutOfSupplies, member.player_entity_id, self.entity_id).unwrap();
                    // Remove 12-ticks and 1-tick alerts
                    AlertState::delete(ctx, AlertType::OutOfSuppliesInTwelveTicks, member.player_entity_id, self.entity_id);
                    AlertState::delete(ctx, AlertType::OutOfSuppliesInOneTick, member.player_entity_id, self.entity_id);
                }
            } else if previous_supplies >= one_tick_threshold && next_supplies < one_tick_threshold {
                for member in &members {
                    // One tick left alert to ALL members
                    AlertState::new(ctx, AlertType::OutOfSuppliesInOneTick, member.player_entity_id, self.entity_id).unwrap();
                    // Remove 12-ticks alerts
                    AlertState::delete(ctx, AlertType::OutOfSuppliesInTwelveTicks, member.player_entity_id, self.entity_id);
                }
            } else if previous_supplies >= twelve_ticks_threhsold && next_supplies < twelve_ticks_threhsold {
                for member in members.iter().filter(|m| m.officer_permission) {
                    // 12 ticks left alert to OFFICERS
                    AlertState::new(ctx, AlertType::OutOfSuppliesInTwelveTicks, member.player_entity_id, self.entity_id).unwrap();
                }
            }

            // Check if we're entering a co-owner/officer/member warning threshold
            if previous_supplies > member_warning_threshold && next_supplies <= member_warning_threshold {
                // Members (not officers or co-owners) are alerted
                for member in members.iter().filter(|m| !m.officer_permission) {
                    let _ = AlertState::new(
                        ctx,
                        AlertType::MemberClaimOwnershipTransfer,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
            if previous_supplies > officer_warning_threshold && next_supplies <= officer_warning_threshold {
                // Officers (not co-owners) are alerted
                for member in members.iter().filter(|m| m.officer_permission && !m.co_owner_permission) {
                    let _ = AlertState::new(
                        ctx,
                        AlertType::OfficerClaimOwnershipTransfer,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
            if previous_supplies > co_owner_warning_threshold && next_supplies <= co_owner_warning_threshold {
                // Co-owners are alerted
                for member in members.iter().filter(|m| m.co_owner_permission) {
                    let _ = AlertState::new(
                        ctx,
                        AlertType::CoOwnerClaimOwnershipTransfer,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            } else if previous_supplies > co_owner_pre_warning_threshold && next_supplies <= co_owner_pre_warning_threshold {
                // Co-owners are alerted unless the threshold is already past the 24h warning
                for member in members.iter().filter(|m| m.co_owner_permission) {
                    let _ = AlertState::new(
                        ctx,
                        AlertType::CoOwnerClaimOwnershipTransferIn24h,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
        } else {
            if previous_supplies <= 0.0 && next_supplies > 0.0 {
                for member in &members {
                    // Remove out of supplies alert to ALL members
                    AlertState::delete(ctx, AlertType::OutOfSupplies, member.player_entity_id, self.entity_id);
                }
            }
            if previous_supplies < one_tick_threshold && next_supplies >= one_tick_threshold {
                for member in &members {
                    // Remove One tick left alert to ALL members
                    AlertState::delete(ctx, AlertType::OutOfSuppliesInOneTick, member.player_entity_id, self.entity_id);
                }
            }
            if previous_supplies < twelve_ticks_threhsold && next_supplies >= twelve_ticks_threhsold {
                for member in members.iter().filter(|m| m.officer_permission) {
                    // Remove 12 ticks left alert to OFFICERS
                    AlertState::delete(ctx, AlertType::OutOfSuppliesInTwelveTicks, member.player_entity_id, self.entity_id);
                }
            }

            // Check if we're leaving a co-owner/officer/member warning threshold
            if previous_supplies <= co_owner_pre_warning_threshold && next_supplies > co_owner_pre_warning_threshold {
                for member in members.iter().filter(|m| m.co_owner_permission) {
                    AlertState::delete(
                        ctx,
                        AlertType::CoOwnerClaimOwnershipTransferIn24h,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
            if previous_supplies <= co_owner_warning_threshold && next_supplies > co_owner_warning_threshold {
                for member in members.iter().filter(|m| m.co_owner_permission) {
                    AlertState::delete(
                        ctx,
                        AlertType::CoOwnerClaimOwnershipTransfer,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
            if previous_supplies <= officer_warning_threshold && next_supplies > officer_warning_threshold {
                for member in members.iter().filter(|m| m.officer_permission && !m.co_owner_permission) {
                    AlertState::delete(
                        ctx,
                        AlertType::OfficerClaimOwnershipTransfer,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
            if previous_supplies <= member_warning_threshold && next_supplies > member_warning_threshold {
                for member in members.iter().filter(|m| !m.officer_permission) {
                    AlertState::delete(
                        ctx,
                        AlertType::MemberClaimOwnershipTransfer,
                        member.player_entity_id,
                        self.entity_id,
                    );
                }
            }
        }
        let self_entity_id = self.entity_id;
        self.supplies = next_supplies as i32;
        ctx.db.claim_local_state().entity_id().update(self);

        // Adjust duration of existing alerts based on remaining supplies. Don't pop new alerts unless a threshold causes it.
        for member in &members {
            if let Some(alert) = AlertState::get(ctx, AlertType::OutOfSuppliesInOneTick, member.player_entity_id, self_entity_id) {
                ClaimState::refresh_end_timestamp(ctx, alert, next_supplies, one_tick_threshold);
            }
            if let Some(alert) = AlertState::get(ctx, AlertType::OutOfSuppliesInTwelveTicks, member.player_entity_id, self_entity_id) {
                ClaimState::refresh_end_timestamp(ctx, alert, next_supplies, one_tick_threshold);
            }
        }
        Ok(())
    }

    pub fn tiles_maintenance(&self, ctx: &ReducerContext) -> f32 {
        marginal_tile_maintenance(self.num_tiles, ctx.db.claim_tile_cost().iter().collect())
    }

    pub fn get_upkeep_multiplier(&self, _ctx: &ReducerContext) -> f32 {
        if self.num_tiles <= 0 {
            return 1f32;
        }
        let stability = self.num_tile_neighbors as f64 / self.num_tiles as f64;

        // Hardcoded for now instead of the static-data claim_stability_param_m/b values.
        // Keep the client ClaimLocalState.UpkeepMultiplier calculation in sync.
        (5.5f64 * (5.65f64 - stability)).exp().max(1f64) as f32
    }

    pub fn full_maintenance(&self, ctx: &ReducerContext) -> f32 {
        self.tiles_maintenance(ctx) * self.get_upkeep_multiplier(ctx) + self.building_maintenance
    }

    pub fn get_required_supplies_for_seconds(&self, ctx: &ReducerContext, seconds: i32) -> i32 {
        let tick_secs = params!(ctx).building_decay_tick_millis / 1000;
        let ticks = (seconds + tick_secs - 1) / tick_secs;
        self.full_maintenance(ctx) as i32 * ticks
    }
}
