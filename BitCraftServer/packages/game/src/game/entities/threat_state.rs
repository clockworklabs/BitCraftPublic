use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::components::{ContributionState, EnemyScalingState},
    threat_state, ThreatState,
};

impl ThreatState {
    fn new(ctx: &ReducerContext, owner_entity_id: u64, target_entity_id: u64, threat: f32) -> ThreatState {
        ThreatState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id,
            target_entity_id,
            threat,
        }
    }

    pub fn add_threat(ctx: &ReducerContext, entity_id: u64, threat_entity_id: u64, amount: f32) {
        if entity_id == threat_entity_id {
            // Never add self-threat
            return;
        }

        if let Some(mut threat) = ctx
            .db
            .threat_state()
            .owner_entity_id()
            .filter(entity_id).find(|t| t.target_entity_id == threat_entity_id)
        {
            threat.threat += amount;
            ctx.db.threat_state().entity_id().update(threat);
        } else {
            let threat = ThreatState::new(ctx, entity_id, threat_entity_id, amount);
            let _ = ctx.db.threat_state().try_insert(threat);
            // Only update scaling if the number of threat entries change
            EnemyScalingState::update(ctx, entity_id);
        }
    }

    pub fn equalize_threat_then_add(ctx: &ReducerContext, entity_id: u64, threat_entity_id: u64, amount: f32) {
        if entity_id == threat_entity_id {
            // Never add self-threat
            return;
        }

        let max_threat = ctx
            .db
            .threat_state()
            .owner_entity_id()
            .filter(entity_id)
            .max_by(|t1, t2| f32::total_cmp(&t1.threat, &t2.threat))
            .map(|t| t.threat)
            .unwrap_or(0.0);

        if let Some(mut threat) = ctx
            .db
            .threat_state()
            .owner_entity_id()
            .filter(entity_id).find(|t| t.target_entity_id == threat_entity_id)
        {
            threat.threat = max_threat + amount;
            ctx.db.threat_state().entity_id().update(threat);
        } else {
            let threat = ThreatState::new(ctx, entity_id, threat_entity_id, max_threat + amount);
            let _ = ctx.db.threat_state().try_insert(threat);
            // Only update scaling if the number of threat entries change
            EnemyScalingState::update(ctx, entity_id);
        }
    }

    pub fn in_combat(ctx: &ReducerContext, entity_id: u64) -> bool {
        ctx.db.threat_state().owner_entity_id().filter(entity_id).next().is_some()
    }

    pub fn clear(ctx: &ReducerContext, owner_entity_id: u64, target_entity_id: u64) {
        if let Some(threat) = ctx
            .db
            .threat_state()
            .owner_entity_id()
            .filter(owner_entity_id).find(|t| t.target_entity_id == target_entity_id)
        {
            ctx.db.threat_state().entity_id().delete(threat.entity_id);
            EnemyScalingState::update(ctx, threat.target_entity_id);
            ContributionState::idle_reset(ctx, threat.target_entity_id);
        }
        if let Some(threat) = ctx
            .db
            .threat_state()
            .owner_entity_id()
            .filter(target_entity_id).find(|t| t.target_entity_id == owner_entity_id)
        {
            ctx.db.threat_state().entity_id().delete(threat.entity_id);
            EnemyScalingState::update(ctx, threat.target_entity_id);
            ContributionState::idle_reset(ctx, threat.target_entity_id);
        }
    }

    pub fn clear_all(ctx: &ReducerContext, owner_entity_id: u64) {
        for threat_state in ctx.db.threat_state().target_entity_id().filter(owner_entity_id) {
            ContributionState::idle_reset(ctx, threat_state.owner_entity_id);
            ctx.db.threat_state().entity_id().delete(threat_state.entity_id);
            EnemyScalingState::update(ctx, threat_state.owner_entity_id);
        }
        ContributionState::idle_reset(ctx, owner_entity_id);
        ctx.db.threat_state().owner_entity_id().delete(owner_entity_id);
        EnemyScalingState::update(ctx, owner_entity_id);
    }

    pub fn clear_others(ctx: &ReducerContext, owner_entity_id: u64) {
        // This is called when an entity dies
        for threat_state in ctx.db.threat_state().target_entity_id().filter(owner_entity_id) {
            ContributionState::idle_reset(ctx, threat_state.owner_entity_id);
            EnemyScalingState::update(ctx, threat_state.owner_entity_id);
        }
        ctx.db.threat_state().target_entity_id().delete(owner_entity_id);
    }
}
