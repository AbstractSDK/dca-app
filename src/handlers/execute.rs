use abstract_core::objects::UncheckedContractEntry;
use abstract_dex_adapter::msg::OfferAsset;
use abstract_sdk::features::{AbstractNameService, AbstractResponse, AccountIdentification};
use cosmwasm_std::{wasm_execute, Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use croncat_app::contract::CRONCAT_ID;
use cw_asset::{Asset, AssetList};

use crate::contract::{AppResult, DCAApp};

use crate::error::AppError;
use crate::msg::{AppExecuteMsg, ExecuteMsg, Frequency};
use crate::state::{DCAEntry, CONFIG, DCA_LIST, NEXT_ID};
use abstract_dex_adapter::api::DexInterface;
use abstract_sdk::prelude::*;
use croncat_app::croncat_integration_utils::{CronCatAction, CronCatInterval, CronCatTaskRequest};
use croncat_app::{CronCatInterface, CRON_CAT_FACTORY};
pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: DCAApp,
    msg: AppExecuteMsg,
) -> AppResult {
    match msg {
        AppExecuteMsg::UpdateConfig {} => update_config(deps, info, app),
        AppExecuteMsg::CreateDCA {
            source_asset,
            target_asset,
            frequency,
            dex,
        } => create_dca(
            deps,
            env,
            info,
            app,
            source_asset,
            target_asset,
            frequency,
            dex,
        ),
        AppExecuteMsg::UpdateDCA {
            dca_id,
            new_source_asset,
            new_target_asset,
            new_frequency,
            new_dex,
        } => todo!(),
        AppExecuteMsg::CancelDCA { dca_id } => todo!(),
        AppExecuteMsg::Convert { dca_id } => convert(deps, env, info, app, dca_id),
    }
}

/// Update the configuration of the app
fn update_config(deps: DepsMut, msg_info: MessageInfo, app: DCAApp) -> AppResult {
    // Only the admin should be able to call this
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;
    let mut _config = CONFIG.load(deps.storage)?;

    Ok(app.tag_response(Response::default(), "update_config"))
}

fn create_dca(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: DCAApp,
    source_asset: String,
    target_asset: String,
    frequency: Frequency,
    dex_name: String,
) -> AppResult {
    // Only the admin should be able to create dca
    app.admin.assert_admin(deps.as_ref(), &info.sender)?;
    let config = CONFIG.load(deps.storage)?;

    // Generate DCA ID
    let dca_id = format!("dca_{}", NEXT_ID.load(deps.storage)?);

    let dca_entry = DCAEntry {
        source_asset,
        target_asset,
        frequency,
        dex: dex_name,
    };
    DCA_LIST.save(deps.storage, dca_id.clone(), &dca_entry)?;

    let interval = frequency.into_interval();

    let task = CronCatTaskRequest {
        interval,
        boundary: None,
        // TODO?: should it be false or argument?
        stop_on_fail: true,
        actions: vec![CronCatAction {
            msg: wasm_execute(
                env.contract.address,
                &ExecuteMsg::from(AppExecuteMsg::Convert {
                    dca_id: dca_id.clone(),
                }),
                vec![],
            )?
            .into(),
            gas_limit: Some(200_000),
        }],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let assets = AssetList::from(vec![Asset::native(
        config.native_denom,
        config.dca_creation_amount,
    )])
    .into();

    let task_msg = app
        .cron_cat(deps.as_ref())
        .create_task(task, dca_id.clone(), assets)?;

    Ok(app.tag_response(
        Response::new()
            .add_message(task_msg)
            .add_attribute("dca_id", dca_id),
        "create_dca",
    ))
}

fn convert(deps: DepsMut, env: Env, info: MessageInfo, app: DCAApp, dca_id: String) -> AppResult {
    let config = CONFIG.load(deps.storage)?;
    let dca = DCA_LIST.load(deps.storage, dca_id.clone())?;

    let manager_addr = app
        .cron_cat(deps.as_ref())
        .query_manager_addr(env.contract.address, dca_id.clone())?;
    if manager_addr != info.sender {
        return Err(AppError::NotManagerConvert {});
    }

    let dex = app.dex(deps.as_ref(), dca.dex);
    let offer_asset = OfferAsset {
        name: dca.source_asset.into(),
        amount: Uint128::new(100),
    };
    let swap_msg = dex.swap(
        offer_asset,
        dca.target_asset.into(),
        Some(Decimal::percent(30)),
        None,
    )?;
    Ok(app.tag_response(Response::new().add_message(swap_msg), "convert"))
}
