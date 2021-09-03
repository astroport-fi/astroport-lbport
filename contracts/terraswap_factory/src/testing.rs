use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, CosmosMsg, StdError, SubMsg, Uint128, WasmMsg,
};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::mock_querier::mock_dependencies;

use crate::state::read_pair;

use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use std::time::{SystemTime, UNIX_EPOCH};
use terraswap::asset::{AssetInfo, PairInfo, WeightedAssetInfo};
use terraswap::factory::{
    ConfigResponse, ExecuteMsg, FactoryPairInfo, InitMsg, PairsResponse, QueryMsg,
};
use terraswap::hook::InitHook;
use terraswap::pair::InitMsg as PairInitMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!(Addr::unchecked("addr0000"), config_res.owner);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // update owner
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(Addr::unchecked("addr0001")),
        pair_code_id: None,
        token_code_id: None,
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!(Addr::unchecked("addr0001"), config_res.owner);

    // update left items
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: Some(100u64),
        token_code_id: Some(200u64),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(100u64, config_res.pair_code_id);
    assert_eq!(Addr::unchecked("addr0001"), config_res.owner);

    // Unauthorzied err
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: None,
        token_code_id: None,
    };

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Err(ContractError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn create_pair() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::new(30),
            end_weight: Uint128::new(20),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            start_weight: Uint128::new(30),
            end_weight: Uint128::new(20),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        start_time,
        end_time,
        init_hook: None,
        description: Some(String::from("description")),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "asset0000-asset0001")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate {
            msg: to_binary(&PairInitMsg {
                asset_infos: asset_infos.clone(),
                token_code_id: 123u64,
                init_hook: Some(InitHook {
                    contract_addr: Addr::unchecked(MOCK_CONTRACT_ADDR),
                    msg: to_binary(&ExecuteMsg::Register {
                        asset_infos: asset_infos.clone()
                    })
                    .unwrap(),
                }),
                start_time,
                end_time,
                description: Some(String::from("description")),
            })
            .unwrap(),
            code_id: 321u64,
            funds: vec![],
            admin: None,
            label: String::from("Terraswap pair"),
        }))]
    );

    let raw_infos = [
        asset_infos[0].info.to_raw(deps.as_ref()).unwrap(),
        asset_infos[1].info.to_raw(deps.as_ref()).unwrap(),
    ];

    let pair_info = read_pair(deps.as_ref(), &raw_infos).unwrap();
    assert_eq!(pair_info.owner, Addr::unchecked("addr0000"));
    assert_eq!(pair_info.contract_addr, Addr::unchecked(""));
    assert_eq!(pair_info.start_time, start_time);
    assert_eq!(pair_info.end_time, end_time);
    assert_eq!(
        pair_info.asset_infos[0]
            .info
            .to_normal(deps.as_ref())
            .unwrap(),
        asset_infos[0].info
    );
    assert_eq!(
        pair_info.asset_infos[0].start_weight,
        asset_infos[0].start_weight
    );
    assert_eq!(
        pair_info.asset_infos[0].end_weight,
        asset_infos[0].end_weight
    );
    assert_eq!(
        pair_info.asset_infos[1]
            .info
            .to_normal(deps.as_ref())
            .unwrap(),
        asset_infos[1].info
    );
    assert_eq!(
        pair_info.asset_infos[1].start_weight,
        asset_infos[1].start_weight
    );
    assert_eq!(
        pair_info.asset_infos[1].end_weight,
        asset_infos[1].end_weight
    );
}

#[test]
fn register() {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        init_hook: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::new(30),
            end_weight: Uint128::new(20),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            start_weight: Uint128::new(30),
            end_weight: Uint128::new(20),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        init_hook: None,
        start_time,
        end_time,
        description: Some(String::from("description")),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    // register terraswap pair querier
    deps.querier.with_terraswap_pairs(&[(
        &Addr::unchecked("pair0000"),
        &PairInfo {
            asset_infos: [
                WeightedAssetInfo {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    start_weight: Uint128::new(30),
                    end_weight: Uint128::new(20),
                },
                WeightedAssetInfo {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    start_weight: Uint128::new(30),
                    end_weight: Uint128::new(20),
                },
            ],
            contract_addr: Addr::unchecked("pair0000"),
            liquidity_token: Addr::unchecked("liquidity0000"),
            start_time,
            end_time,
            description: Some(String::from("description")),
        },
    )]);

    let msg = ExecuteMsg::Register {
        asset_infos: asset_infos.clone(),
    };

    let env = mock_env();
    let info = mock_info("pair0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        env,
        QueryMsg::Pair {
            asset_infos: [asset_infos[0].info.clone(), asset_infos[1].info.clone()],
        },
    )
    .unwrap();

    let pair_res: FactoryPairInfo = from_binary(&query_res).unwrap();
    assert_eq!(
        pair_res,
        FactoryPairInfo {
            owner: Addr::unchecked("addr0000"),
            liquidity_token: Addr::unchecked("liquidity0000"),
            contract_addr: Addr::unchecked("pair0000"),
            asset_infos: asset_infos.clone(),
            start_time,
            end_time,
        }
    );

    let msg = ExecuteMsg::Register {
        asset_infos: [asset_infos[1].clone(), asset_infos[0].clone()],
    };

    let env = mock_env();
    let info = mock_info("pair0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Std(StdError::generic_err("Pair was already registered"))
    );

    // Store one more item to test query pairs
    let asset_infos_2 = [
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::new(30),
            end_weight: Uint128::new(20),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0002"),
            },
            start_weight: Uint128::new(30),
            end_weight: Uint128::new(20),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos_2.clone(),
        init_hook: None,
        start_time,
        end_time,
        description: Some(String::from("description")),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    // register terraswap pair querier
    deps.querier.with_terraswap_pairs(&[(
        &Addr::unchecked("pair0001"),
        &PairInfo {
            asset_infos: [
                WeightedAssetInfo {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    start_weight: Uint128::new(30),
                    end_weight: Uint128::new(20),
                },
                WeightedAssetInfo {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    start_weight: Uint128::new(30),
                    end_weight: Uint128::new(20),
                },
            ],
            contract_addr: Addr::unchecked("pair0001"),
            liquidity_token: Addr::unchecked("liquidity0001"),
            start_time,
            end_time,
            description: Some(String::from("description")),
        },
    )]);

    let msg = ExecuteMsg::Register {
        asset_infos: asset_infos_2.clone(),
    };

    let env = mock_env();
    let info = mock_info("pair0001", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let query_msg = QueryMsg::Pairs {
        start_after: None,
        limit: None,
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let pairs_res: PairsResponse = from_binary(&res).unwrap();
    assert_eq!(
        pairs_res.pairs,
        vec![
            FactoryPairInfo {
                owner: Addr::unchecked("addr0000"),
                liquidity_token: Addr::unchecked("liquidity0000"),
                contract_addr: Addr::unchecked("pair0000"),
                asset_infos: asset_infos.clone(),
                start_time,
                end_time,
            },
            FactoryPairInfo {
                owner: Addr::unchecked("addr0000"),
                liquidity_token: Addr::unchecked("liquidity0001"),
                contract_addr: Addr::unchecked("pair0001"),
                asset_infos: asset_infos_2.clone(),
                start_time,
                end_time,
            }
        ]
    );

    let query_msg = QueryMsg::Pairs {
        start_after: None,
        limit: Some(1),
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let pairs_res: PairsResponse = from_binary(&res).unwrap();
    assert_eq!(
        pairs_res.pairs,
        vec![FactoryPairInfo {
            owner: Addr::unchecked("addr0000"),
            liquidity_token: Addr::unchecked("liquidity0000"),
            contract_addr: Addr::unchecked("pair0000"),
            asset_infos: asset_infos.clone(),
            start_time,
            end_time,
        }]
    );

    let query_msg = QueryMsg::Pairs {
        start_after: Some([asset_infos[0].info.clone(), asset_infos[1].info.clone()]),
        limit: None,
    };

    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let pairs_res: PairsResponse = from_binary(&res).unwrap();
    assert_eq!(
        pairs_res.pairs,
        vec![FactoryPairInfo {
            owner: Addr::unchecked("addr0000"),
            liquidity_token: Addr::unchecked("liquidity0001"),
            contract_addr: Addr::unchecked("pair0001"),
            asset_infos: asset_infos_2.clone(),
            start_time,
            end_time,
        }]
    );

    // try unregister
    let msg = ExecuteMsg::Unregister {
        asset_infos: [
            AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
        ],
    };

    // check unauthorized
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let res = execute(deps.as_mut(), env, info, msg.clone());

    match res {
        Err(ContractError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unregister"),
            attr("pair", "asset0000-asset0001")
        ]
    );

    // query pairs to check that the pair has been unregistered
    let query_msg = QueryMsg::Pairs {
        start_after: None,
        limit: None,
    };

    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let pairs_res: PairsResponse = from_binary(&res).unwrap();

    assert_eq!(
        pairs_res.pairs,
        vec![FactoryPairInfo {
            owner: Addr::unchecked("addr0000"),
            liquidity_token: Addr::unchecked("liquidity0001"),
            contract_addr: Addr::unchecked("pair0001"),
            asset_infos: asset_infos_2.clone(),
            start_time,
            end_time,
        }]
    );
}
