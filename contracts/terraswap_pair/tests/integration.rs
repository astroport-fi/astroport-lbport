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
    mock_env as mock_env_std, mock_info, MockApi as MockApiStd, MockQuerier as MockQuerierStd,
    MockStorage as MockStorageStd,
};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Coin, CosmosMsg, Response, SubMsg, Uint128, WasmMsg,
};
use cosmwasm_vm::testing::{
    instantiate, mock_backend_with_balances, mock_env, query, MockApi, MockQuerier, MockStorage,
    MOCK_CONTRACT_ADDR,
};
use cosmwasm_vm::{Instance, InstanceOptions};
use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};

use cw20::MinterResponse;
use std::time::{SystemTime, UNIX_EPOCH};
use terraswap::asset::{AssetInfo, PairInfo, WeightedAssetInfo};
use terraswap::pair::{InstantiateMsg, QueryMsg};
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

// This line will test the output of cargo wasm
static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/terraswap_pair.wasm");
// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

const DEFAULT_GAS_LIMIT: u64 = 500_000;
const OWNER: &str = "Owner";

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
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let asset_infos = [
        WeightedAssetInfo {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            start_weight: Uint128::from(1u128),
            end_weight: Uint128::from(1u128),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::from(1u128),
            end_weight: Uint128::from(1u128),
        },
    ];

    let msg = InstantiateMsg {
        asset_infos: asset_infos.clone(),
        token_code_id: 10u64,
        init_hook: None,
        start_time: start_time.clone(),
        end_time: end_time.clone(),
        description: Some(String::from("description")),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let res: Response = instantiate(&mut deps, env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                msg: to_binary(&TokenInstantiateMsg {
                    name: "terraswap liquidity token".to_string(),
                    symbol: "uLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                    init_hook: None,
                })
                .unwrap(),
                code_id: 10u64,
                funds: vec![],
                admin: None,
                label: String::from("terraswap liquidity token"),
            }),
            1
        )]
    );

    // it worked, let's query the state
    let res = query(&mut deps, env, QueryMsg::Pair {}).unwrap();
    let pair_info: PairInfo = from_binary(&res).unwrap();

    assert_eq!(MOCK_CONTRACT_ADDR, pair_info.contract_addr.as_str());
    // why pair_info.liquidity_token is empty ?
    assert_eq!("liquidity0000", pair_info.liquidity_token.as_str());
    assert_eq!(asset_infos, pair_info.asset_infos);
    assert_eq!(start_time, pair_info.start_time);
    assert_eq!(end_time, pair_info.end_time);
    assert_eq!("description", pair_info.description.unwrap().as_str());
}

fn mock_app() -> App {
    let env = mock_env_std();
    let api = MockApiStd::default();
    let bank = BankKeeper::new();
    let storage = MockStorageStd::new();
    let terra_mock_querier = TerraMockQuerier::new(MockQuerierStd::new(&[]));

    App::new(api, env.block, bank, storage, terra_mock_querier)
}

fn store_token_code(app: &mut App) -> u64 {
    let terra_swap_token_contract = Box::new(ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    ));

    app.store_code(terra_swap_token_contract)
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

fn instantiate_pair(app: &mut App, pair_code_id: u64, msg: &InstantiateMsg, name: &str) -> Addr {
    let name = String::from(name);

    app.instantiate_contract(
        pair_code_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        name.clone(),
        None,
    )
    .unwrap()
}

#[test]
fn multi_initialize() {
    let mut app = mock_app();

    let token_code_id = store_token_code(&mut app);
    let pair_code_id = store_pair_code(&mut app);

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let msg = InstantiateMsg {
        asset_infos: [
            WeightedAssetInfo {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
            WeightedAssetInfo {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
        ],
        token_code_id,
        init_hook: None,
        start_time,
        end_time,
        description: None,
    };

    let pair_instance = instantiate_pair(&mut app, pair_code_id, &msg, "TerraSwapPair");

    let res: PairInfo = app
        .wrap()
        .query_wasm_smart(pair_instance.clone(), &QueryMsg::Pair {})
        .unwrap();
    assert_eq!("Contract #0", res.contract_addr);
    assert_eq!("Contract #1", res.liquidity_token);
}
