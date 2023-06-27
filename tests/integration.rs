use abstract_core::{app::BaseInstantiateMsg, objects::gov_type::GovernanceDetails};
use abstract_dca_app::state::Config;
use abstract_dca_app::{
    contract::{DCA_APP_ID, DCA_APP_VERSION},
    msg::{AppInstantiateMsg, ConfigResponse, InstantiateMsg},
    *,
};
use abstract_interface::{Abstract, AbstractAccount, AppDeployer, VCExecFns, *};
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, deploy::Deploy, prelude::*};

use cosmwasm_std::{Addr, Uint128};

// consts for testing
const ADMIN: &str = "admin";
const DENOM: &str = "abstr";

/// Set up the test environment with the contract installed
fn setup() -> anyhow::Result<(AbstractAccount<Mock>, Abstract<Mock>, DCAApp<Mock>)> {
    // Create a sender
    let sender = Addr::unchecked(ADMIN);
    // Create the mock
    let mock = Mock::new(&sender);

    // Construct the counter interface
    let contract = DCAApp::new(DCA_APP_ID, mock.clone());

    // Deploy Abstract to the mock
    let abstr_deployment = Abstract::deploy_on(mock, Empty {})?;

    // Create a new account to install the app onto
    let account =
        abstr_deployment
            .account_factory
            .create_default_account(GovernanceDetails::Monarchy {
                monarch: ADMIN.to_string(),
            })?;

    // claim the namespace so app can be deployed
    abstr_deployment
        .version_control
        .claim_namespaces(1, vec!["my-namespace".to_string()])?;

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

    let modules = account.manager.module_infos(None, None)?;
    contract.set_address(&modules.module_infos[1].address);

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
    Ok(())
}
