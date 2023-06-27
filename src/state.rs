use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

use crate::msg::Frequency;

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub native_denom: String,
    pub dca_creation_amount: Uint128,
    pub refill_threshold: Uint128,
}

#[cosmwasm_schema::cw_serde]
pub struct DCAEntry {
    pub source_asset: String,
    pub target_asset: String,
    pub frequency: Frequency,
    pub dex: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_ID: Item<u64> = Item::new("next_id");
pub const DCA_LIST: Map<String, DCAEntry> = Map::new("dca_list");
