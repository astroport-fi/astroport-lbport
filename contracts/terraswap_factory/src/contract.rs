use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, ReplyOn, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};

use crate::querier::query_liquidity_token;
use crate::state::{pair_key, read_pair, read_pairs, Config, CONFIG, PAIRS};

use crate::error::ContractError;
use terraswap::asset::{AssetInfo, WeightedAssetInfo};
use terraswap::factory::{
    ConfigResponse, ExecuteMsg, FactoryPairInfo, FactoryPairInfoRaw, InitMsg, MigrateMsg,
    PairsResponse, QueryMsg,
};
use terraswap::hook::InitHook;
use terraswap::pair::InitMsg as PairInitMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        owner: info.sender,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
    };
    CONFIG.save(deps.storage, &config)?;
    let mut messages: Vec<SubMsg> = vec![];
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
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
        } => try_update_config(deps, info, owner, token_code_id, pair_code_id),
        ExecuteMsg::CreatePair {
            asset_infos,
            start_time,
            end_time,
            description,
            init_hook,
        } => try_create_pair(
            deps,
            env,
            info,
            asset_infos,
            start_time,
            end_time,
            description,
            init_hook,
        ),
        ExecuteMsg::Register { asset_infos } => try_register(deps, info, asset_infos),
        ExecuteMsg::Unregister { asset_infos } => try_unregister(deps, env, info, asset_infos),
    }
}

// Only owner can execute it
pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(owner) = owner {
        config.owner = owner;
    }
    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }
    if let Some(pair_code_id) = pair_code_id {
        config.pair_code_id = pair_code_id;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

#[allow(clippy::too_many_arguments)]
// Anyone can execute it to create swap pair
pub fn try_create_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: [WeightedAssetInfo; 2],
    start_time: u64,
    end_time: u64,
    description: Option<String>,
    init_hook: Option<InitHook>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    let raw_infos = [
        asset_infos[0].info.to_raw(deps.as_ref())?,
        asset_infos[1].info.to_raw(deps.as_ref())?,
    ];
    if read_pair(deps.as_ref(), &raw_infos).is_ok() {
        return Err(ContractError::Std(StdError::generic_err(
            "Pair already exists",
        )));
    }

    let raw_asset_infos = [
        asset_infos[0].to_raw(deps.as_ref())?,
        asset_infos[1].to_raw(deps.as_ref())?,
    ];
    PAIRS.save(
        deps.storage,
        &pair_key(&raw_infos),
        &FactoryPairInfoRaw {
            owner: info.sender,
            liquidity_token: Addr::unchecked(""),
            contract_addr: Addr::unchecked(""),
            asset_infos: raw_asset_infos,
            start_time,
            end_time,
        },
    )?;

    let mut messages: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: config.pair_code_id,
            funds: vec![],
            admin: None,
            label: String::from("Terraswap pair"),
            msg: to_binary(&PairInitMsg {
                asset_infos: asset_infos.clone(),
                token_code_id: config.token_code_id,
                init_hook: Some(InitHook {
                    contract_addr: env.contract.address,
                    msg: to_binary(&ExecuteMsg::Register {
                        asset_infos: asset_infos.clone(),
                    })?,
                }),
                start_time,
                end_time,
                description,
            })?,
        }
            .into(),
        id: 0,
        gas_limit: None,
        reply_on: ReplyOn::Never,
    }];

    if let Some(hook) = init_hook {
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

    Ok(Response::new()
        .add_submessages(messages)
        .add_attributes(vec![
            attr("action", "create_pair"),
            attr("pair", format!("{}-{}", asset_infos[0], asset_infos[1])),
        ]))
}

/// create pair execute this message
pub fn try_register(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [WeightedAssetInfo; 2],
) -> Result<Response, ContractError> {
    let raw_infos = [
        asset_infos[0].info.to_raw(deps.as_ref())?,
        asset_infos[1].info.to_raw(deps.as_ref())?,
    ];
    let pair_info: FactoryPairInfoRaw = read_pair(deps.as_ref(), &raw_infos)?;
    if pair_info.contract_addr != Addr::unchecked("") {
        return Err(ContractError::Std(StdError::generic_err(
            "Pair was already registered",
        )));
    }

    let pair_contract = info.sender;
    let liquidity_token = query_liquidity_token(deps.as_ref(), pair_contract.clone())?;
    PAIRS.save(
        deps.storage,
        &pair_key(&raw_infos),
        &FactoryPairInfoRaw {
            contract_addr: pair_contract.clone(),
            liquidity_token,
            ..pair_info
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "register"),
        attr("pair_contract_addr", pair_contract),
    ]))
}

/// remove from list of pairs
pub fn try_unregister(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref())?,
        asset_infos[1].to_raw(deps.as_ref())?,
    ];

    let pair_info: FactoryPairInfoRaw = read_pair(deps.as_ref(), &raw_infos)?;

    // Permission check
    if pair_info.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    //remove_pair(deps.storage, &pair_info);
    PAIRS.remove(deps.storage, &pair_key(&raw_infos));

    Ok(Response::new().add_attributes(vec![
        attr("action", "unregister"),
        attr("pair", format!("{}-{}", asset_infos[0], asset_infos[1])),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_binary(&query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_binary(&query_pairs(deps, start_after, limit)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.clone(),
        token_code_id: state.token_code_id,
        pair_code_id: state.pair_code_id,
    };

    Ok(resp)
}

pub fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<FactoryPairInfo> {
    let raw_infos = [asset_infos[0].to_raw(deps)?, asset_infos[1].to_raw(deps)?];
    let pair_info: FactoryPairInfoRaw = read_pair(deps, &raw_infos)?;
    pair_info.to_normal(deps)
}

pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some([start_after[0].to_raw(deps)?, start_after[1].to_raw(deps)?])
    } else {
        None
    };

    let pairs: Vec<FactoryPairInfo> = read_pairs(deps, start_after, limit);
    let resp = PairsResponse { pairs };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
