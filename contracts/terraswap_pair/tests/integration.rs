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

use cosmwasm_std::{
    testing::{mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Coin, Uint128,
};

use cw20::{Cw20Coin, Cw20ExecuteMsg};
use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};

use std::time::{SystemTime, UNIX_EPOCH};
use terraswap::asset::{AssetInfo, PairInfo, WeightedAssetInfo};
use terraswap::pair::{InstantiateMsg, QueryMsg};
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

const OWNER: &str = "Owner";
const TOKEN_INITIAL_AMOUNT: Uint128 = Uint128::new(1000000_00000);

fn mock_app() -> App {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();
    let terra_mock_querier = TerraMockQuerier::new(MockQuerier::new(&[]));

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

fn instantiate_token(app: &mut App, token_code_id: u64, name: &str) -> Addr {
    let name = String::from(name);

    let msg = TokenInstantiateMsg {
        name: name.clone(),
        symbol: name.clone(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: OWNER.to_string(),
            amount: TOKEN_INITIAL_AMOUNT,
        }],
        mint: None,
        init_hook: None,
    };

    app.instantiate_contract(token_code_id, Addr::unchecked(OWNER), &msg, &[], name, None)
        .unwrap()
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

    let owner = Addr::unchecked(OWNER);
    let token_code_id = store_token_code(&mut app);

    app.init_bank_balance(
        &owner.clone(),
        vec![
            Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(200_00000),
            },
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(200_00000),
            },
        ],
    )
    .unwrap();

    //let lp_token_instance = instantiate_token(&mut app, token_code_id, "uluna-uusd");

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
    assert_eq!(start_time, res.start_time);

    let msg_token_increase = Cw20ExecuteMsg::IncreaseAllowance {
        spender: pair_instance.to_string(),
        expires: None,
        amount: TOKEN_INITIAL_AMOUNT + TOKEN_INITIAL_AMOUNT,
    };

    // app.execute_contract(
    //     owner.clone(),
    //     lp_token_instance.clone(),
    //     &msg_token_increase,
    //     &[],
    // )
    // .unwrap();
}
