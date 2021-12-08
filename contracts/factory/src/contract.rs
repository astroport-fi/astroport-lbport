use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use protobuf::Message;

use astroport_lbp::asset::{AssetInfo, PairInfo, WeightedAssetInfo};
use astroport_lbp::factory::{
    ConfigResponse, ExecuteMsg, FactoryPairInfo, InstantiateMsg, MigrateMsg, PairsResponse,
    QueryMsg,
};
use astroport_lbp::pair::ExecuteMsg::UpdatePairConfigs;
use astroport_lbp::pair::InstantiateMsg as PairInstantiateMsg;

use crate::error::ContractError;
use crate::querier::query_pair_info;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    pair_key, read_pair, read_pairs, Config, TmpPairInfo, CONFIG, PAIRS, TMP_PAIR_INFO,
};

// version info for migration info
const CONTRACT_NAME: &str = "astroport-lbp-factory";
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
        commission_rate: msg.commission_rate,
        collector_addr: match msg.collector_addr {
            Some(addr) => Some(deps.api.addr_validate(&addr)?),
            None => None,
        },
        spilt_to_collector: msg.spilt_to_collector
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
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
            commission_rate,
            collector_addr,
            spilt_to_collector,
        } => try_update_config(
            deps,
            info,
            owner,
            token_code_id,
            pair_code_id,
            commission_rate,
            collector_addr,
            spilt_to_collector,
        ),
        ExecuteMsg::CreatePair {
            asset_infos,
            start_time,
            end_time,
            description,
        } => try_create_pair(
            deps,
            env,
            info,
            asset_infos,
            start_time,
            end_time,
            description,
        ),
        ExecuteMsg::Unregister { asset_infos } => try_unregister(deps, env, info, asset_infos),
        ExecuteMsg::UpdatePair { pair, end_time } => try_update_pair(deps, info, pair, end_time),
    }
}

// Only owner can execute it
pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
    commission_rate: Option<String>,
    collector_addr: Option<String>,
    spilt_to_collector: Option<String>,
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
    if let Some(commission_rate) = commission_rate {
        config.commission_rate = commission_rate;
    }
    if let Some(collector_addr) = collector_addr {
        config.collector_addr = Some(deps.api.addr_validate(&collector_addr)?);
    }
    if let Some(spilt_to_collector) = spilt_to_collector {
        config.spilt_to_collector = Some(spilt_to_collector);
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

#[allow(clippy::too_many_arguments)]
// Only the owner can execute it to create swap pair
pub fn try_create_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    weighted_asset_infos: [WeightedAssetInfo; 2],
    start_time: u64,
    end_time: Option<u64>,
    description: Option<String>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

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
            pair_key,
            owner: info.sender,
        },
    )?;

    Ok(Response::new()
        .add_submessage(SubMsg {
            id: 0,
            msg: WasmMsg::Instantiate {
                admin: Some(config.owner.to_string()),
                code_id: config.pair_code_id,
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: weighted_asset_infos,
                    token_code_id: config.token_code_id,
                    start_time,
                    end_time,
                    description,
                    commission_rate: config.commission_rate,
                    collector_addr: config.collector_addr,
                    spilt_to_collector: config.spilt_to_collector,
                })?,
                funds: vec![],
                label: "Astroport pair".to_string(),
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        })
        .add_attributes(vec![
            attr("action", "create_pair"),
            attr("pair", format!("{}-{}", asset_infos[0], asset_infos[1])),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let tmp = TMP_PAIR_INFO.load(deps.storage)?;
    if PAIRS.may_load(deps.storage, &tmp.pair_key)?.is_some() {
        return Err(ContractError::PairWasRegistered {});
    }

    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract = deps.api.addr_validate(res.get_contract_address())?;

    PAIRS.save(
        deps.storage,
        &tmp.pair_key,
        &FactoryPairInfo {
            owner: tmp.owner,
            contract_addr: pair_contract.clone(),
        },
    )?;

    Ok(Response::new().add_attributes(vec![("pair_contract_addr", pair_contract)]))
}

// Only owner can execute it
pub fn try_update_pair(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    end_time: Option<u64>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let pair_addr = deps.api.addr_validate(&pair)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    Ok(Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&UpdatePairConfigs {
                end_time: end_time,
                commission_rate: config.commission_rate,
                collector_addr: config.collector_addr,
                spilt_to_collector: config.spilt_to_collector,
            })
            .unwrap(),
            funds: vec![],
        })))
        .add_attribute("action", "update_pair"))
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
    let config: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: config.owner.clone(),
        token_code_id: config.token_code_id,
        pair_code_id: config.pair_code_id,
        commission_rate: config.commission_rate,
        collector_addr: config.collector_addr,
        spilt_to_collector: config.spilt_to_collector,
    };

    Ok(resp)
}

pub fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_addr = PAIRS
        .load(deps.storage, &pair_key(&asset_infos))?
        .contract_addr;
    let pair_info = query_pair_info(deps, &pair_addr)?;
    Ok(pair_info)
}

pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after =
        start_after.map(|start_after| [start_after[0].clone(), start_after[1].clone()]);

    let pairs: Vec<PairInfo> = read_pairs(deps, start_after, limit)
        .iter()
        .map(|pair| query_pair_info(deps, &pair.contract_addr).unwrap())
        .collect();

    Ok(PairsResponse { pairs })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
