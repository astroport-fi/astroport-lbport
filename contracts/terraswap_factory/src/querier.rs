use cosmwasm_std::{Addr, QueryRequest, StdResult, WasmQuery, to_binary, Deps};
use terraswap::pair::{QueryMsg};
use terraswap::asset::{PairInfo};

pub fn query_liquidity_token(
    deps: Deps,
    contract_addr: Addr,
) -> StdResult<Addr> {
    let res: PairInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&QueryMsg::Pair {})?,
    }))?;

    Ok(res.liquidity_token)
}
