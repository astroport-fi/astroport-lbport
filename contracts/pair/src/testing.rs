use crate::contract::{
    assert_max_spread, compute_swap, execute, instantiate, query_pair_info, query_pool,
    query_reverse_simulation, query_simulation, reply,
};
use crate::mock_querier::mock_dependencies;
use proptest::prelude::*;

use crate::error::ContractError;
use crate::math::uint2dec;
use astroport_lbp::asset::{Asset, AssetInfo, PairInfo, WeightedAsset, WeightedAssetInfo};
use astroport_lbp::pair::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolResponse, ReverseSimulationResponse,
    SimulationResponse,
};
use astroport_lbp::token::InstantiateMsg as TokenInstantiateMsg;
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, BlockInfo, Coin, ContractResult, Decimal, DepsMut, Env, Reply,
    ReplyOn, Response, StdError, SubMsg, SubMsgExecutionResponse, Timestamp, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

const COMMISSION_AMOUNT: u128 = 15;
const COMMISSION_RATIO: u128 = 10000;
const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128);

fn mock_env_with_block_time(time: u64) -> Env {
    let mut env = mock_env();
    env.block = BlockInfo {
        height: 1,
        time: Timestamp::from_seconds(time),
        chain_id: "columbus".to_string(),
    };
    env
}

fn store_liquidity_token(deps: DepsMut) {
    // store liquidity token by name: liquidity0000

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    reply(deps, mock_env(), reply_msg).unwrap();
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

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
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: 10u64,
                msg: to_binary(&TokenInstantiateMsg {
                    name: "astroport-lbp liquidity token".to_string(),
                    symbol: "uLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None,
                    }),
                })
                .unwrap(),
                funds: vec![],
                label: String::from("astroport-lbp liquidity token"),
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Success
        },]
    );

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    // it worked, let's query the state
    let pair_info: PairInfo = query_pair_info(deps.as_ref()).unwrap();
    assert_eq!("liquidity0000", pair_info.liquidity_token.as_str());
    assert_eq!(
        pair_info.asset_infos,
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
            },
        ]
    );
    assert_eq!("description", pair_info.description.unwrap());
}

#[test]
fn provide_liquidity() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(200_000000000000000000u128),
    }]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
    )]);

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
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
        ],
        slippage_tolerance: None,
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100_000000000000000000u128),
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::from(100_000000000000000000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        mint_msg,
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(100_000000000000000000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // provide more liquidity 1:2, which is not propotional to 1:1,
    // then it must accept 1:1 and treat left amount as donation
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                200_000000000000000000u128 + 200_000000000000000000u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &Uint128::from(100_000000000000000000u128),
            )],
        ),
        (
            &"asset0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &Uint128::from(200_000000000000000000u128),
            )],
        ),
    ]);

    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(200_000000000000000000u128),
            },
        ],
        slippage_tolerance: None,
    };

    let env = mock_env_with_block_time(env.block.time.seconds() + 1000);
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200_000000000000000000u128),
        }],
    );

    // only accept 100, then 50 share will be generated with 100 * (100 / 200)
    let res: Response = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::from(100_000000000000000000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        mint_msg,
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(50_000000000000000000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // check wrong argument
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(50_000000000000000000u128),
            },
        ],
        slippage_tolerance: None,
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100_000000000000000000u128),
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::Std(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Native token balance missmatch between the argument and the transferred".to_string()
        ),
        _ => panic!("Must return generic error"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                100_000000000000000000u128 + 100_000000000000000000u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &Uint128::from(100_000000000000000000u128),
            )],
        ),
        (
            &"asset0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &Uint128::from(100_000000000000000000u128),
            )],
        ),
    ]);

    // failed because the price is under slippage_tolerance
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(98_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
    };

    let env = mock_env_with_block_time(env.block.time.seconds() + 1000);
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100_000000000000000000u128),
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::Std(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Operation exceeds max splippage tolerance")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100_000000000000000000u128 + 98_000000000000000000u128 /* user deposit must be pre-applied */),
        }],
    )]);

    // failed because the price is under slippage_tolerance
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(98_000000000000000000u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
    };

    let env = mock_env_with_block_time(env.block.time.seconds() + 1000);
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(98_000000000000000000u128),
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::Std(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Operation exceeds max splippage tolerance")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                100_000000000000000000u128 + 100_000000000000000000u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    // successfully provides
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(99_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
    };

    let env = mock_env_with_block_time(env.block.time.seconds() + 1000);
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100_000000000000000000u128),
        }],
    );
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100_000000000000000000u128 + 99_000000000000000000u128 /* user deposit must be pre-applied */),
        }],
    )]);

    // successfully provides
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: Uint128::from(100_000000000000000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(99_000000000000000000u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
    };

    let env = mock_env_with_block_time(env.block.time.seconds() + 1000);
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(99_000000000000000000u128),
        }],
    );
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
}

#[test]
fn withdraw_liquidity() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&"addr0000".to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
    ]);

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
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    // withdraw liquidity
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        amount: Uint128::from(100u128),
    });

    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let attribute_withdrawn_share = res.attributes.get(1).expect("no attribute");
    let attribute_refund_assets = res.attributes.get(2).expect("no attribute");
    let msg_refund_0 = res.messages.get(0).expect("no message");
    let msg_refund_1 = res.messages.get(1).expect("no message");
    let msg_burn_liquidity = res.messages.get(2).expect("no message");
    assert_eq!(
        msg_refund_0,
        &SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(100u128),
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        msg_refund_1,
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        msg_burn_liquidity,
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    assert_eq!(
        attribute_withdrawn_share,
        &attr("withdrawn_share", 100u128.to_string())
    );
    assert_eq!(
        attribute_refund_assets,
        &attr("refund_assets", "100uusd, 100asset0000")
    );
}

#[test]
fn try_native_to_token() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;
    let total_share = Uint128::from(30000000000u128);
    let asset_pool_amount = Uint128::from(20000000000u128);
    let collateral_pool_amount = Uint128::from(30000000000u128);
    let price = Decimal::from_ratio(collateral_pool_amount, asset_pool_amount);
    let exchange_rate = Decimal::from(Decimal256::one() / Decimal256::from(price));
    let offer_amount = Uint128::from(1500000000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount, /* user deposit must be pre-applied */
    }]);

    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
    ]);

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
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    // normal swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env_with_block_time(start_time);
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let msg_transfer = res.messages.get(0).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // 952.380953 = 20000 - 20000 * 30000 / (30000 + 1500)
    let expected_ret_amount = Uint128::from(952_380_953u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_commission_amount =
        expected_ret_amount.multiply_ratio(COMMISSION_AMOUNT, COMMISSION_RATIO); // 0.15%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_commission_amount)
        .unwrap();
    let expected_tax_amount = Uint128::zero(); // no tax for token

    // check simulation res
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: collateral_pool_amount, /* user deposit must be pre-applied */
        }],
    )]);

    let simulation_res: SimulationResponse = query_simulation(
        deps.as_ref(),
        env.clone(),
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        start_time,
    )
    .unwrap();

    let amount_diff =
        expected_return_amount.u128() as u128 - simulation_res.return_amount.u128() as u128;
    let commission_diff =
        expected_commission_amount.u128() as u128 - simulation_res.commission_amount.u128() as u128;
    let spread_diff = simulation_res.spread_amount.u128() as u128 - expected_spread_amount.u128();

    let diff_tolerance = 10u128;

    assert_eq!(amount_diff < diff_tolerance, true);
    assert_eq!(commission_diff < diff_tolerance, true);
    assert_eq!(spread_diff < diff_tolerance, true);

    assert_eq!(String::from("1"), simulation_res.ask_weight);
    assert_eq!(String::from("1"), simulation_res.offer_weight);

    // check reverse simulation res
    let reverse_simulation_res: ReverseSimulationResponse = query_reverse_simulation(
        deps.as_ref(),
        env,
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: expected_return_amount,
        },
        start_time,
    )
    .unwrap();

    let offer_diff =
        reverse_simulation_res.offer_amount.u128() as u128 - offer_amount.u128() as u128;
    let commission_diff = reverse_simulation_res.commission_amount.u128() as u128
        - expected_commission_amount.u128() as u128;
    let spread_diff =
        reverse_simulation_res.spread_amount.u128() as u128 - expected_spread_amount.u128() as u128;

    assert_eq!(offer_diff < diff_tolerance, true);
    assert_eq!(commission_diff < diff_tolerance, true);
    assert_eq!(spread_diff < diff_tolerance, true);

    assert_eq!(String::from("1"), reverse_simulation_res.ask_weight);
    assert_eq!(String::from("1"), reverse_simulation_res.offer_weight);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "swap"),
            attr("offer_asset", "uusd"),
            attr("ask_asset", "asset0000"),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", simulation_res.return_amount.to_string()),
            attr("tax_amount", expected_tax_amount.to_string()),
            attr("spread_amount", simulation_res.spread_amount.to_string()),
            attr(
                "commission_amount",
                simulation_res.commission_amount.to_string()
            ),
        ]
    );

    assert_eq!(
        &SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(simulation_res.return_amount),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        },
        msg_transfer,
    );
}

#[test]
fn try_token_to_native() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;
    let total_share = Uint128::from(20000000000u128);
    let asset_pool_amount = Uint128::from(30000000000u128);
    let collateral_pool_amount = Uint128::from(20000000000u128);
    let price = Decimal::from_ratio(collateral_pool_amount, asset_pool_amount);
    let exchange_rate = price;
    let offer_amount = Uint128::from(1500000000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount,
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &(asset_pool_amount + offer_amount),
            )],
        ),
    ]);

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
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    // unauthorized access; can not execute swap directy for token swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env_with_block_time(env.block.time.seconds() + start_time);
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized { .. } => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // normal sell
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let env = mock_env_with_block_time(start_time);
    let info = mock_info("asset0000", &[]);

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let msg_transfer = res.messages.get(0).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // 952.380953 = 20000 - 20000 * 30000 / (30000 + 1500)
    let expected_ret_amount = Uint128::from(952_380_953u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_commission_amount =
        expected_ret_amount.multiply_ratio(COMMISSION_AMOUNT, COMMISSION_RATIO); // 0.15%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_commission_amount)
        .unwrap();
    let expected_tax_amount = std::cmp::min(
        Uint128::from(1000000u128),
        expected_return_amount
            .checked_sub(
                expected_return_amount
                    .multiply_ratio(Uint128::from(100u128), Uint128::from(101u128)),
            )
            .unwrap(),
    );
    // check simulation res
    // return asset token balance as normal
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &(asset_pool_amount))],
        ),
    ]);

    let simulation_res: SimulationResponse = query_simulation(
        deps.as_ref(),
        env.clone(),
        Asset {
            amount: offer_amount,
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
        },
        start_time,
    )
    .unwrap();

    let ret_diff =
        (expected_return_amount.u128() as i128 - simulation_res.return_amount.u128() as i128).abs();
    let commission_diff = (expected_commission_amount.u128() as i128
        - simulation_res.commission_amount.u128() as i128)
        .abs();
    let spread_diff =
        (expected_spread_amount.u128() as i128 - simulation_res.spread_amount.u128() as i128).abs();

    let diff_tolerance = 10i128;

    assert_eq!(ret_diff < diff_tolerance, true);
    assert_eq!(commission_diff < diff_tolerance, true);
    assert_eq!(spread_diff < diff_tolerance, true);

    // check reverse simulation res
    let reverse_simulation_res: ReverseSimulationResponse = query_reverse_simulation(
        deps.as_ref(),
        env,
        Asset {
            amount: expected_return_amount,
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
        start_time,
    )
    .unwrap();

    let offer_diff =
        (offer_amount.u128() as i128 - reverse_simulation_res.offer_amount.u128() as i128).abs();
    let commission_diff = (expected_commission_amount.u128() as i128
        - reverse_simulation_res.commission_amount.u128() as i128)
        .abs();
    let spread_diff = (expected_spread_amount.u128() as i128
        - reverse_simulation_res.spread_amount.u128() as i128)
        .abs();

    let diff_tolerance = 5i128;

    assert_eq!(offer_diff < diff_tolerance, true);
    assert_eq!(commission_diff < diff_tolerance, true);
    assert_eq!(spread_diff < diff_tolerance, true);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "swap"),
            attr("offer_asset", "asset0000"),
            attr("ask_asset", "uusd"),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", simulation_res.return_amount.to_string()),
            attr("tax_amount", expected_tax_amount.to_string()),
            attr("spread_amount", simulation_res.spread_amount.to_string()),
            attr("commission_amount", expected_commission_amount.to_string()),
        ]
    );

    assert_eq!(
        &SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: simulation_res
                        .return_amount
                        .checked_sub(expected_tax_amount)
                        .unwrap(),
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        },
        msg_transfer,
    );

    // failed due to non asset token contract try to execute sell
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let env = mock_env_with_block_time(start_time);
    let info = mock_info("liquidtity0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized { .. } => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_max_spread() {
    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Uint128::from(1200000000u128),
        Uint128::from(989999u128),
        Uint128::zero(),
    )
    .unwrap_err();

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Uint128::from(1200000000u128),
        Uint128::from(990000u128),
        Uint128::zero(),
    )
    .unwrap();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Uint128::zero(),
        Uint128::from(989999u128),
        Uint128::from(10001u128),
    )
    .unwrap_err();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Uint128::zero(),
        Uint128::from(990000u128),
        Uint128::from(10000u128),
    )
    .unwrap();
}

#[test]
fn test_spread() {
    let tkn_contract = Addr::unchecked("TKN");
    let tkn_amount = Uint128::from(50_000_000_u128 * DECIMAL_FRACTIONAL.u128());

    let usdc_contract = Addr::unchecked("USDC");
    let usdc_amount = Uint128::from(250_000_u128 * DECIMAL_FRACTIONAL.u128());

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &tkn_contract.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &tkn_amount)],
        ),
        (
            &usdc_contract.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &usdc_amount)],
        ),
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            WeightedAssetInfo {
                info: AssetInfo::Token {
                    contract_addr: tkn_contract.clone(),
                },
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
            WeightedAssetInfo {
                info: AssetInfo::Token {
                    contract_addr: usdc_contract.clone(),
                },
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
        ],
        token_code_id: 10u64,
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: tkn_contract.clone(),
                },
                amount: tkn_amount.clone(),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: usdc_contract.clone(),
                },
                amount: usdc_amount.clone(),
            },
        ],
        slippage_tolerance: None,
    };

    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Check balances
    let res: PoolResponse = query_pool(deps.as_ref(), env.clone()).unwrap();
    assert_eq!(
        res.assets[0].info,
        AssetInfo::Token {
            contract_addr: tkn_contract.clone()
        }
    );
    assert_eq!(res.assets[0].amount, tkn_amount);

    assert_eq!(
        res.assets[1].info,
        AssetInfo::Token {
            contract_addr: usdc_contract.clone()
        }
    );
    assert_eq!(res.assets[1].amount, usdc_amount);

    let simulation_res: SimulationResponse = query_simulation(
        deps.as_ref(),
        env,
        Asset {
            amount: Uint128::from(10_u128 * DECIMAL_FRACTIONAL.u128()),
            info: AssetInfo::Token {
                contract_addr: usdc_contract.clone(),
            },
        },
        start_time,
    )
    .unwrap();

    // Spot price: (ask_pool / ask_weight) / (offer_pool / offer_weight) * offer_amount
    // (50000000 / 1) / ( 250000 / 1) * 10 * DECIMAL_PRECISION  = 2000000000000
    let spot_price = Uint128::new(2000000000000);

    // return_amount: ask_pool * (1 - (offer_pool / (offer_pool + offer_amount)) ^ (offer_weight / ask_weight))
    // 50000000000000000 * (1 - (250000000000000 / 250010000000000)) = 1999920003199
    let return_amount = Uint128::new(1999920003199);
    let commission_amount: Uint128 = return_amount * Decimal::from_str("0.0015").unwrap();
    let return_amount_without_commission = return_amount - commission_amount;

    assert_eq!(
        simulation_res.return_amount,
        return_amount_without_commission
    );

    assert_eq!(simulation_res.spread_amount, spot_price - return_amount);
}

#[test]
fn test_deduct() {
    let mut deps = mock_dependencies(&[]);

    let tax_rate = Decimal::percent(2);
    let tax_cap = Uint128::from(1_000_000u128);
    deps.querier.with_tax(
        Decimal::percent(2),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let amount = Uint128::from(1000_000_000u128);
    let expected_after_amount = std::cmp::max(
        amount.checked_sub(amount * tax_rate).unwrap(),
        amount.checked_sub(tax_cap).unwrap(),
    );

    let after_amount = (Asset {
        info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        amount,
    })
    .deduct_tax(deps.as_ref())
    .unwrap();

    assert_eq!(expected_after_amount, after_amount.amount);
}

#[test]
fn test_query_pool() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;
    let total_share_amount = Uint128::from(111u128);
    let asset_0_amount = Uint128::from(222u128);
    let asset_1_amount = Uint128::from(333u128);
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: asset_0_amount,
    }]);

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_1_amount)],
        ),
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share_amount)],
        ),
    ]);

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
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // store liquidity token
    store_liquidity_token(deps.as_mut());

    let res: PoolResponse = query_pool(deps.as_ref(), env).unwrap();
    assert_eq!(
        res.assets,
        [
            WeightedAsset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: asset_0_amount,
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            },
            WeightedAsset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                amount: asset_1_amount,
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(1u128),
            }
        ]
    );
    assert_eq!(res.total_share, total_share_amount);
}

#[test]
fn test_weight_calculations() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 100;
    let total_share = Uint128::from(50_000_000_____000_000_000u128);
    let asset_pool_amount = Uint128::from(250_000_____000_000_000u128);
    let collateral_pool_amount = total_share.clone();

    let offer_amount = Uint128::from(1_000____000_000_000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount, /* user deposit must be pre-applied */
    }]);

    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            WeightedAssetInfo {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                start_weight: Uint128::from(1u128),
                end_weight: Uint128::from(30u128),
            },
            WeightedAssetInfo {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0000"),
                },
                start_weight: Uint128::from(49u128),
                end_weight: Uint128::from(20u128),
            },
        ],
        token_code_id: 10u64,
        start_time,
        end_time: Some(end_time),
        description: Some(String::from("description")),
        commission_rate: "0.0015".to_string(),
        collector_addr: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // check simulation res
    struct TestCase {
        expected_error: bool,
        start_time: u64,
        expected_ask_weight: String,
        expected_offer_weight: String,
    }

    let mut test_cases: Vec<TestCase> = Vec::new();
    test_cases.push(TestCase {
        expected_error: true,
        start_time: start_time - 1,
        expected_ask_weight: Default::default(),
        expected_offer_weight: Default::default(),
    });

    test_cases.push(TestCase {
        expected_error: true,
        start_time: end_time + 1,
        expected_ask_weight: Default::default(),
        expected_offer_weight: Default::default(),
    });

    test_cases.push(TestCase {
        expected_error: false,
        start_time: start_time,
        expected_ask_weight: String::from("49"),
        expected_offer_weight: String::from("1"),
    });
    test_cases.push(TestCase {
        expected_error: false,
        start_time: start_time + 50,
        expected_ask_weight: String::from("34.5"),
        expected_offer_weight: String::from("15.5"),
    });
    test_cases.push(TestCase {
        expected_error: false,
        start_time: start_time + 100,
        expected_ask_weight: String::from("20"),
        expected_offer_weight: String::from("30"),
    });

    for t in &test_cases {
        let simulation_res = query_simulation(
            deps.as_ref(),
            env.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: offer_amount,
            },
            t.start_time,
        );

        let simulation_res = simulation_res.unwrap_or_else(|e| {
            if !t.expected_error {
                panic!("{:?}", e);
            }

            SimulationResponse {
                return_amount: Default::default(),
                spread_amount: Default::default(),
                commission_amount: Default::default(),
                ask_weight: Default::default(),
                offer_weight: Default::default(),
            }
        });

        if !t.expected_error {
            assert_eq!(simulation_res.ask_weight.as_str(), &t.expected_ask_weight);
            assert_eq!(
                simulation_res.offer_weight.as_str(),
                &t.expected_offer_weight
            );
        }
    }
}

#[test]
fn compute_swap_rounding() {
    let offer_pool = Uint128::from(5_000_000_000_000_000_u128);
    let offer_weight = Decimal256::one();
    let ask_pool = Uint128::from(1_000_000_000_000_u128);
    let ask_weight = Decimal256::one();
    let offer_amount = Uint128::from(1_u128);
    let commission_rate = "0.0015";

    let return_amount = Uint128::from(0_u128);
    let spread_amount = Uint128::from(0_u128);
    let commission_amount = Uint128::from(0_u128);

    assert_eq!(
        compute_swap(
            offer_pool,
            offer_weight,
            ask_pool,
            ask_weight,
            offer_amount,
            commission_rate.to_string()
        ),
        Ok((return_amount, spread_amount, commission_amount))
    );
}

proptest! {
    #[test]
    fn compute_swap_test(
        offer_pool in 1_000_000..9_000_000_000_000_000_000u128,
        ask_pool in 1_000_000..9_000_000_000_000_000_000u128,
        offer_weight in 1..50u128,
        ask_weight in 1..50u128,
        offer_amount in 1..100_000_000000u128
    ) {

        let offer_pool = Uint128::from(offer_pool);
        let ask_pool = Uint128::from(ask_pool);
        let offer_weight = uint2dec(Uint128::from(offer_weight));
        let ask_weight = uint2dec(Uint128::from(ask_weight));
        let offer_amount = Uint128::from(offer_amount);
        let commission_rate = "0.0015".to_string();

        // Make sure there are no overflows
        compute_swap(
            offer_pool,
            offer_weight,
            ask_pool,
            ask_weight,
            offer_amount,
            commission_rate,
        ).unwrap();
    }
}
