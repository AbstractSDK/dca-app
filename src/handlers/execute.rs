#![allow(clippy::too_many_arguments)]

use abstract_dex_adapter::msg::OfferAsset;
use abstract_sdk::features::AbstractResponse;
use cosmwasm_std::{
    wasm_execute, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128,
};
use cw_asset::{Asset, AssetList};

use crate::contract::{AppResult, DCAApp};

use crate::error::AppError;
use crate::msg::{AppExecuteMsg, ExecuteMsg, Frequency};
use crate::state::{Config, DCAEntry, CONFIG, DCA_LIST, NEXT_ID};
use abstract_dex_adapter::api::DexInterface;
use abstract_sdk::AbstractSdkResult;
use croncat_app::croncat_integration_utils::{CronCatAction, CronCatTaskRequest};
use croncat_app::{CronCat, CronCatInterface};

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
        } => update_dca(
            deps,
            env,
            info,
            app,
            dca_id,
            new_source_asset,
            new_target_asset,
            new_frequency,
            new_dex,
        ),
        AppExecuteMsg::CancelDCA { dca_id } => cancel_dca(deps, info, app, dca_id),
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

/// Helper to for task creation message
fn create_convert_task_internal(
    env: Env,
    dca: DCAEntry,
    dca_id: String,
    cron_cat: CronCat<DCAApp>,
    config: Config,
) -> AbstractSdkResult<CosmosMsg> {
    let interval = dca.frequency.to_interval();
    let task = CronCatTaskRequest {
        interval,
        boundary: None,
        // TODO?: should it be argument?
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
            gas_limit: Some(300_000),
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
    cron_cat.create_task(task, dca_id, assets)
}

/// Create new DCA
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
    let id = NEXT_ID.update(deps.storage, |id| AppResult::Ok(id + 1))?;
    let dca_id = format!("dca_{id}");

    let dca_entry = DCAEntry {
        source_asset,
        target_asset,
        frequency,
        dex: dex_name,
    };
    DCA_LIST.save(deps.storage, dca_id.clone(), &dca_entry)?;

    let cron_cat = app.cron_cat(deps.as_ref());
    let task_msg = create_convert_task_internal(env, dca_entry, dca_id.clone(), cron_cat, config)?;

    Ok(app.tag_response(
        Response::new()
            .add_message(task_msg)
            .add_attribute("dca_id", dca_id),
        "create_dca",
    ))
}

/// Update existing dca
fn update_dca(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: DCAApp,
    dca_id: String,
    new_source_asset: Option<String>,
    new_target_asset: Option<String>,
    new_frequency: Option<Frequency>,
    new_dex: Option<String>,
) -> AppResult {
    app.admin.assert_admin(deps.as_ref(), &info.sender)?;

    // Only if frequency is changed we have to re-create a task
    let recreate_task = new_frequency.is_some();

    let dca = DCA_LIST.update(deps.storage, dca_id.clone(), |dca| {
        let old_dca = dca.ok_or(AppError::DcaNotFound {})?;
        let new_dca = DCAEntry {
            source_asset: new_source_asset.unwrap_or(old_dca.source_asset),
            target_asset: new_target_asset.unwrap_or(old_dca.target_asset),
            frequency: new_frequency.unwrap_or(old_dca.frequency),
            dex: new_dex.unwrap_or(old_dca.dex),
        };
        AppResult::Ok(new_dca)
    })?;

    let response = if recreate_task {
        let config = CONFIG.load(deps.storage)?;
        let cron_cat = app.cron_cat(deps.as_ref());
        let remove_task_msg = cron_cat.remove_task(dca_id.clone())?;
        let create_task_msg = create_convert_task_internal(env, dca, dca_id, cron_cat, config)?;
        Response::new().add_messages(vec![remove_task_msg, create_task_msg])
    } else {
        Response::new()
    };
    Ok(app.tag_response(response, "update_dca"))
}

/// Remove existing dca, remove task from cron_cat
fn cancel_dca(deps: DepsMut, info: MessageInfo, app: DCAApp, dca_id: String) -> AppResult {
    app.admin.assert_admin(deps.as_ref(), &info.sender)?;

    DCA_LIST.remove(deps.storage, dca_id.clone());

    let cron_cat = app.cron_cat(deps.as_ref());
    let remove_task_msg = cron_cat.remove_task(dca_id)?;

    Ok(app.tag_response(Response::new().add_message(remove_task_msg), "cancel_dca"))
}

/// Execute swap if called my croncat manager
/// Refill task if needed
fn convert(deps: DepsMut, env: Env, info: MessageInfo, app: DCAApp, dca_id: String) -> AppResult {
    let config = CONFIG.load(deps.storage)?;
    let dca = DCA_LIST.load(deps.storage, dca_id.clone())?;

    let cron_cat = app.cron_cat(deps.as_ref());

    let manager_addr = cron_cat.query_manager_addr(env.contract.address.clone(), dca_id.clone())?;
    if manager_addr != info.sender {
        return Err(AppError::NotManagerConvert {});
    }
    let mut messages = vec![];

    // In case task running out of balance - refill it
    let task_balance = cron_cat
        .query_task_balance(env.contract.address, dca_id.clone())?
        .balance
        .unwrap();
    if task_balance.native_balance < config.refill_threshold {
        messages.push(
            cron_cat.refill_task(
                dca_id,
                AssetList::from(vec![Asset::native(
                    config.native_denom,
                    config.dca_creation_amount,
                )])
                .into(),
            )?,
        );
    }

    let offer_asset = OfferAsset {
        name: dca.source_asset.into(),
        amount: Uint128::new(100),
    };
    // TODO: remove dca on failed swap?
    // Or `stop_on_fail` should be enough
    messages.push(app.dex(deps.as_ref(), dca.dex).swap(
        offer_asset,
        dca.target_asset.into(),
        Some(Decimal::percent(30)),
        None,
    )?);
    Ok(app.tag_response(Response::new().add_messages(messages), "convert"))
}
