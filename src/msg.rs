use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{Uint128, Uint64};
use croncat_app::croncat_integration_utils::CronCatInterval;

use crate::{contract::DCAApp, state::Config};

// This is used for type safety
// The second part is used to indicate the messages are used as the apps messages
// This is equivalent to
// pub type InstantiateMsg = <App as abstract_sdk::base::InstantiateEndpoint>::InstantiateMsg;
// pub type ExecuteMsg = <App as abstract_sdk::base::ExecuteEndpoint>::ExecuteMsg;
// pub type QueryMsg = <App as abstract_sdk::base::QueryEndpoint>::QueryMsg;
// pub type MigrateMsg = <App as abstract_sdk::base::MigrateEndpoint>::MigrateMsg;

// impl app::AppExecuteMsg for AppExecuteMsg {}
// impl app::AppQueryMsg for AppQueryMsg {}
abstract_app::app_messages!(DCAApp, AppExecuteMsg, AppQueryMsg);

#[cosmwasm_schema::cw_serde]
#[derive(Copy)]
#[non_exhaustive]
pub enum Frequency {
    /// Blocks will schedule the next DCA purchase every `n` blocks.
    EveryNBlocks(u64),
    /// Time will schedule the next DCA purchase after every `n` seconds.
    EveryNSeconds(u64),
}

impl Frequency {
    pub fn into_interval(self) -> CronCatInterval {
        match self {
            Frequency::EveryNBlocks(blocks) => CronCatInterval::Block(blocks),
            Frequency::EveryNSeconds(_) => todo!(),
        }
    }
}
/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct AppInstantiateMsg {
    pub native_denom: String,
    pub dca_creation_amount: Uint128,
    pub refill_threshold: Uint128,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
#[cfg_attr(feature = "interface", impl_into(ExecuteMsg))]
pub enum AppExecuteMsg {
    UpdateConfig {},
    /// Used to create a new DCA
    CreateDCA {
        /// The name of the asset to be used for purchasing
        source_asset: String,
        /// The name of the asset to be purchased
        target_asset: String,
        /// The frequency of purchase
        frequency: Frequency,
        /// The DEX to be used for the swap
        dex: String,
    },

    /// Used to update an existing DCA
    UpdateDCA {
        /// Unique identifier for the DCA
        dca_id: String,
        /// Optional new name of the asset to be used for purchasing
        new_source_asset: Option<String>,
        /// Optional new name of the asset to be purchased
        new_target_asset: Option<String>,
        /// Optional new frequency of purchase
        new_frequency: Option<Frequency>,
        /// Optional new DEX to be used for the swap
        new_dex: Option<String>,
    },

    /// Used to cancel an existing DCA
    CancelDCA {
        /// Unique identifier for the DCA
        dca_id: String,
    },
    Convert {
        dca_id: String,
    },
}

#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[cfg_attr(feature = "interface", impl_into(QueryMsg))]
#[derive(QueryResponses)]
pub enum AppQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cosmwasm_schema::cw_serde]
pub enum AppMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}
