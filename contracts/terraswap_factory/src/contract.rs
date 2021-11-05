use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use protobuf::Message;

use terraswap::asset::{AssetInfo, WeightedAssetInfo};
use terraswap::factory::{
    ConfigResponse, ExecuteMsg, FactoryPairInfo, InstantiateMsg, MigrateMsg, PairsResponse,
    QueryMsg,
};
use terraswap::hook::InitHook;
use terraswap::pair::InstantiateMsg as PairInstantiateMsg;

use crate::error::ContractError;
use crate::querier::query_liquidity_token;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    pair_key, read_pair, read_pairs, Config, TmpPairInfo, CONFIG, PAIRS, TMP_PAIR_INFO,
};

// version info for migration info
const CONTRACT_NAME: &str = "terraswap-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let owner = deps.api.addr_validate(&msg.owner)?;

    let config = Config {
        owner,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
    };
    CONFIG.save(deps.storage, &config)?;

    if let Some(hook) = msg.init_hook {
        Ok(
            Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: hook.contract_addr.to_string(),
                msg: hook.msg,
                funds: vec![],
            })),
        )
    } else {
        Ok(Response::new())
    }
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
    _env: Env,
    info: MessageInfo,
    weighted_asset_infos: [WeightedAssetInfo; 2],
    start_time: u64,
    end_time: u64,
    description: Option<String>,
    init_hook: Option<InitHook>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    let asset_infos = [
        weighted_asset_infos[0].info.clone(),
        weighted_asset_infos[1].info.clone(),
    ];
    if read_pair(deps.as_ref(), &asset_infos).is_ok() {
        return Err(ContractError::Std(StdError::generic_err(
            "Pair already exists",
        )));
    }
    let pair_key = pair_key(&asset_infos);
    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            asset_infos: weighted_asset_infos.clone(),
        },
    )?;
    PAIRS.save(
        deps.storage,
        &pair_key,
        &FactoryPairInfo {
            owner: info.sender,
            liquidity_token: Addr::unchecked(""),
            contract_addr: Addr::unchecked(""),
            asset_infos: weighted_asset_infos.clone(),
            start_time,
            end_time,
        },
    )?;

    let messages: Vec<SubMsg> = vec![SubMsg {
        id: 0,
        msg: WasmMsg::Instantiate {
            admin: Some(config.owner.to_string()),
            code_id: config.pair_code_id,
            msg: to_binary(&PairInstantiateMsg {
                asset_infos: weighted_asset_infos,
                token_code_id: config.token_code_id,
                init_hook: None,
                start_time,
                end_time,
                description,
            })?,
            funds: vec![],
            label: "TerraSwap pair".to_string(),
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    if let Some(hook) = init_hook {
        Ok(Response::new()
            .add_submessages(messages)
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: hook.contract_addr.to_string(),
                msg: hook.msg,
                funds: vec![],
            })))
    } else {
        Ok(Response::new().add_submessages(messages))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let tmp = TMP_PAIR_INFO.load(deps.storage)?;

    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract_addr = deps.api.addr_validate(res.get_contract_address())?;

    Ok(try_register(deps, pair_contract_addr, tmp.asset_infos)?)
    // Ok(Response::new().add_attribute("liquidity_token_addr", config.liquidity_token))

    // Ok(Response::new())
}

/// create pair execute this message
pub fn try_register(
    deps: DepsMut,
    pair_contract: Addr,
    weighted_asset_infos: [WeightedAssetInfo; 2],
) -> Result<Response, ContractError> {
    let asset_infos = [
        weighted_asset_infos[0].info.clone(),
        weighted_asset_infos[1].info.clone(),
    ];
    let pair_info: FactoryPairInfo = read_pair(deps.as_ref(), &asset_infos)?;
    if pair_info.contract_addr != Addr::unchecked("") {
        return Err(ContractError::PairWasRegistered {});
    }

    let liquidity_token = query_liquidity_token(deps.as_ref(), pair_contract.clone())?;
    PAIRS.save(
        deps.storage,
        &pair_key(&asset_infos),
        &FactoryPairInfo {
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
    let pair_info: FactoryPairInfo = read_pair(deps.as_ref(), &asset_infos)?;

    // Permission check
    if pair_info.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    PAIRS.remove(deps.storage, &pair_key(&asset_infos));

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
    PAIRS.load(deps.storage, &pair_key(&asset_infos))
}

pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after =
        start_after.map(|start_after| [start_after[0].clone(), start_after[1].clone()]);
    let pairs: Vec<FactoryPairInfo> = read_pairs(deps, start_after, limit);
    let resp = PairsResponse { pairs };
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
