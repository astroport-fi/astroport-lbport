use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use astroport_lbp::asset::PairInfo;

pub const PAIR_INFO: Item<PairInfo> = Item::new("pair_info");
pub const MIGRATION_INFO: Item<MigrationInfo> = Item::new("migration_info");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationInfo {
    pub owner: Addr,
    pub pool_address: Option<Addr>,
    pub lp_token_address: Option<Addr>,
    pub prev_lp_tokens_total: Uint128,
    pub new_lp_tokens_minted: Uint128,
}
