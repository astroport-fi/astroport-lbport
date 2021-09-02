use cosmwasm_std::{
    Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Querier, Response, StdError, StdResult,
    Storage, WasmMsg,
};

use cw2::set_contract_version;
use cw20_base::contract::{create_accounts, execute as cw20_execute, query as cw20_query};
use cw20_base::msg::{ExecuteMsg, QueryMsg};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use cw20_base::ContractError;
use terraswap::token::InstantiateMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(mut deps: DepsMut, _env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Check valid token info
    msg.validate()?;

    // Create initial accounts
    let total_supply = create_accounts(&mut deps, &msg.initial_balances)?;

    // Check supply cap
    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(StdError::generic_err("Initial supply greater than cap"));
        }
    }

    let mint = match msg.mint {
        Some(m) => Some(MinterData {
            minter: Addr::unchecked(m.minter),
            cap: m.cap,
        }),
        None => None,
    };

    // Store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint,
    };

    TOKEN_INFO.save(deps.storage, &data)?;

    if let Some(hook) = msg.init_hook {
        Ok(Response::new().add_message(WasmMsg::Execute {
            contract_addr: hook.contract_addr.to_string(),
            msg: hook.msg,
            funds: vec![],
        }))
    } else {
        Ok(Response::default())
    }
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    cw20_execute(deps, env, info, msg)
}

// pub fn migrate(
//     deps: DepsMut,
//     env: Env,
//     msg: MigrateMsg,
// ) -> Result {
//     cw20_migrate(deps, env, msg)
// }

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    cw20_query(deps, env, msg)
}
