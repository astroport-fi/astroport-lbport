use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::factory::QueryMsg as FactoryQueryMsg;
use crate::pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse};

use cosmwasm_std::{to_binary, AllBalanceResponse, Api, BalanceResponse, BankQuery, Coin, Addr, Querier, QueryRequest, StdResult, Storage, Uint128, WasmQuery, Deps};
use cw20::{TokenInfoResponse, Cw20QueryMsg, BalanceResponse as Cw20BalanceResponse};

pub fn query_balance(
    deps: Deps,
    account_addr: &Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = deps.querier.query(
        &QueryRequest::Bank(BankQuery::Balance {
            address: account_addr.to_string(),
            denom,
        }))?;
    Ok(balance.amount.amount)
}

pub fn query_all_balances(
    deps: Deps,
    account_addr: &Addr,
) -> StdResult<Vec<Coin>> {
    let all_balances: AllBalanceResponse = deps.querier.query(
        &QueryRequest::Bank(BankQuery::AllBalances {
            address: account_addr.to_string(),
        }))?;
    Ok(all_balances.amount)
}

pub fn query_token_balance(
    deps: Deps,
    contract_addr: &Addr,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    // load balance form the token contract
    let res: Cw20BalanceResponse = deps.querier.query(
        &QueryRequest::Wasm(
            WasmQuery::Smart {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20QueryMsg::Balance {
                    address: account_addr.to_string(),
                })?,
            }))
        .unwrap_or_else(|_| Cw20BalanceResponse{ balance: Uint128::zero()});

    Ok(res.balance)
}

pub fn query_supply(
    deps: Deps,
    contract_addr: Addr,
) -> StdResult<Uint128> {
    let res: TokenInfoResponse = deps.querier.query(
        &QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;

    Ok(res.total_supply)
}

pub fn query_pair_info<S: Storage, A: Api, Q: Querier>(
    deps: Deps,
    factory_contract: &Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

pub fn simulate<S: Storage, A: Api, Q: Querier>(
    deps: Deps,
    pair_contract: &Addr,
    offer_asset: &Asset,
    block_time: u64,
) -> StdResult<SimulationResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
            block_time,
        })?,
    }))
}

pub fn reverse_simulate<S: Storage, A: Api, Q: Querier>(
    deps: Deps,
    pair_contract: &Addr,
    ask_asset: &Asset,
    block_time: u64,
) -> StdResult<ReverseSimulationResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
            block_time,
        })?,
    }))
}
