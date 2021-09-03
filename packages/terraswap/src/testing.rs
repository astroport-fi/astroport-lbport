use crate::asset::{Asset, AssetInfo, PairInfo, WeightedAssetInfo};
use crate::mock_querier::mock_dependencies;
use crate::querier::{
    query_all_balances, query_balance, query_pair_info, query_supply, query_token_balance,
};

use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{to_binary, BankMsg, Coin, CosmosMsg, Decimal, Addr, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn token_balance_querier() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
    )]);

    deps.querier.with_cw20_query_handler();
    assert_eq!(
        Uint128::from(123u128),
        query_token_balance(
            deps.as_ref(),
            &Addr::unchecked("liquidity0000"),
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
        )
        .unwrap()
    );
    deps.querier.with_default_query_handler()
}

#[test]
fn balance_querier() {
    let deps = mock_dependencies(
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );

    assert_eq!(
        query_balance(
            deps.as_ref(),
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            "uusd".to_string()
        )
        .unwrap(),
        Uint128::from(200u128)
    );
}

#[test]
fn all_balances_querier() {
    let deps = mock_dependencies(
        &[
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            },
        ],
    );

    assert_eq!(
        query_all_balances(deps.as_ref(), &Addr::unchecked(MOCK_CONTRACT_ADDR),).unwrap(),
        vec![
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            }
        ]
    );
}

#[test]
fn supply_querier() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    deps.querier.with_cw20_query_handler();

    assert_eq!(
        query_supply(deps.as_ref(), Addr::unchecked("liquidity0000")).unwrap(),
        Uint128::from(492u128)
    )
}

#[test]
fn test_asset_info() {
    let token_info: AssetInfo = AssetInfo::Token {
        contract_addr: Addr::unchecked("asset0000"),
    };
    let native_token_info: AssetInfo = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };
    
    assert_eq!(false, token_info.equal(&native_token_info));
    
    assert_eq!(
        false,
        token_info.equal(&AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0001"),
        })
    );
    
    assert_eq!(
        true,
        token_info.equal(&AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0000"),
        })
    );
    
    assert_eq!(true, native_token_info.is_native_token());
    assert_eq!(false, token_info.is_native_token());
    
    let mut deps = mock_dependencies(
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(123u128),
        }],
    );
    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);
    
    assert_eq!(
        native_token_info
            .query_pool(deps.as_ref(), &Addr::unchecked(MOCK_CONTRACT_ADDR))
            .unwrap(),
        Uint128::from(123u128)
    );
    
    deps.querier.with_cw20_query_handler();
    
    assert_eq!(
        token_info
            .query_pool(deps.as_ref(), &Addr::unchecked(MOCK_CONTRACT_ADDR))
            .unwrap(),
        Uint128::from(123u128)
    );
}

#[test]
fn test_asset() {
    let mut deps = mock_dependencies(
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(123u128),
        }],
    );

    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0000"),
        },
    };

    let native_token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    };

    assert_eq!(token_asset.compute_tax(deps.as_ref()).unwrap(), Uint128::zero());
    assert_eq!(
        native_token_asset.compute_tax(deps.as_ref()).unwrap(),
        Uint128::from(1220u128)
    );

    assert_eq!(
        native_token_asset.deduct_tax(deps.as_ref()).unwrap(),
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(121903u128),
        }
    );

    assert_eq!(
        token_asset
            .into_msg(
                deps.as_ref(),
                Addr::unchecked("asset0000"),
                Addr::unchecked("addr0000")
            )
            .unwrap(),
        CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(123123u128),
                })
                .unwrap(),
                funds: vec![],
            })
    );

    assert_eq!(
        native_token_asset
            .into_msg(
                deps.as_ref(),
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Addr::unchecked("addr0000")
            )
            .unwrap(),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(121903u128),
            }]
        })
    );
}

#[test]
fn query_terraswap_pair_contract() {
    let mut deps = mock_dependencies(&[]);
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    deps.querier.with_terraswap_pairs(&[(
        &"asset0000uusd".to_string(),
        &PairInfo {
            asset_infos: [
                WeightedAssetInfo {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("asset0000"),
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
            contract_addr: Addr::unchecked("pair0000"),
            liquidity_token: Addr::unchecked("liquidity0000"),
            start_time,
            end_time,
            description: None,
        },
    )]);

    let pair_info: PairInfo = query_pair_info(
        deps.as_ref(),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &[
            AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ],
    )
    .unwrap();

    assert_eq!(pair_info.contract_addr, Addr::unchecked("pair0000"),);
    assert_eq!(pair_info.liquidity_token, Addr::unchecked("liquidity0000"),);
}
