use spacetimedb::ReducerContext;

use crate::game::coordinates::*;
use crate::messages::components::{footprint_tile_state, FootprintTileState, LocationState};

impl FootprintTileState {
    pub fn get_at_location<'a>(ctx: &'a ReducerContext, coordinates: &SmallHexTile) -> impl Iterator<Item = FootprintTileState> + 'a {
        LocationState::select_all(ctx, coordinates).filter_map(|ls| ctx.db.footprint_tile_state().entity_id().find(ls.entity_id))
    }
}
