use crate::contract::{AppResult, DCAApp};
use crate::msg::{DCAQueryMsg, ConfigResponse, DCAResponse};
use crate::state::{CONFIG, DCA_LIST};
use cosmwasm_std::{to_binary, Binary, Deps, Env, StdResult};

pub fn query_handler(deps: Deps, _env: Env, _app: &DCAApp, msg: DCAQueryMsg) -> AppResult<Binary> {
    match msg {
        DCAQueryMsg::Config {} => to_binary(&query_config(deps)?),
        DCAQueryMsg::DCA { dca_id } => to_binary(&query_dca(deps, dca_id)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

/// Get dca
fn query_dca(deps: Deps, dca_id: String) -> StdResult<DCAResponse> {
    let dca = DCA_LIST.may_load(deps.storage, dca_id)?;
    Ok(DCAResponse { dca })
}
