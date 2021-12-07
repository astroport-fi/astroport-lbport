use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Addr, Binary, Coin, Decimal, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use astroport_lbp::asset::{Asset, AssetInfo};
use astroport_lbp::factory::FactoryPairInfo;
use astroport_lbp::pair::SimulationResponse;
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use terra_cosmwasm::{
    SwapResponse, TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair { asset_infos: [AssetInfo; 2] },
    Simulation { offer_asset: Asset },
}

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    token_querier: TokenQuerier,
    tax_querier: TaxQuerier,
    astroport_lbp_factory_querier: AstroportFactoryQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<Addr, HashMap<Addr, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&Addr, &[(&Addr, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&Addr, &[(&Addr, &Uint128)])],
) -> HashMap<Addr, HashMap<Addr, Uint128>> {
    let mut balances_map: HashMap<Addr, HashMap<Addr, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<Addr, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert((*addr).clone(), **balance);
        }

        balances_map.insert(
            Addr::unchecked((*contract_addr).clone()),
            contract_balances_map,
        );
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

#[derive(Clone, Default)]
pub struct AstroportFactoryQuerier {
    pairs: HashMap<String, FactoryPairInfo>,
}

impl AstroportFactoryQuerier {
    pub fn new(pairs: &[(&String, &FactoryPairInfo)]) -> Self {
        AstroportFactoryQuerier {
            pairs: pairs_to_map(pairs),
        }
    }
}

pub(crate) fn pairs_to_map(
    pairs: &[(&String, &FactoryPairInfo)],
) -> HashMap<String, FactoryPairInfo> {
    let mut pairs_map: HashMap<String, FactoryPairInfo> = HashMap::new();
    for (key, pair) in pairs.iter() {
        pairs_map.insert(key.to_string(), (*pair).clone());
    }
    pairs_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MockQueryMsg {
    Price {},
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if route == &TerraRoute::Treasury {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            SystemResult::Ok(to_binary(&res).into())
                        }
                        TerraQuery::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            SystemResult::Ok(to_binary(&res).into())
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else if route == &TerraRoute::Market {
                    match query_data {
                        TerraQuery::Swap {
                            offer_coin,
                            ask_denom: _,
                        } => {
                            let res = SwapResponse {
                                receive: offer_coin.clone(),
                            };
                            SystemResult::Ok(to_binary(&res).into())
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                if contract_addr.to_string().starts_with("token")
                    || contract_addr.to_string().starts_with("asset")
                {
                    self.handle_cw20(&Addr::unchecked(contract_addr), msg)
                } else {
                    self.handle_default(msg)
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_default(&self, msg: &Binary) -> QuerierResult {
        match from_binary(&msg).unwrap() {
            QueryMsg::Pair { asset_infos } => {
                let key = asset_infos[0].to_string() + asset_infos[1].to_string().as_str();
                match self.astroport_lbp_factory_querier.pairs.get(&key) {
                    Some(_v) => SystemResult::Ok(
                        to_binary(&FactoryPairInfo {
                            owner: Addr::unchecked("owner0000"),
                            contract_addr: Addr::unchecked("pair"),
                        })
                        .into(),
                    ),
                    None => SystemResult::Err(SystemError::InvalidRequest {
                        error: "No pair info exists".to_string(),
                        request: msg.as_slice().into(),
                    }),
                }
            }
            QueryMsg::Simulation { offer_asset } => SystemResult::Ok(
                to_binary(&SimulationResponse {
                    return_amount: offer_asset.amount,
                    commission_amount: Uint128::zero(),
                    ask_weight: "".to_string(),
                    spread_amount: Uint128::zero(),
                    offer_weight: "".to_string(),
                })
                .into(),
            ),
        }
    }

    fn handle_cw20(&self, contract_addr: &Addr, msg: &Binary) -> QuerierResult {
        match from_binary(&msg).unwrap() {
            Cw20QueryMsg::TokenInfo {} => {
                let balances: &HashMap<Addr, Uint128> =
                    match self.token_querier.balances.get(contract_addr) {
                        Some(balances) => balances,
                        None => {
                            return SystemResult::Err(SystemError::Unknown {});
                        }
                    };

                let mut total_supply = Uint128::zero();

                for balance in balances {
                    total_supply += *balance.1;
                }

                SystemResult::Ok(
                    to_binary(&TokenInfoResponse {
                        name: "mAPPL".to_string(),
                        symbol: "mAPPL".to_string(),
                        decimals: 6,
                        total_supply: total_supply,
                    })
                    .into(),
                )
            }
            Cw20QueryMsg::Balance { address } => {
                let balances: &HashMap<Addr, Uint128> =
                    match self.token_querier.balances.get(contract_addr) {
                        Some(balances) => balances,
                        None => {
                            return SystemResult::Err(SystemError::Unknown {});
                        }
                    };

                let balance = match balances.get(&Addr::unchecked(address)) {
                    Some(v) => v,
                    None => {
                        return SystemResult::Err(SystemError::Unknown {});
                    }
                };

                SystemResult::Ok(to_binary(&BalanceResponse { balance: *balance }).into())
            }
            _ => panic!("DO NOT ENTER HERE"),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            tax_querier: TaxQuerier::default(),
            astroport_lbp_factory_querier: AstroportFactoryQuerier::default(),
        }
    }

    pub fn with_balance(&mut self, balances: &[(&Addr, &[Coin])]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.to_string(), balance.to_vec());
        }
    }

    pub fn with_token_balances(&mut self, balances: &[(&Addr, &[(&Addr, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    pub fn with_astroport_lbp_pairs(&mut self, pairs: &[(&String, &FactoryPairInfo)]) {
        self.astroport_lbp_factory_querier = AstroportFactoryQuerier::new(pairs);
    }
}
