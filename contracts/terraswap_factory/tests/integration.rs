//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests as follows:
//! 1. Copy them over verbatim
//! 2. Then change
//!      let mut deps = mock_dependencies(20, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)

use cosmwasm_std::testing::{
    mock_env as mock_env_std, mock_info, MockApi as MockApi_std, MockQuerier as MockQuerier_std,
    MockStorage as MockStorage_std,
};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Coin, ContractResult, CosmosMsg, Response, SubMsg, Uint128,
    WasmMsg,
};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_backend_with_balances, mock_env, query, MockApi, MockQuerier,
    MockStorage,
};
use cosmwasm_vm::{Instance, InstanceOptions};

use std::time::{SystemTime, UNIX_EPOCH};
use terraswap::asset::{AssetInfo, WeightedAssetInfo};
use terraswap::factory::{ConfigResponse, ExecuteMsg, FactoryPairInfo, InstantiateMsg, QueryMsg};

use terraswap::pair::InstantiateMsg as PairInstantiateMsg;

use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};

// This line will test the output of cargo wasm
static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/terraswap_factory.wasm");
// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

const DEFAULT_GAS_LIMIT: u64 = 500_000;

pub fn mock_instance(
    wasm: &[u8],
    contract_balance: &[(&str, &[Coin])],
) -> Instance<MockApi, MockStorage, MockQuerier> {
    // TODO: check_wasm is not exported from cosmwasm_vm
    // let terra_features = features_from_csv("staking,terra");
    // check_wasm(wasm, &terra_features).unwrap();
    let backend = mock_backend_with_balances(contract_balance);
    Instance::from_code(
        wasm,
        backend,
        InstanceOptions {
            gas_limit: DEFAULT_GAS_LIMIT,
            print_debug: false,
        },
        None,
    )
    .unwrap()
}

#[test]
fn proper_initialization() {
    let mut deps = mock_instance(WASM, &[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: "owner0000".to_string(),
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res: Response = instantiate(&mut deps, env.clone(), info, msg).unwrap();

    let query_res = query(&mut deps, env, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!(Addr::unchecked("owner0000"), config_res.owner);
}

#[test]
fn update_config() {
    let mut deps = mock_instance(WASM, &[]);
    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: "owner0000".to_string(),
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res: Response = instantiate(&mut deps, env, info, msg).unwrap();

    // update owner
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(Addr::unchecked("addr0001")),
        pair_code_id: None,
        token_code_id: None,
    };

    let res: Response = execute(&mut deps, env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(&mut deps, env, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!(Addr::unchecked("addr0001"), config_res.owner);

    // update left items
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: Some(100u64),
        token_code_id: Some(200u64),
    };

    let res: Response = execute(&mut deps, env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(&mut deps, env, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(100u64, config_res.pair_code_id);
    assert_eq!(Addr::unchecked("addr0001"), config_res.owner);

    // Unauthorzied err
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: None,
        token_code_id: None,
    };

    let res: ContractResult<Response> = execute(&mut deps, env, info, msg);
    assert_eq!(res.unwrap_err(), "Unauthorized");
}

fn mock_app() -> App {
    let env = mock_env_std();
    let api = MockApi_std::default();
    let bank = BankKeeper::new();
    let storage = MockStorage_std::new();
    let terra_mock_querier = TerraMockQuerier::new(MockQuerier_std::new(&[]));

    App::new(api, env.block, bank, storage, terra_mock_querier)
}

fn store_factory_code(app: &mut App) -> u64 {
    let pair_contract = Box::new(
        ContractWrapper::new(
            terraswap_factory::contract::execute,
            terraswap_factory::contract::instantiate,
            terraswap_factory::contract::query,
        )
        .with_reply(terraswap_factory::contract::reply),
    );

    app.store_code(pair_contract)
}

fn store_pair_code(app: &mut App) -> u64 {
    let pair_contract = Box::new(
        ContractWrapper::new(
            terraswap_pair::contract::execute,
            terraswap_pair::contract::instantiate,
            terraswap_pair::contract::query,
        )
        .with_reply(terraswap_pair::contract::reply),
    );

    app.store_code(pair_contract)
}

fn store_token_code(app: &mut App) -> u64 {
    let terra_swap_token_contract = Box::new(ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    ));

    app.store_code(terra_swap_token_contract)
}

#[test]
fn create_and_register_pair_with_reply() {
    let mut app = mock_app();

    let factory_code_id = store_factory_code(&mut app);
    let pair_code_id = store_pair_code(&mut app);
    let token_code_id = store_token_code(&mut app);

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;
    let owner = "owner0000";

    let msg = InstantiateMsg {
        pair_code_id,
        token_code_id,
        owner: owner.to_string(),
        init_hook: None,
    };

    // we can just call .unwrap() to assert this was a success
    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            Addr::unchecked(owner),
            &msg,
            &[],
            "TerraSwapFactory",
            None,
        )
        .unwrap();

    let asset_infos = [
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::new(1),
            end_weight: Uint128::new(1),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            start_weight: Uint128::new(1),
            end_weight: Uint128::new(1),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        start_time,
        end_time,
        init_hook: None,
        description: Some(String::from("description")),
    };

    app.execute_contract(
        Addr::unchecked("addr0000"),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap();

    let res: FactoryPairInfo = app
        .wrap()
        .query_wasm_smart(
            factory_instance.clone(),
            &QueryMsg::Pair {
                asset_infos: [asset_infos[0].info.clone(), asset_infos[1].info.clone()],
            },
        )
        .unwrap();
    assert_eq!("Contract #1", res.contract_addr.to_string());
}

#[test]
fn create_pair() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;
    let mut deps = mock_instance(WASM, &[]);

    let owner = "owner0000";

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: owner.to_string(),
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res: Response = instantiate(&mut deps, env, info, msg).unwrap();

    let asset_infos = [
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::new(1),
            end_weight: Uint128::new(1),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            start_weight: Uint128::new(1),
            end_weight: Uint128::new(1),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        start_time,
        end_time,
        init_hook: None,
        description: Some(String::from("description")),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res: Response = execute(&mut deps, env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "asset0000-asset0001")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
                    init_hook: None,
                    start_time,
                    end_time,
                    description: Some(String::from("description")),
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: String::from("TerraSwap pair"),
                admin: Some(owner.to_string()),
            }),
            0
        )]
    );
}
