use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::querier::{query_balance, query_token_balance};
use cosmwasm_std::{to_binary, BankMsg, Coin, CosmosMsg, Decimal, Addr, StdError, StdResult, Uint128, WasmMsg, MessageInfo, Deps};
use terra_cosmwasm::TerraQuerier;
use cw20::Cw20ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WeightedAsset {
    pub info: AssetInfo,
    pub amount: Uint128,
    pub start_weight: Uint128,
    pub end_weight: Uint128,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.info)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WeightedAssetInfo {
    pub info: AssetInfo,
    pub start_weight: Uint128,
    pub end_weight: Uint128,
}

impl fmt::Display for WeightedAssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.info)
    }
}

impl WeightedAssetInfo {
    pub fn to_raw(
        &self,
        deps: Deps,
    ) -> StdResult<WeightedAssetInfoRaw> {
        Ok(WeightedAssetInfoRaw {
            info: self.info.to_raw(deps)?,
            start_weight: self.start_weight,
            end_weight: self.end_weight,
        })
    }
}

static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

impl Asset {
    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    pub fn compute_tax(
        &self,
        deps: Deps,
    ) -> StdResult<Uint128> {
        let amount = self.amount;
        if let AssetInfo::NativeToken { denom } = &self.info {
            if denom == "uluna" {
                Ok(Uint128::zero())
            } else {
                let terra_querier = TerraQuerier::new(&deps.querier);
                let tax_rate: Decimal = (terra_querier.query_tax_rate()?).rate;
                let tax_cap: Uint128 = (terra_querier.query_tax_cap(denom.to_string())?).cap;
                Ok(std::cmp::min(
                    (amount.checked_sub(
                        amount.multiply_ratio(
                            DECIMAL_FRACTION,
                            DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION,
                        )))?,
                    tax_cap,
                ))
            }
        } else {
            Ok(Uint128::zero())
        }
    }

    pub fn deduct_tax(
        &self,
        deps: Deps,
    ) -> StdResult<Coin> {
        let amount = self.amount;
        if let AssetInfo::NativeToken { denom } = &self.info {
            Ok(Coin {
                denom: denom.to_string(),
                amount: (amount.checked_sub( self.compute_tax(deps).unwrap())).unwrap(),
            })
        } else {
            Err(StdError::generic_err("cannot deduct tax from token asset"))
        }
    }

    pub fn into_msg(
        self,
        deps: Deps,
        _sender: Addr,
        recipient: Addr,
    ) -> StdResult<CosmosMsg> {
        let amount = self.amount;

        match &self.info {
            AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer { recipient: recipient.to_string(), amount })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken { .. } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![self.deduct_tax(deps)?],
            })),
        }
    }

    pub fn assert_sent_native_token_balance(&self, info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken { denom } = &self.info {
            match info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance missmatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance missmatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken { denom } => write!(f, "{}", denom),
            AssetInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}

impl AssetInfo {
    pub fn to_raw(
        &self,
        _deps: Deps,
    ) -> StdResult<AssetInfoRaw> {
        match self {
            AssetInfo::NativeToken { denom } => Ok(AssetInfoRaw::NativeToken {
                denom: denom.to_string(),
            }),
            AssetInfo::Token { contract_addr, .. } => Ok(AssetInfoRaw::Token {
                contract_addr: contract_addr.clone(),
            }),
        }
    }

    pub fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken { .. } => true,
            AssetInfo::Token { .. } => false,
        }
    }
    pub fn query_pool(
        &self,
        deps: Deps,
        pool_addr: &Addr,
    ) -> StdResult<Uint128> {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                query_token_balance(deps, &contract_addr, &pool_addr)
            }
            AssetInfo::NativeToken { denom, .. } => {
                query_balance(deps, pool_addr, denom.to_string())
            }
        }
    }

    pub fn equal(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfo::Token { contract_addr, .. } => self_contract_addr == contract_addr,
                    AssetInfo::NativeToken { .. } => false,
                }
            }
            AssetInfo::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfo::Token { .. } => false,
                    AssetInfo::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WeightedAssetInfoRaw {
    pub info: AssetInfoRaw,
    pub start_weight: Uint128,
    pub end_weight: Uint128,
}

impl WeightedAssetInfoRaw {
    pub fn to_normal(
        &self,
        _deps: Deps,
    ) -> StdResult<WeightedAssetInfo> {
        Ok(WeightedAssetInfo {
            info: match &self.info {
                AssetInfoRaw::NativeToken { denom } => AssetInfo::NativeToken {
                    denom: denom.to_string(),
                },
                AssetInfoRaw::Token { contract_addr } => AssetInfo::Token {
                    contract_addr: contract_addr.clone(),
                },
            },
            start_weight: self.start_weight,
            end_weight: self.end_weight,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AssetInfoRaw {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

impl AssetInfoRaw {
    pub fn to_normal(
        &self,
        _deps: Deps,
    ) -> StdResult<AssetInfo> {
        match self {
            AssetInfoRaw::NativeToken { denom } => Ok(AssetInfo::NativeToken {
                denom: denom.to_string(),
            }),
            AssetInfoRaw::Token { contract_addr } => Ok(AssetInfo::Token {
                contract_addr: contract_addr.clone(),
            }),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfoRaw::NativeToken { denom } => denom.as_bytes(),
            AssetInfoRaw::Token { contract_addr } => contract_addr.as_bytes(),
        }
    }

    pub fn equal(&self, asset: &AssetInfoRaw) -> bool {
        match self {
            AssetInfoRaw::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfoRaw::Token { contract_addr, .. } => {
                        self_contract_addr == contract_addr
                    }
                    AssetInfoRaw::NativeToken { .. } => false,
                }
            }
            AssetInfoRaw::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfoRaw::Token { .. } => false,
                    AssetInfoRaw::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairInfo {
    pub asset_infos: [WeightedAssetInfo; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,
    pub start_time: u64,
    pub end_time: u64,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairInfoRaw {
    pub asset_infos: [WeightedAssetInfoRaw; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,
    pub start_time: u64,
    pub end_time: u64,
    pub description: Option<String>,
}

impl PairInfoRaw {
    pub fn to_normal(
        &self,
        deps: Deps,
    ) -> StdResult<PairInfo> {
        Ok(PairInfo {
            liquidity_token: self.liquidity_token.clone(),
            start_time: self.start_time,
            contract_addr: self.contract_addr.clone(),
            asset_infos: [
                self.asset_infos[0].to_normal(deps)?,
                self.asset_infos[1].to_normal(deps)?,
            ],

            end_time: self.end_time,
            description: self.description.clone(),
        })
    }

    pub fn query_pools(
        &self,
        deps: Deps,
        contract_addr: &Addr,
    ) -> StdResult<[WeightedAsset; 2]> {
        let info_0: WeightedAssetInfo = self.asset_infos[0].to_normal(deps)?;
        let info_1: WeightedAssetInfo = self.asset_infos[1].to_normal(deps)?;
        Ok([
            WeightedAsset {
                amount: info_0.info.query_pool(deps, contract_addr)?,
                info: info_0.info,
                start_weight: info_0.start_weight,
                end_weight: info_0.end_weight,
            },
            WeightedAsset {
                amount: info_1.info.query_pool(deps, contract_addr)?,
                info: info_1.info,
                start_weight: info_1.start_weight,
                end_weight: info_1.end_weight,
            },
        ])
    }
}
