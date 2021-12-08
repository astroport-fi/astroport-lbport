use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Deps, Order, StdError};

use crate::error::ContractError;
use astroport_lbp::asset::AssetInfo;
use astroport_lbp::factory::FactoryPairInfo;
use cw_storage_plus::{Bound, Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub commission_rate: String,
    pub collector_addr: Option<Addr>,
    pub spilt_to_collector: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TmpPairInfo {
    pub pair_key: Vec<u8>,
    pub owner: Addr,
}

pub const TMP_PAIR_INFO: Item<TmpPairInfo> = Item::new("tmp_pair_info");

pub const CONFIG: Item<Config> = Item::new("config");
pub const PAIRS: Map<&[u8], FactoryPairInfo> = Map::new("pair_info");

pub fn pair_key(asset_infos: &[AssetInfo; 2]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat()
}

pub fn read_pair(
    deps: Deps,
    asset_infos: &[AssetInfo; 2],
) -> Result<FactoryPairInfo, ContractError> {
    match PAIRS.load(deps.storage, &pair_key(&asset_infos.clone())) {
        Ok(v) => Ok(v),
        Err(_e) => Err(StdError::generic_err("no pair data stored").into()),
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> Vec<FactoryPairInfo> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::exclusive);
    PAIRS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, pair_info) = item.unwrap();
            pair_info
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<[AssetInfo; 2]>) -> Option<Vec<u8>> {
    start_after.map(|asset_infos| {
        let mut asset_infos = asset_infos.to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let mut v = [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()]
            .concat()
            .as_slice()
            .to_vec();
        v.push(1);
        v
    })
}

pub fn read_tmp_pair(deps: Deps) -> Result<TmpPairInfo, ContractError> {
    Ok(TMP_PAIR_INFO.load(deps.storage)?)
}
