use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{Asset, WeightedAsset, WeightedAssetInfo};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, Env, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner (address which can migrate liquidity)
    pub owner: String,
    /// Asset infos
    pub asset_infos: [WeightedAssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,
    /// LBP start time
    pub start_time: u64,
    /// LBP end time
    pub end_time: u64,
    /// Pair description
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// ProvideLiquidity a user provides pool liquidity
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<Addr>,
    },
    /// Function to migrate Liquidity after the LBP is over
    MigrateLiquidity {
        pool_address: String,
    },
    /// Callbacks; only callable by the contract itself.
    Callback(CallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<Addr>,
    },
    WithdrawLiquidity {},
    ClaimNewShares {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    UpdateStateOnLiquidityAdditionToPool {},
}

// Modified from
// https://github.com/CosmWasm/cosmwasm-plus/blob/v0.2.3/packages/cw20/src/receiver.rs#L15
impl CallbackMsg {
    pub fn to_cosmos_msg(self, env: &Env) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(self))?,
            funds: vec![],
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair {},
    Pool {},
    Migration {},
    Simulation { offer_asset: Asset, block_time: u64 },
    ReverseSimulation { ask_asset: Asset, block_time: u64 },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolResponse {
    pub assets: [WeightedAsset; 2],
    pub total_share: Uint128,
}

/// SimulationResponse returns swap simulation response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationInfoResponse {
    pub owner: Addr,
    pub pool_address: Option<Addr>,
    pub lp_token_address: Option<Addr>,
    pub prev_lp_tokens_total: Uint128,
    pub new_lp_tokens_minted: Uint128,
}

/// SimulationResponse returns swap simulation response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
    pub ask_weight: String,
    pub offer_weight: String,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
    pub ask_weight: String,
    pub offer_weight: String,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
