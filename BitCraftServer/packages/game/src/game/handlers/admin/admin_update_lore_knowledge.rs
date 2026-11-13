use std::collections::HashMap;

use crate::game::handlers::authentication::has_role;
use crate::knowledge_scroll_desc;
use crate::messages::authentication::Role;
use crate::messages::components::{knowledge_lore_state, knowledge_secondary_state, KnowledgeEntry, KnowledgeState};
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::reducer]
fn admin_update_lore_knowledge(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // lore
    let knowledge_to_lore_ids: HashMap<i32, i32> = ctx
        .db
        .knowledge_scroll_desc()
        .iter()
        .map(|x| (x.secondary_knowledge_id, x.item_id))
        .collect();

    for knowledge_secondary_state in ctx.db.knowledge_secondary_state().iter() {
        let mut knowledge_lore_state = ctx
            .db
            .knowledge_lore_state()
            .entity_id()
            .find(knowledge_secondary_state.entity_id)
            .unwrap();
        let mut updated = false;
        for knowledge_entry in knowledge_secondary_state.entries {
            if let Some(item_id) = knowledge_to_lore_ids.get(&knowledge_entry.id) {
                if knowledge_entry.state == KnowledgeState::Acquired
                    && knowledge_lore_state.entries.iter().find(|e| e.id == *item_id).is_none() {
                        knowledge_lore_state.entries.push(KnowledgeEntry {
                            id: *item_id,
                            state: KnowledgeState::Acquired,
                        });
                        updated = true;
                    }
            }
        }
        if updated {
            log::info!("Added lore knowledge for {}", knowledge_lore_state.entity_id);
            ctx.db.knowledge_lore_state().entity_id().update(knowledge_lore_state);
        }
    }

    Ok(())
}
