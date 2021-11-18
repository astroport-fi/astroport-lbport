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
    mock_env as mock_env_std, MockApi as MockApiStd, MockQuerier as MockQuerierStd,
    MockStorage as MockStorageStd,
};
use cosmwasm_std::{attr, to_binary, Addr, Coin, Uint128};
use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};

use astroport_lbp::asset::{Asset, AssetInfo, PairInfo, WeightedAssetInfo};
use astroport_lbp::pair::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use cw20::Cw20ExecuteMsg;
use std::time::{SystemTime, UNIX_EPOCH};

const OWNER: &str = "Owner";

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
        astroport_lbp_token::contract::execute,
        astroport_lbp_token::contract::instantiate,
        astroport_lbp_token::contract::query,
    ));

    app.store_code(terra_swap_token_contract)
}

fn store_pair_code(app: &mut App) -> u64 {
    let pair_contract = Box::new(
        ContractWrapper::new(
            astroport_lbp_pair::contract::execute,
            astroport_lbp_pair::contract::instantiate,
            astroport_lbp_pair::contract::query,
        )
        .with_reply(astroport_lbp_pair::contract::reply),
    );

    app.store_code(pair_contract)
}

fn instantiate_pair(app: &mut App) -> Addr {
    let token_code_id = store_token_code(app);
    let pair_code_id = store_pair_code(app);

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
        start_time,
        end_time,
        description: None,
    };

    app.instantiate_contract(
        pair_code_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        "Astroport Pair",
        None,
    )
    .unwrap()
}

#[test]
fn multi_initialize() {
    let mut app = mock_app();

    let pair_instance = instantiate_pair(&mut app);

    let res: PairInfo = app
        .wrap()
        .query_wasm_smart(pair_instance.clone(), &QueryMsg::Pair {})
        .unwrap();

    assert_eq!("Contract #0", res.contract_addr);
    assert_eq!("Contract #1", res.liquidity_token);
}

#[test]
fn provide_and_withdraw_liquidity() {
    let mut app = mock_app();
    let alice_address = Addr::unchecked("alice");

    let pair_instance = instantiate_pair(&mut app);

    let pair_info: PairInfo = app
        .wrap()
        .query_wasm_smart(pair_instance.clone(), &QueryMsg::Pair {})
        .unwrap();

    // Set alice balances
    app.init_bank_balance(
        &alice_address,
        vec![
            Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(200u128),
            },
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(200u128),
            },
        ],
    )
    .unwrap();

    // Provide liquidity
    let (msg, coins) = provide_liquidity_msg(Uint128::new(100), Uint128::new(100));
    let res = app
        .execute_contract(alice_address.clone(), pair_instance.clone(), &msg, &coins)
        .unwrap();

    assert_eq!(
        res.events[1].attributes[1],
        attr("action", "provide_liquidity")
    );
    assert_eq!(
        res.events[1].attributes[2],
        attr("assets", "100uusd, 100uluna")
    );
    assert_eq!(
        res.events[1].attributes[3],
        attr("share", 100u128.to_string())
    );
    assert_eq!(res.events[3].attributes[1], attr("action", "mint"));
    assert_eq!(res.events[3].attributes[2], attr("to", "alice"));
    assert_eq!(res.events[3].attributes[3], attr("amount", 100.to_string()));

    // Check withdraw
    for n in vec![0, pair_info.end_time * 2] {
        app.update_block(|b| b.time = b.time.plus_seconds(n));

        // Withdraw liquidity
        let res = app
            .execute_contract(
                alice_address.clone(),
                pair_info.liquidity_token.clone(),
                &Cw20ExecuteMsg::Send {
                    contract: pair_instance.to_string(),
                    amount: Uint128::new(50),
                    msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
                },
                &[],
            )
            .unwrap();

        assert_eq!(
            res.events[3].attributes[1],
            attr("action", "withdraw_liquidity")
        );
        assert_eq!(res.events[3].attributes[2], attr("withdrawn_share", "50"));
        assert_eq!(
            res.events[3].attributes[3],
            attr("refund_assets", "50uluna, 50uusd")
        );

        assert_eq!(res.events[4].attributes[0], attr("recipient", "alice"));
        assert_eq!(res.events[4].attributes[2], attr("amount", "50uluna"));

        assert_eq!(res.events[5].attributes[0], attr("recipient", "alice"));
        assert_eq!(res.events[5].attributes[2], attr("amount", "50uusd"));
    }

    // No more liquidity to withdraw. Should return error
    app.execute_contract(
        alice_address.clone(),
        pair_info.liquidity_token.clone(),
        &Cw20ExecuteMsg::Send {
            contract: pair_instance.to_string(),
            amount: Uint128::new(1),
            msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        },
        &[],
    )
    .unwrap_err();
}

fn provide_liquidity_msg(uusd_amount: Uint128, uluna_amount: Uint128) -> (ExecuteMsg, [Coin; 2]) {
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: uusd_amount.clone(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: uluna_amount.clone(),
            },
        ],
        slippage_tolerance: None,
    };

    let coins = [
        Coin {
            denom: "uusd".to_string(),
            amount: uusd_amount.clone(),
        },
        Coin {
            denom: "uluna".to_string(),
            amount: uluna_amount.clone(),
        },
    ];

    (msg, coins)
}
