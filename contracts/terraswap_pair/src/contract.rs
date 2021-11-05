use crate::math::{calc_in_given_out, calc_out_given_in, uint2dec};
use crate::state::PAIR_INFO;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Addr, Binary, Coin, Decimal, Deps, DepsMut, Env,
    MessageInfo, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use terraswap::U256;

use crate::error::ContractError;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;
use terraswap::asset::{Asset, AssetInfo, PairInfo, WeightedAsset};
use terraswap::hook::InitHook;
use terraswap::pair::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PoolResponse, QueryMsg,
    ReverseSimulationResponse, SimulationResponse,
};
use terraswap::querier::query_supply;
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

/// Commission rate == 0.15%
pub const COMMISSION_RATE: &str = "0.0015";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Check LBP parameters
    if msg.start_time < env.block.time.seconds() {
        return Err(ContractError::Std(StdError::generic_err(
            "start_time is less then current time",
        )));
    }

    if msg.end_time <= msg.start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "end_time is less then or same as start_time",
        )));
    }

    for asset in msg.asset_infos.iter() {
        if asset.start_weight.is_zero() {
            return Err(ContractError::Std(StdError::generic_err(
                "start_weight can not be 0",
            )));
        }

        if asset.end_weight.is_zero() {
            return Err(ContractError::Std(StdError::generic_err(
                "end_weight can not be 0",
            )));
        }
    }

    let pair_info: &PairInfo = &PairInfo {
        contract_addr: env.contract.address.clone(),
        liquidity_token: Addr::unchecked(""),
        asset_infos: [msg.asset_infos[0].clone(), msg.asset_infos[1].clone()],
        start_time: msg.start_time,
        end_time: msg.end_time,
        description: msg.description,
    };

    PAIR_INFO.save(deps.storage, pair_info)?;

    // Create LP token
    let mut messages: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_binary(&TokenInstantiateMsg {
                name: "terraswap liquidity token".to_string(),
                symbol: "uLP".to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
                init_hook: Some(InitHook {
                    msg: to_binary(&ExecuteMsg::PostInitialize {})?,
                    contract_addr: env.contract.address,
                }),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("terraswap liquidity token"),
        }
        .into(),
        id: 0,
        gas_limit: None,
        reply_on: ReplyOn::Never,
    }];

    if let Some(hook) = msg.init_hook {
        messages.push(SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: hook.contract_addr.to_string(),
                msg: hook.msg,
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never,
        });
    }

    Ok(Response::new().add_submessages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::PostInitialize {} => try_post_initialize(deps, info),
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
        } => try_provide_liquidity(deps, env, info, assets, slippage_tolerance),
        ExecuteMsg::Swap {
            offer_asset,
            belief_price,
            max_spread,
            to,
        } => {
            if !offer_asset.is_native_token() {
                return Err(ContractError::Unauthorized {});
            }

            try_swap(
                deps,
                env,
                info.clone(),
                info.sender,
                offer_asset,
                belief_price,
                max_spread,
                to,
            )
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender.clone();
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Swap {
            belief_price,
            max_spread,
            to,
        }) => {
            // only asset contract can execute this message
            let mut authorized: bool = false;
            let config: PairInfo = PAIR_INFO.load(deps.storage)?;
            let pools: [WeightedAsset; 2] =
                config.query_pools(deps.as_ref(), &env.contract.address)?;
            for pool in pools.iter() {
                if let AssetInfo::Token { contract_addr, .. } = &pool.info {
                    if contract_addr == &info.sender {
                        authorized = true;
                    }
                }
            }

            if !authorized {
                return Err(ContractError::Unauthorized {});
            }

            try_swap(
                deps,
                env,
                info,
                Addr::unchecked(cw20_msg.sender),
                Asset {
                    info: AssetInfo::Token { contract_addr },
                    amount: cw20_msg.amount,
                },
                belief_price,
                max_spread,
                to,
            )
        }
        Ok(Cw20HookMsg::WithdrawLiquidity {}) => try_withdraw_liquidity(
            deps,
            env,
            info,
            Addr::unchecked(cw20_msg.sender),
            cw20_msg.amount,
        ),
        Err(err) => Err(ContractError::Std(err)),
    }
}

// Must token contract execute it
pub fn try_post_initialize(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut config: PairInfo = PAIR_INFO.load(deps.storage)?;

    // permission check
    if config.liquidity_token != Addr::unchecked("") {
        return Err(ContractError::Unauthorized {});
    }
    config.liquidity_token = info.sender.clone();
    PAIR_INFO.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("liquidity_token_addr", info.sender.as_str()))
}

/// CONTRACT - should approve contract to use the amount of token
pub fn try_provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
) -> Result<Response, ContractError> {
    for asset in assets.iter() {
        asset.assert_sent_native_token_balance(&info)?;
    }

    let pair_info: PairInfo = PAIR_INFO.load(deps.storage)?;
    let mut pools: [WeightedAsset; 2] =
        pair_info.query_pools(deps.as_ref(), &env.contract.address)?;
    let deposits: [Uint128; 2] = [
        assets
            .iter()
            .find(|a| a.info.equal(&pools[0].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
        assets
            .iter()
            .find(|a| a.info.equal(&pools[1].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
    ];

    if deposits[0].is_zero() || deposits[1].is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let mut messages: Vec<SubMsg> = vec![];
    for (i, pool) in pools.iter_mut().enumerate() {
        // If the pool is token contract, then we need to execute TransferFrom msg to receive funds
        if let AssetInfo::Token { contract_addr, .. } = &pool.info {
            messages.push(SubMsg {
                msg: WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: deposits[i],
                    })?,
                    funds: vec![],
                }
                .into(),
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
            });
        } else {
            // If the asset is native token, balance is already increased
            // To calculated properly we should subtract user deposit from the pool
            pool.amount = pool.amount.checked_sub(deposits[i]).unwrap();
        }
    }

    // assert slippage tolerance
    assert_slippage_tolerance(&slippage_tolerance, &deposits, &pools)?;

    let liquidity_token = pair_info.liquidity_token.clone();
    let total_share = query_supply(deps.as_ref(), &liquidity_token)?;

    let share = if total_share.is_zero() {
        // Initial share = collateral amount
        Uint128::new(
            (U256::from(deposits[0].u128()) * U256::from(deposits[1].u128()))
                .integer_sqrt()
                .as_u128(),
        )
    } else {
        // min(1, 2)
        // 1. sqrt(deposit_0 * exchange_rate_0_to_1 * deposit_0) * (total_share / sqrt(pool_0 * pool_1))
        // == deposit_0 * total_share / pool_0
        // 2. sqrt(deposit_1 * exchange_rate_1_to_0 * deposit_1) * (total_share / sqrt(pool_1 * pool_1))
        // == deposit_1 * total_share / pool_1
        std::cmp::min(
            deposits[0].multiply_ratio(total_share, pools[0].amount),
            deposits[1].multiply_ratio(total_share, pools[1].amount),
        )
    };

    // mint LP token to sender
    messages.push(SubMsg {
        msg: WasmMsg::Execute {
            contract_addr: pair_info.liquidity_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: share,
            })?,
            funds: vec![],
        }
        .into(),
        id: 0,
        gas_limit: None,
        reply_on: ReplyOn::Never,
    });
    Ok(Response::new()
        .add_submessages(messages)
        .add_attributes(vec![
            attr("action", "provide_liquidity"),
            attr("assets", format!("{}, {}", assets[0], assets[1])),
            attr("share", share.to_string()),
        ]))
}

pub fn try_withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let pair_info: PairInfo = PAIR_INFO.load(deps.storage)?;

    if info.sender != pair_info.liquidity_token {
        return Err(ContractError::Unauthorized {});
    }
    let liquidity_addr: Addr = pair_info.liquidity_token.clone();

    let pools: [WeightedAsset; 2] = pair_info.query_pools(deps.as_ref(), &env.contract.address)?;
    let total_share: Uint128 = query_supply(deps.as_ref(), &liquidity_addr)?;

    let share_ratio: Decimal = Decimal::from_ratio(amount, total_share);
    let refund_assets: Vec<Asset> = pools
        .iter()
        .map(|a| Asset {
            info: a.info.clone(),
            amount: a.amount * share_ratio,
        })
        .collect();

    // update pool info
    Ok(Response::new()
        .add_submessages(vec![
            // refund asset tokens
            SubMsg {
                id: 0,
                msg: refund_assets[0].clone().into_msg(
                    deps.as_ref(),
                    env.contract.address.clone(),
                    sender.clone(),
                )?,
                gas_limit: None,
                reply_on: ReplyOn::Never,
            },
            SubMsg {
                id: 0,
                msg: refund_assets[1].clone().into_msg(
                    deps.as_ref(),
                    env.contract.address,
                    sender,
                )?,
                gas_limit: None,
                reply_on: ReplyOn::Never,
            },
            // burn liquidity token
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: pair_info.liquidity_token.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: ReplyOn::Never,
            },
        ])
        .add_attributes(vec![
            attr("action", "withdraw_liquidity"),
            attr("withdrawn_share", &amount.to_string()),
            attr(
                "refund_assets",
                format!("{}, {}", refund_assets[0], refund_assets[1]),
            ),
        ]))
}

// CONTRACT - a user must do token approval
#[allow(clippy::too_many_arguments)]
pub fn try_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    offer_asset: Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
) -> Result<Response, ContractError> {
    offer_asset.assert_sent_native_token_balance(&info)?;

    let pair_info: PairInfo = PAIR_INFO.load(deps.storage)?;

    let pools: [WeightedAsset; 2] = pair_info.query_pools(deps.as_ref(), &env.contract.address)?;

    let offer_pool: WeightedAsset;
    let ask_pool: WeightedAsset;

    // If the asset balance is already increased
    // To calculated properly we should subtract user deposit from the pool
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = WeightedAsset {
            amount: pools[0].amount.checked_sub(offer_asset.amount).unwrap(),
            info: pools[0].info.clone(),
            start_weight: pools[0].start_weight,
            end_weight: pools[0].end_weight,
        };
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = WeightedAsset {
            amount: pools[1].amount.checked_sub(offer_asset.amount).unwrap(),
            info: pools[1].info.clone(),
            start_weight: pools[1].start_weight,
            end_weight: pools[1].end_weight,
        };
        ask_pool = pools[0].clone();
    } else {
        return Err(ContractError::Std(StdError::generic_err(
            "Wrong asset info is given",
        )));
    }

    let ask_weight = get_current_weight(
        ask_pool.start_weight,
        ask_pool.end_weight,
        pair_info.start_time,
        pair_info.end_time,
        env.block.time.seconds(),
    )?;
    let offer_weight = get_current_weight(
        offer_pool.start_weight,
        offer_pool.end_weight,
        pair_info.start_time,
        pair_info.end_time,
        env.block.time.seconds(),
    )?;

    let offer_amount = offer_asset.amount;
    let (return_amount, spread_amount, commission_amount) = compute_swap(
        offer_pool.amount,
        offer_weight,
        ask_pool.amount,
        ask_weight,
        offer_amount,
    )?;

    // check max spread limit if exist
    assert_max_spread(
        belief_price,
        max_spread,
        offer_amount,
        return_amount + commission_amount,
        spread_amount,
    )?;

    // compute tax
    let return_asset = Asset {
        info: ask_pool.info.clone(),
        amount: return_amount,
    };

    let tax_amount = return_asset.compute_tax(deps.as_ref())?;

    // 1. send collateral token from the contract to a user
    // 2. send inactive commission to collector
    Ok(Response::new()
        .add_submessages(vec![SubMsg {
            id: 0,
            msg: return_asset.into_msg(
                deps.as_ref(),
                env.contract.address,
                to.unwrap_or(sender),
            )?,
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }])
        .add_attributes(vec![
            attr("action", "swap"),
            attr("offer_asset", offer_asset.info.to_string()),
            attr("ask_asset", ask_pool.info.to_string()),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", return_amount.to_string()),
            attr("tax_amount", tax_amount.to_string()),
            attr("spread_amount", spread_amount.to_string()),
            attr("commission_amount", commission_amount.to_string()),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Pair {} => to_binary(&query_pair_info(deps)?),
        QueryMsg::Pool {} => to_binary(&query_pool(deps)?),
        QueryMsg::Simulation {
            offer_asset,
            block_time,
        } => to_binary(&query_simulation(deps, offer_asset, block_time)?),
        QueryMsg::ReverseSimulation {
            ask_asset,
            block_time,
        } => to_binary(&query_reverse_simulation(deps, ask_asset, block_time)?),
    }
}

pub fn query_pair_info(deps: Deps) -> StdResult<PairInfo> {
    PAIR_INFO.load(deps.storage)
}

pub fn query_pool(deps: Deps) -> StdResult<PoolResponse> {
    let pair_info: PairInfo = PAIR_INFO.load(deps.storage)?;
    let contract_addr = pair_info.contract_addr.clone();
    let assets: [WeightedAsset; 2] = pair_info.query_pools(deps, &contract_addr)?;
    let total_share: Uint128 = query_supply(deps, &pair_info.liquidity_token)?;

    let resp = PoolResponse {
        assets,
        total_share,
    };

    Ok(resp)
}

pub fn query_simulation(
    deps: Deps,
    offer_asset: Asset,
    block_time: u64,
) -> StdResult<SimulationResponse> {
    let pair_info: PairInfo = PAIR_INFO.load(deps.storage)?;

    let contract_addr = pair_info.contract_addr.clone();
    let pools: [WeightedAsset; 2] = pair_info.query_pools(deps, &contract_addr)?;

    let offer_pool: WeightedAsset;
    let ask_pool: WeightedAsset;
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();
    } else {
        return Err(StdError::generic_err(
            "Given offer asset does not belong to pairs",
        ));
    }

    let ask_weight = get_current_weight(
        ask_pool.start_weight,
        ask_pool.end_weight,
        pair_info.start_time,
        pair_info.end_time,
        block_time,
    )?;

    let offer_weight = get_current_weight(
        offer_pool.start_weight,
        offer_pool.end_weight,
        pair_info.start_time,
        pair_info.end_time,
        block_time,
    )?;

    let (return_amount, spread_amount, commission_amount) = compute_swap(
        offer_pool.amount,
        offer_weight,
        ask_pool.amount,
        ask_weight,
        offer_asset.amount,
    )?;

    Ok(SimulationResponse {
        return_amount,
        spread_amount,
        commission_amount,
        ask_weight: ask_weight.to_string(),
        offer_weight: offer_weight.to_string(),
    })
}

pub fn query_reverse_simulation(
    deps: Deps,
    ask_asset: Asset,
    block_time: u64,
) -> StdResult<ReverseSimulationResponse> {
    let pair_info: PairInfo = PAIR_INFO.load(deps.storage)?;

    let contract_addr = pair_info.contract_addr.clone();
    let pools: [WeightedAsset; 2] = pair_info.query_pools(deps, &contract_addr)?;

    let offer_pool: WeightedAsset;
    let ask_pool: WeightedAsset;
    if ask_asset.info.equal(&pools[0].info) {
        ask_pool = pools[0].clone();
        offer_pool = pools[1].clone();
    } else if ask_asset.info.equal(&pools[1].info) {
        ask_pool = pools[1].clone();
        offer_pool = pools[0].clone();
    } else {
        return Err(StdError::generic_err(
            "Given ask asset is not blong to pairs",
        ));
    }

    let ask_weight = get_current_weight(
        ask_pool.start_weight,
        ask_pool.end_weight,
        pair_info.start_time,
        pair_info.end_time,
        block_time,
    )?;
    let offer_weight = get_current_weight(
        offer_pool.start_weight,
        offer_pool.end_weight,
        pair_info.start_time,
        pair_info.end_time,
        block_time,
    )?;

    let (offer_amount, spread_amount, commission_amount) = compute_offer_amount(
        offer_pool.amount,
        offer_weight,
        ask_pool.amount,
        ask_weight,
        ask_asset.amount,
    )?;

    Ok(ReverseSimulationResponse {
        offer_amount,
        spread_amount,
        commission_amount,
        ask_weight: ask_weight.to_string(),
        offer_weight: offer_weight.to_string(),
    })
}

pub fn amount_of(coins: &[Coin], denom: String) -> Uint128 {
    match coins.iter().find(|x| x.denom == denom) {
        Some(coin) => coin.amount,
        None => Uint128::zero(),
    }
}

fn get_ask_by_spot_price(
    offer_pool: Uint128,
    offer_weight: Decimal256,
    ask_pool: Uint128,
    ask_weight: Decimal256,
    offer_amount: Uint128,
) -> Uint128 {
    let ask_pool: Uint256 = ask_pool.into();
    let offer_pool: Uint256 = offer_pool.into();
    let offer_amount: Uint256 = offer_amount.into();

    let ask_ratio = Decimal256::from_uint256(ask_pool)
        .div(Decimal256::from_str(&ask_weight.to_string()).unwrap());
    let offer_ratio = Decimal256::from_uint256(offer_pool)
        .div(Decimal256::from_str(&offer_weight.to_string()).unwrap());

    let ask_amount = ask_ratio.div(offer_ratio).mul(offer_amount);

    ask_amount.into()
}

pub fn compute_swap(
    offer_pool: Uint128,
    offer_weight: Decimal256,
    ask_pool: Uint128,
    ask_weight: Decimal256,
    offer_amount: Uint128,
) -> StdResult<(Uint128, Uint128, Uint128)> {
    // offer => ask
    let return_amount =
        calc_out_given_in(offer_pool, offer_weight, ask_pool, ask_weight, offer_amount);

    // calculate spread & commission
    let spot_price =
        get_ask_by_spot_price(offer_pool, offer_weight, ask_pool, ask_weight, offer_amount);

    let spread_amount: Uint128 = spot_price
        .checked_sub(return_amount)
        .unwrap_or_else(|_| Uint128::zero());

    let commission_amount: Uint128 = return_amount * Decimal::from_str(COMMISSION_RATE).unwrap();

    // commission will be absorbed to pool
    let return_amount: Uint128 = return_amount.checked_sub(commission_amount).unwrap();

    Ok((return_amount, spread_amount, commission_amount))
}

fn compute_offer_amount(
    offer_pool: Uint128,
    offer_weight: Decimal256,
    ask_pool: Uint128,
    ask_weight: Decimal256,
    ask_amount: Uint128,
) -> StdResult<(Uint128, Uint128, Uint128)> {
    // ask => offer

    let one_minus_commission = Decimal256::one() - Decimal256::from_str(COMMISSION_RATE).unwrap();

    let before_commission_deduction =
        ask_amount * (Decimal256::one() / one_minus_commission).into();

    let offer_amount = calc_in_given_out(
        offer_pool,
        offer_weight,
        ask_pool,
        ask_weight,
        before_commission_deduction,
    );

    let spot_price =
        get_ask_by_spot_price(offer_pool, offer_weight, ask_pool, ask_weight, offer_amount);

    let spread_amount = spot_price
        .checked_sub(before_commission_deduction)
        .unwrap_or_else(|_| Uint128::zero());

    let commission_amount =
        before_commission_deduction * Decimal::from_str(COMMISSION_RATE).unwrap();

    Ok((offer_amount, spread_amount, commission_amount))
}

/// If `belief_price` and `max_spread` both are given,
/// we compute new spread else we just use terraswap
/// spread to check `max_spread`
pub fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    offer_amount: Uint128,
    return_amount: Uint128,
    spread_amount: Uint128,
) -> StdResult<()> {
    if let (Some(max_spread), Some(belief_price)) = (max_spread, belief_price) {
        let expected_return =
            offer_amount * Decimal::from(Decimal256::one() / Decimal256::from(belief_price));
        let spread_amount = expected_return
            .checked_sub(return_amount)
            .unwrap_or_else(|_| Uint128::zero());

        if return_amount < expected_return
            && Decimal::from_ratio(spread_amount, expected_return) > max_spread
        {
            return Err(StdError::generic_err("Operation exceeds max spread limit"));
        }
    } else if let Some(max_spread) = max_spread {
        if Decimal::from_ratio(spread_amount, return_amount + spread_amount) > max_spread {
            return Err(StdError::generic_err("Operation exceeds max spread limit"));
        }
    }

    Ok(())
}

fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Uint128; 2],
    pools: &[WeightedAsset; 2],
) -> StdResult<()> {
    if let Some(slippage_tolerance) = *slippage_tolerance {
        let slippage_tolerance: Decimal256 = slippage_tolerance.into();
        if slippage_tolerance > Decimal256::one() {
            return Err(StdError::generic_err(
                "slippage_tolerance cannot bigger than 1",
            ));
        }

        let one_minus_slippage_tolerance = Decimal256::one() - slippage_tolerance;
        let deposits: [Uint256; 2] = [deposits[0].into(), deposits[1].into()];
        let pools: [Uint256; 2] = [pools[0].amount.into(), pools[1].amount.into()];

        // Ensure each prices are not dropped as much as slippage tolerance rate
        if Decimal256::from_ratio(deposits[0], deposits[1]) * one_minus_slippage_tolerance
            > Decimal256::from_ratio(pools[0], pools[1])
            || Decimal256::from_ratio(deposits[1], deposits[0]) * one_minus_slippage_tolerance
                > Decimal256::from_ratio(pools[1], pools[0])
        {
            return Err(StdError::generic_err(
                "Operation exceeds max splippage tolerance",
            ));
        }
    }

    Ok(())
}

/// Uses start_time and end_time parameters, start_weight and end_weight for both assets
/// and current timestamp to calculate the weight for assets
fn get_current_weight(
    start_weight: Uint128,
    end_weight: Uint128,
    start_time: u64,
    end_time: u64,
    block_time: u64,
) -> StdResult<Decimal256> {
    if block_time < start_time {
        return Err(StdError::generic_err("Sale has not started yet"));
    }

    if block_time > end_time {
        return Err(StdError::generic_err("Sale has already finished"));
    }

    let start_weight_fixed = uint2dec(start_weight);
    let time_diff = uint2dec(Uint128::from(end_time - start_time));

    if end_weight > start_weight {
        let ratio = uint2dec(Uint128::from(
            (end_weight.u128() - start_weight.u128()) * (block_time - start_time) as u128,
        ))
        .div(time_diff);

        Ok(start_weight_fixed.add(ratio))
    } else {
        let ratio = uint2dec(Uint128::from(
            (start_weight.u128() - end_weight.u128()) * (block_time - start_time) as u128,
        ))
        .div(time_diff);

        Ok(start_weight_fixed.sub(ratio))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
