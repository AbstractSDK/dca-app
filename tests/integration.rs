use std::cell::RefCell;
use std::rc::Rc;

use abstract_core::objects::UncheckedContractEntry;
use abstract_core::{app::BaseInstantiateMsg, objects::gov_type::GovernanceDetails};
use abstract_dca_app::msg::Frequency;
use abstract_dca_app::state::Config;
use abstract_dca_app::{
    contract::{DCA_APP_ID, DCA_APP_VERSION},
    msg::{AppInstantiateMsg, ConfigResponse, InstantiateMsg},
    *,
};
use abstract_dex_adapter::msg::DexInstantiateMsg;
use abstract_dex_adapter::EXCHANGE;
use abstract_interface::{Abstract, AbstractAccount, AppDeployer, VCExecFns, *};
use croncat_app::contract::CRONCAT_ID;
use croncat_app::{CroncatApp, CRON_CAT_FACTORY};
use croncat_integration_testing::test_helpers::set_up_croncat_contracts;
use croncat_integration_testing::DENOM;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, deploy::Deploy, prelude::*};

use cosmwasm_std::{coin, coins, Addr, Decimal, Uint128};
use wyndex_bundle::{EUR, USD};

// consts for testing
const ADMIN: &str = "admin";
const WYNDEX_WITHOUT_CHAIN: &str = "wyndex";

/// Set up the test environment with the contract installed
fn setup() -> anyhow::Result<(AbstractAccount<Mock>, Abstract<Mock>, DCAApp<Mock>)> {
    // Create a sender
    let sender = Addr::unchecked(ADMIN);
    // Create the mock
    let mut mock = Mock::new(&sender);
    let croncat = set_up_croncat_contracts(None);
    mock.app = Rc::new(RefCell::new(croncat.app));

    // Construct the counter interface
    let mut contract = DCAApp::new(DCA_APP_ID, mock.clone());

    // Deploy Abstract to the mock
    let abstr_deployment = Abstract::deploy_on(mock.clone(), Empty {})?;
    let _wyndex = wyndex_bundle::WynDex::deploy_on(mock.clone(), Empty {})?;
    let dex_adapter = abstract_dex_adapter::interface::DexAdapter::new(EXCHANGE, mock.clone());

    dex_adapter.deploy(
        abstract_dex_adapter::contract::CONTRACT_VERSION.parse()?,
        DexInstantiateMsg {
            swap_fee: Decimal::percent(1),
            recipient_account: 0,
        },
    )?;

    let mut croncat_contract = CroncatApp::new(CRONCAT_ID, mock.clone());
    // Create account for croncat namespace
    abstr_deployment
        .account_factory
        .create_default_account(GovernanceDetails::Monarchy {
            monarch: ADMIN.to_string(),
        })?;
    abstr_deployment
        .version_control
        .claim_namespaces(1, vec!["croncat".to_string()])?;
    croncat_contract.deploy(croncat_app::contract::CRONCAT_MODULE_VERSION.parse()?)?;

    // Register factory entry
    let factory_entry = UncheckedContractEntry::try_from(CRON_CAT_FACTORY.to_owned())?;
    abstr_deployment.ans_host.execute(
        &abstract_core::ans_host::ExecuteMsg::UpdateContractAddresses {
            to_add: vec![(factory_entry, croncat.factory.to_string())],
            to_remove: vec![],
        },
        None,
    )?;

    // Create a new account to install the app onto
    let account =
        abstr_deployment
            .account_factory
            .create_default_account(GovernanceDetails::Monarchy {
                monarch: ADMIN.to_string(),
            })?;
    account.install_module(
        CRONCAT_ID,
        &croncat_app::msg::InstantiateMsg {
            base: BaseInstantiateMsg {
                ans_host_address: abstr_deployment.ans_host.addr_str()?,
            },
            module: croncat_app::msg::AppInstantiateMsg {},
        },
        None,
    )?;
    let module_addr = account.manager.module_info(CRONCAT_ID)?.unwrap().address;
    croncat_contract.set_address(&module_addr);
    let manager_addr = account.manager.address()?;
    croncat_contract.set_sender(&manager_addr);
    mock.set_balance(&account.proxy.address()?, coins(500_000, DENOM))?;

    contract.deploy(DCA_APP_VERSION.parse()?)?;

    account.install_module(
        DCA_APP_ID,
        &InstantiateMsg {
            base: BaseInstantiateMsg {
                ans_host_address: abstr_deployment.ans_host.addr_str()?,
            },
            module: AppInstantiateMsg {
                native_denom: DENOM.to_owned(),
                dca_creation_amount: Uint128::new(5_000_000),
                refill_threshold: Uint128::new(1_000_000),
            },
        },
        None,
    )?;

    let module_addr = account.manager.module_info(DCA_APP_ID)?.unwrap().address;
    contract.set_address(&module_addr);
    let manager_addr = account.manager.address()?;
    contract.set_sender(&manager_addr);
    mock.set_balance(&account.proxy.address()?, vec![coin(10_000, EUR)])?;

    Ok((account, abstr_deployment, contract))
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    // Set up the environment and contract
    let (_account, _abstr, app) = setup()?;

    let config: ConfigResponse = app.config()?;
    assert_eq!(
        config,
        ConfigResponse {
            config: Config {
                native_denom: DENOM.to_owned(),
                dca_creation_amount: Uint128::new(5_000_000),
                refill_threshold: Uint128::new(1_000_000)
            }
        }
    );
    let res = app
        .create_dca(
            USD.to_owned(),
            Frequency::EveryNBlocks(1),
            WYNDEX_WITHOUT_CHAIN.to_owned(),
            EUR.to_owned(),
        )
        .unwrap();
    Ok(())
}
