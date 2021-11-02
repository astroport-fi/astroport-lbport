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
    mock_env as mock_env_std, mock_info, MockApi as MockApi_std, MockStorage as MockStorage_std,
};
use cosmwasm_std::{from_binary, to_binary, Addr, Coin, Response, Uint128};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_backend_with_balances, mock_env, query, MockApi, MockQuerier,
    MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_vm::{Instance, InstanceOptions};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_multi_test::{App, BankKeeper, ContractWrapper, Executor};
use std::time::{SystemTime, UNIX_EPOCH};
use terraswap::asset::{Asset, AssetInfo, PairInfo, WeightedAsset, WeightedAssetInfo};
use terraswap::pair::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolResponse, QueryMsg};
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;
// This line will test the output of cargo wasm
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/terraswap_pair.wasm");
static WASM: &[u8] = include_bytes!("../../../artifacts/terraswap_pair.wasm");
// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

const DEFAULT_GAS_LIMIT: u64 = 500_000_000;

const OWNER: &str = "Owner";
const TOKEN_INITIAL_AMOUNT: Uint128 = Uint128::new(1000000_00000);

fn mock_app() -> App {
    let env = mock_env_std();
    let api = MockApi_std::default();
    let bank = BankKeeper {};

    App::new(api, env.block, bank, MockStorage_std::new())
}

fn store_token_code(app: &mut App) -> u64 {
    let terra_swap_token_contract = Box::new(ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    ));

    app.store_code(terra_swap_token_contract)
}

fn instantiate_token(app: &mut App, owner: Addr, token_code_id: u64) -> Addr {
    let token_name = "TerraSwapToken";

    let init_msg = TokenInstantiateMsg {
        name: token_name.to_string(),
        symbol: "TerraT".to_string(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: OWNER.to_string(),
            amount: TOKEN_INITIAL_AMOUNT,
        }],
        mint: None,
        init_hook: None,
    };

    let token_instance = app
        .instantiate_contract(token_code_id, owner, &init_msg, &[], token_name, None)
        .unwrap();

    return token_instance;
}

fn store_pair_code(app: &mut App) -> u64 {
    let pair_contract = Box::new(ContractWrapper::new(
        terraswap_pair::contract::execute,
        terraswap_pair::contract::instantiate,
        terraswap_pair::contract::query,
    ));

    app.store_code(pair_contract)
}

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
            print_debug: true,
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

    let msg = InstantiateMsg {
        asset_infos: [
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
        ],
        token_code_id: 10u64,
        init_hook: None,
        start_time,
        end_time,
        description: Some(String::from("description")),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res: Response = instantiate(&mut deps, env.clone(), info, msg).unwrap();

    // cannot change it after post intialization
    let msg = ExecuteMsg::PostInitialize {};
    let info = mock_info("liquidity0000", &[]);
    let _res: Response = execute(&mut deps, env.clone(), info, msg).unwrap();

    // it worked, let's query the state
    let res = query(&mut deps, env, QueryMsg::Pair {}).unwrap();
    let pair_info: PairInfo = from_binary(&res).unwrap();
    assert_eq!(MOCK_CONTRACT_ADDR, pair_info.contract_addr.as_str());
    assert_eq!(
        [
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
            }
        ],
        pair_info.asset_infos
    );

    assert_eq!("liquidity0000", pair_info.liquidity_token.as_str());
    assert_eq!("description", pair_info.description.unwrap());
}

#[test]
fn provide_liquidity_cw20_hook() {
    let mut app = mock_app();
    let owner = Addr::unchecked(OWNER);
    let token_code_id = store_token_code(&mut app);

    app.init_bank_balance(
        &owner.clone(),
        vec![Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(200_00000),
        }],
    )
    .unwrap();

    let token_instance = instantiate_token(&mut app, owner.clone(), token_code_id);

    let pair_code_id = store_pair_code(&mut app);

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let init_msg = InstantiateMsg {
        asset_infos: [
            WeightedAssetInfo {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
            WeightedAssetInfo {
                info: AssetInfo::Token {
                    contract_addr: token_instance.clone(),
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

    let pair_instance = app
        .instantiate_contract(
            pair_code_id,
            owner.clone(),
            &init_msg,
            &[],
            "TerraSwapPair",
            None,
        )
        .unwrap();

    let res: PairInfo = app
        .wrap()
        .query_wasm_smart(pair_instance.clone(), &QueryMsg::Pair {})
        .unwrap();
    assert_eq!(start_time, res.start_time);

    let msg_token_increase = Cw20ExecuteMsg::IncreaseAllowance {
        spender: pair_instance.to_string(),
        expires: None,
        amount: TOKEN_INITIAL_AMOUNT + TOKEN_INITIAL_AMOUNT,
    };

    app.execute_contract(
        owner.clone(),
        token_instance.clone(),
        &msg_token_increase,
        &[],
    )
    .unwrap();

    let msg_provide = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_instance.clone(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
    };

    // direct provide liquidity
    app.execute_contract(
        owner.clone(),
        pair_instance.clone(),
        &msg_provide,
        &[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::from(100u128),
        }],
    )
    .unwrap();

    let res: PoolResponse = app
        .wrap()
        .query_wasm_smart(pair_instance.clone(), &QueryMsg::Pool {})
        .unwrap();
    assert_eq!(Uint128::new(100), res.total_share);
    assert_eq!(
        res.assets,
        [
            WeightedAsset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::from(100u128),
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
            WeightedAsset {
                info: AssetInfo::Token {
                    contract_addr: token_instance.clone(),
                },
                amount: Uint128::from(100u128),
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
        ]
    );

    let msg_provide_liquidity_by_hook = Cw20ExecuteMsg::Send {
        contract: pair_instance.to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::ProvideLiquidity {
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(100u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_instance.clone(),
                    },
                    amount: Uint128::from(100u128),
                },
            ],
            slippage_tolerance: None,
        })
        .unwrap(),
    };

    // provide liquidity by Cw20HookMsg
    app.execute_contract(
        owner.clone(),
        token_instance.clone(),
        &msg_provide_liquidity_by_hook,
        &[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(100),
        }],
    )
    .unwrap();

    let res: PoolResponse = app
        .wrap()
        .query_wasm_smart(pair_instance.clone(), &QueryMsg::Pool {})
        .unwrap();
    assert_eq!(Uint128::new(200), res.clone().total_share);
    assert_eq!(
        res.assets,
        [
            WeightedAsset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::from(200u128),
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
            WeightedAsset {
                info: AssetInfo::Token {
                    contract_addr: token_instance.clone(),
                },
                amount: Uint128::from(200u128),
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
        ]
    );
}
