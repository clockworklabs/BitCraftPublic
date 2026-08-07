use crate::messages::{
    generic::world_region_state,
    inter_module::{inter_module_message_v5, InterModuleMessageV5, MessageContentsV5},
};
use _autogen::InterModuleTableUpdatesV2;
use spacetimedb::{ReducerContext, Table};
use std::cell::RefCell;

pub mod _autogen;
pub mod reducers;

pub mod claim_create_empire_settlement_state;
pub mod claim_set_name;
pub mod empire_create_building;
pub mod empire_remove_crown;
pub mod empire_update_emperor_crown;
pub mod grant_hub_item;
pub mod on_claim_members_changed;
pub mod on_deployable_recovered;
pub mod on_empire_building_deleted;
pub mod on_player_joined_empire;
pub mod on_player_left_empire;
pub mod on_player_name_set;
pub mod player_create;
pub mod player_skip_queue;
pub mod recover_deployable;
pub mod region_destroy_siege_engine;
pub mod replace_identity;
pub mod restore_skills;
pub mod system_chat_broadcast;
pub mod transfer_player;
pub mod transfer_player_housing;
pub mod user_update_region;

#[allow(dead_code)]
pub struct SharedTransactionAccumulator<'a> {
    pub ctx: &'a ReducerContext,
}

impl Drop for SharedTransactionAccumulator<'_> {
    fn drop(&mut self) {
        self.send_shared_transaction();
    }
}

enum InterModuleAccumulator {
    None,                                 //This is not a shared reducer
    Uninitialized,                        //This is a shared reducer, but no shared operations have been performed yet
    Initialized(InterModuleTableUpdatesV2), //List of performed shared operations
}

thread_local! {
    static TABLE_UPDATES_GLOBAL: RefCell<InterModuleAccumulator> = RefCell::new(InterModuleAccumulator::None);
    static TABLE_UPDATES_OTHER_REGIONS: RefCell<InterModuleAccumulator> = RefCell::new(InterModuleAccumulator::None);
    static DELAYED_MESSAGES: RefCell<Vec<(crate::messages::inter_module::MessageContentsV5, crate::inter_module::InterModuleDestination)>> = RefCell::new(Vec::new());
    static TIMESTAMP: RefCell<i64> = RefCell::new(0);
}

#[derive(Clone, Copy)]
pub enum InterModuleDestination {
    Global,
    AllOtherRegions,
    GlobalAndAllOtherRegions,
    Region(u8),
}

impl SharedTransactionAccumulator<'_> {
    pub fn begin_shared_transaction(&self) {
        let ts = self.ctx.timestamp.to_micros_since_unix_epoch();

        TABLE_UPDATES_GLOBAL.with_borrow_mut(|t| {
            match t {
                InterModuleAccumulator::Uninitialized |
                InterModuleAccumulator::Initialized(_) => {
                    if TIMESTAMP.with_borrow(|t| *t == ts) {
                        spacetimedb::log::error!("Function with `#[shared_table_reducer]` attribute called by another shared reducer. This **WILL** cause a panic.");
                    }
                    else {
                        spacetimedb::log::warn!("There is already a pending shared transaction that will be overwritten. This might've been caused by previous shared reducer panic.");
                    }
                },
                InterModuleAccumulator::None => {}
            }
            *t = InterModuleAccumulator::Uninitialized;
        });
        TABLE_UPDATES_OTHER_REGIONS.with_borrow_mut(|t| {
            match t {
                InterModuleAccumulator::Uninitialized |
                InterModuleAccumulator::Initialized(_) => {
                    if TIMESTAMP.with_borrow(|t| *t == ts) {
                        spacetimedb::log::error!("Function with `#[shared_table_reducer]` attribute called by another shared reducer. This **WILL** cause a panic.");
                    }
                    else {
                        spacetimedb::log::warn!("There is already a pending shared transaction that will be overwritten. This might've been caused by previous shared reducer panic.");
                    }
                },
                InterModuleAccumulator::None => {}
            }
            *t = InterModuleAccumulator::Uninitialized;
        });

        DELAYED_MESSAGES.with_borrow_mut(|v| {
            if v.len() > 0 {
                if TIMESTAMP.with_borrow(|t| *t == ts) {
                    spacetimedb::log::error!("Function with `#[shared_table_reducer]` attribute called by another shared reducer. This **WILL** cause a panic.");
                }
                else {
                    spacetimedb::log::warn!("There are inter-module messages that were never sent and will now be cleared. This might've been caused by previous shared reducer panic.");
                }
                v.clear();
            }
        });

        TIMESTAMP.set(ts);
    }

    pub fn send_shared_transaction(&self) {
        TABLE_UPDATES_GLOBAL.with_borrow_mut(|t| {
            if let InterModuleAccumulator::Initialized(a) = t {
                self.ctx.db.inter_module_message_v5().insert(InterModuleMessageV5 {
                    id: 0,
                    to: 0,
                    contents: MessageContentsV5::TableUpdate(a.clone()),
                });
            }
            *t = InterModuleAccumulator::None;
        });
        TABLE_UPDATES_OTHER_REGIONS.with_borrow_mut(|t| {
            if let InterModuleAccumulator::Initialized(a) = t {
                let region_info = self.ctx.db.world_region_state().iter().next().unwrap();
                let cur_region = region_info.region_index;
                let region_count = region_info.region_count;
                for i in 1..=region_count {
                    if i == cur_region {
                        continue;
                    }
                    self.ctx.db.inter_module_message_v5().insert(InterModuleMessageV5 {
                        id: 0,
                        to: i,
                        contents: MessageContentsV5::TableUpdate(a.clone()),
                    });
                }
            }
            *t = InterModuleAccumulator::None;
        });

        DELAYED_MESSAGES.with_borrow_mut(|v| {
            for (msg, dst) in &mut *v {
                send_inter_module_message(self.ctx, msg.clone(), *dst);
            }
            v.clear();
        });

        TIMESTAMP.set(0);
    }
}

pub fn add_global_table_update<F>(callback: F)
where
    F: FnOnce(&mut InterModuleTableUpdatesV2),
{
    TABLE_UPDATES_GLOBAL.with_borrow_mut(|t| {
        if let InterModuleAccumulator::None = t {
            panic!("Shared operations require reducers decorated with `#[shared_table_reducer]` attribute");
        }
        if let InterModuleAccumulator::Uninitialized = t {
            *t = InterModuleAccumulator::Initialized(InterModuleTableUpdatesV2::new());
        }
        if let InterModuleAccumulator::Initialized(a) = t {
            callback(a);
        }
    });
}

pub fn add_region_table_update<F>(callback: F)
where
    F: FnOnce(&mut InterModuleTableUpdatesV2),
{
    TABLE_UPDATES_OTHER_REGIONS.with_borrow_mut(|t| {
        if let InterModuleAccumulator::None = t {
            panic!("Shared operations require reducers decorated with `#[shared_table_reducer]` attribute");
        }
        if let InterModuleAccumulator::Uninitialized = t {
            *t = InterModuleAccumulator::Initialized(InterModuleTableUpdatesV2::new());
        }
        if let InterModuleAccumulator::Initialized(a) = t {
            callback(a);
        }
    });
}

pub fn send_inter_module_message<T>(ctx: &ReducerContext, contents: T, dst: crate::inter_module::InterModuleDestination)
where
    T: Into<crate::messages::inter_module::MessageContentsV5>,
{
    let contents = contents.into();
    let is_none = TABLE_UPDATES_OTHER_REGIONS.with_borrow(|t| if let InterModuleAccumulator::None = t { true } else { false });
    if !is_none {
        DELAYED_MESSAGES.with_borrow_mut(|v| v.push((contents, dst)));
        return;
    }

    match dst {
        crate::inter_module::InterModuleDestination::Global | crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions => {
            ctx.db
                .inter_module_message_v5()
                .insert(crate::messages::inter_module::InterModuleMessageV5 {
                    id: 0,
                    to: 0,
                    contents: contents.clone(),
                });
        }

        _ => {}
    }

    match dst {
        crate::inter_module::InterModuleDestination::AllOtherRegions
        | crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions => {
            let region_info = ctx.db.world_region_state().iter().next().unwrap();
            let cur_region = region_info.region_index;
            let region_count = region_info.region_count;
            for i in 1..=region_count {
                if i == cur_region {
                    continue;
                }
                ctx.db.inter_module_message_v5().insert(InterModuleMessageV5 {
                    id: 0,
                    to: i,
                    contents: contents.clone(),
                });
            }
        }

        _ => {}
    }

    if let crate::inter_module::InterModuleDestination::Region(region_id) = dst {
        if region_id <= 0 {
            panic!("Region id must be > 0");
        }
        let region_info = ctx.db.world_region_state().iter().next().unwrap();
        let cur_region = region_info.region_index;
        let region_count = region_info.region_count;
        if region_id == cur_region {
            panic!("Destination cannot be current module");
        }
        if region_id > region_count {
            panic!("Region with provided id doesn't exist");
        }

        ctx.db
            .inter_module_message_v5()
            .insert(crate::messages::inter_module::InterModuleMessageV5 {
                id: 0,
                to: region_id,
                contents: contents,
            });
    }
}
