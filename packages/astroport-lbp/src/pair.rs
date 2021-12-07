use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{Asset, WeightedAsset, WeightedAssetInfo};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Asset infos
    pub asset_infos: [WeightedAssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,
    /// LBP start time
    pub start_time: u64,
    /// LBP end time
    pub end_time: Option<u64>,
    /// Pair description
    pub description: Option<String>,
    // Commission Rate for a pair
    pub commission_rate: String,
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
    /// Update Pair parameters like: end_date & commissison_fee
    UpdatePairConfigs {
        end_time: Option<Option<u64>>,
        commission_rate: Option<String>,
    },
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair {},
    Pool {},
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
