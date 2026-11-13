use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::messages::action_request::BarterStallSetMarketModeEnabledRequest;

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn barter_stall_set_market_mode_enabled(_ctx: &ReducerContext, _request: BarterStallSetMarketModeEnabledRequest) -> Result<(), String> {
    Err("Market mode has been discontinued".into())
}
