use cosmwasm_std::{to_binary, Addr, Deps, QueryRequest, StdResult, WasmQuery};
use terraswap::asset::PairInfo;
use terraswap::pair::QueryMsg;

pub fn query_liquidity_token(deps: Deps, contract_addr: Addr) -> StdResult<Addr> {
    Ok(query_pair_info(deps, &contract_addr)?.liquidity_token)
}

pub fn query_pair_info(deps: Deps, pair_contract: &Addr) -> StdResult<PairInfo> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&QueryMsg::Pair {})?,
    }))
}
