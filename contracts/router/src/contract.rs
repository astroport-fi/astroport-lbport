use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::operations::execute_swap_operation;
use crate::querier::compute_tax;
use crate::state::{Config, CONFIG};

use crate::error::ContractError;
use astroport_lbp::asset::{Asset, AssetInfo};
use astroport_lbp::factory::FactoryPairInfo;
use astroport_lbp::pair::{QueryMsg as PairQueryMsg, SimulationResponse};
use astroport_lbp::querier::query_factory_pair_info;
use astroport_lbp::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use std::collections::HashMap;
use terra_cosmwasm::{SwapResponse, TerraMsgWrapper, TerraQuerier};

// version info for migration info
const CONTRACT_NAME: &str = "astroport-lbp-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(
        deps.storage,
        &Config {
            astroport_lbp_factory: msg.astroport_lbp_factory,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, msg),
        ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => execute_swap_operations(deps, env, info.sender, operations, minimum_receive, to),
        ExecuteMsg::ExecuteSwapOperation { operation, to } => {
            execute_swap_operation(deps, env, info, operation, to)
        }
        ExecuteMsg::AssertMinimumReceive {
            asset_info,
            prev_balance,
            minimum_receive,
            receiver,
        } => assert_minium_receive(
            deps.as_ref(),
            asset_info,
            prev_balance,
            minimum_receive,
            receiver,
        ),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => {
            let to_addr = if let Some(to_addr) = to {
                Some(deps.api.addr_validate(to_addr.as_str())?)
            } else {
                None
            };
            execute_swap_operations(deps, env, sender, operations, minimum_receive, to_addr)
        }
    }
}

pub fn execute_swap_operations(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<Addr>,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(ContractError::MustProvideOperations {});
    }

    // Assert the operations are properly set
    assert_operations(&operations)?;

    let to = if let Some(to) = to { to } else { sender };
    let target_asset_info = operations.last().unwrap().get_ask_asset_info();

    let mut operation_index = 0;
    let mut messages: Vec<CosmosMsg<TerraMsgWrapper>> = operations
        .into_iter()
        .map(|op| {
            operation_index += 1;
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: op,
                    to: if operation_index == operations_len {
                        Some(to.clone())
                    } else {
                        None
                    },
                })?,
                funds: vec![],
            }))
        })
        .collect::<StdResult<Vec<CosmosMsg<TerraMsgWrapper>>>>()?;

    // Execute minimum amount assertion
    if let Some(minimum_receive) = minimum_receive {
        let receiver_balance = target_asset_info.query_pool(deps.as_ref(), &to)?;
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::AssertMinimumReceive {
                asset_info: target_asset_info,
                prev_balance: receiver_balance,
                minimum_receive,
                receiver: to,
            })?,
        }));
    }
    Ok(Response::new().add_messages(messages))
}

fn assert_minium_receive(
    deps: Deps,
    asset_info: AssetInfo,
    prev_balance: Uint128,
    minimum_receive: Uint128,
    receiver: Addr,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let receiver_balance = asset_info.query_pool(deps, &receiver)?;
    let swap_amount = receiver_balance.checked_sub(prev_balance)?;

    if swap_amount < minimum_receive {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "assertion failed; minimum receive amount: {}, swap amount: {}",
            minimum_receive, swap_amount
        ))));
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            block_time,
            operations,
        } => to_binary(&simulate_swap_operations(
            deps,
            offer_amount,
            block_time,
            operations,
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        astroport_lbp_factory: state.astroport_lbp_factory,
    };
    Ok(resp)
}

fn simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    block_time: u64,
    operations: Vec<SwapOperation>,
) -> StdResult<SimulateSwapOperationsResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let astroport_lbp_factory = config.astroport_lbp_factory;
    let terra_querier = TerraQuerier::new(&deps.querier);

    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(StdError::generic_err("must provide operations"));
    }

    assert_operations(&operations)?;
    assert_operations_order(&operations)?;

    let mut operation_index = 0;
    let mut offer_amount = offer_amount;
    for operation in operations.into_iter() {
        operation_index += 1;

        match operation {
            SwapOperation::NativeSwap {
                offer_denom,
                ask_denom,
            } => {
                // Deduct tax before query simulation
                // because last swap is swap_send
                if operation_index == operations_len {
                    offer_amount = offer_amount.checked_sub(compute_tax(
                        deps,
                        offer_amount,
                        offer_denom.clone(),
                    )?)?;
                }

                let res: SwapResponse = terra_querier.query_swap(
                    Coin {
                        denom: offer_denom,
                        amount: offer_amount,
                    },
                    ask_denom,
                )?;

                offer_amount = res.receive.amount;
            }

            SwapOperation::AstroSwap {
                offer_asset_info,
                ask_asset_info,
            } => {
                let pair_info: FactoryPairInfo = query_factory_pair_info(
                    deps,
                    &astroport_lbp_factory,
                    &[offer_asset_info.clone(), ask_asset_info.clone()],
                )?;

                // Deduct tax before querying simulation
                if let AssetInfo::NativeToken { denom } = offer_asset_info.clone() {
                    offer_amount =
                        offer_amount.checked_sub(compute_tax(deps, offer_amount, denom)?)?;
                }
                let mut res: SimulationResponse =
                    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: pair_info.contract_addr.to_string(),
                        msg: to_binary(&PairQueryMsg::Simulation {
                            offer_asset: Asset {
                                info: offer_asset_info,
                                amount: offer_amount,
                            },
                            block_time,
                        })?,
                    }))?;

                // Deduct tax after querying simulation
                if let AssetInfo::NativeToken { denom } = ask_asset_info.clone() {
                    res.return_amount = res.return_amount.checked_sub(compute_tax(
                        deps,
                        res.return_amount,
                        denom,
                    )?)?;
                }

                offer_amount = res.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: offer_amount,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

fn assert_operations_order(operations: &[SwapOperation]) -> StdResult<()> {
    let mut prev_ask = String::new();

    for operation in operations.iter() {
        let offer_asset = operation.get_offer_asset_info();
        let ask_asset = operation.get_ask_asset_info();

        if !prev_ask.is_empty() && prev_ask != offer_asset.to_string() {
            return Err(StdError::generic_err(
                "invalid operations order; offer does not equal to prev ask",
            ));
        }

        prev_ask = ask_asset.to_string()
    }

    Ok(())
}

fn assert_operations(operations: &[SwapOperation]) -> StdResult<()> {
    let mut ask_asset_map: HashMap<String, bool> = HashMap::new();

    for operation in operations.iter() {
        let offer_asset = operation.get_offer_asset_info();
        let ask_asset = operation.get_ask_asset_info();

        ask_asset_map.remove(&offer_asset.to_string());
        ask_asset_map.insert(ask_asset.to_string(), true);
    }

    if ask_asset_map.keys().len() != 1 {
        return Err(StdError::generic_err(
            "invalid operations; multiple output token",
        ));
    }

    Ok(())
}

#[test]
fn test_invalid_operations() {
    // empty error
    assert_eq!(true, assert_operations(&vec![]).is_err());

    // uluna output
    assert_eq!(
        true,
        assert_operations(&vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "uluna".to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            }
        ])
        .is_ok()
    );

    // asset0002 output
    assert_eq!(
        true,
        assert_operations(&vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "uluna".to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0002"),
                },
            },
        ])
        .is_ok()
    );

    // multiple output token types error
    assert_eq!(
        true,
        assert_operations(&vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "ukrw".to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uaud".to_string(),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0002"),
                },
            },
        ])
        .is_err()
    );
}

#[test]
fn test_invalid_operations_order() {
    assert_eq!(
        true,
        assert_operations_order(&vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "uluna".to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0002"),
                },
            },
        ])
        .is_ok()
    );

    assert_eq!(
        true,
        assert_operations_order(&vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "uluna".to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset0001"),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            }
        ])
        .is_err()
    );
}
