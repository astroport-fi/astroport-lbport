use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{attr, to_binary, Addr, CosmosMsg, StdError, SubMsg, Uint128, WasmMsg};

use cosmwasm_vm::testing::MOCK_CONTRACT_ADDR;
use std::time::{SystemTime, UNIX_EPOCH};
use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};
use terraswap::asset::AssetInfo;
use terraswap::asset::WeightedAssetInfo;
use terraswap::factory::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use terraswap::hook::InitHook;

fn mock_app() -> App {
    let api = MockApi::default();
    let env = mock_env();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();
    let tmq = TerraMockQuerier::new(MockQuerier::new(&[]));
    App::new(api, env.block, bank, storage, tmq)
}

fn token_code_id(app: &mut App) -> u64 {
    let token_contract = Box::new(ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    ));
    app.store_code(token_contract)
}

fn pair_code_id(app: &mut App) -> u64 {
    let pair_contract = Box::new(ContractWrapper::new(
        terraswap_pair::contract::execute,
        terraswap_pair::contract::instantiate,
        terraswap_pair::contract::query,
    ));
    app.store_code(pair_contract)
}

fn factory_code_id(app: &mut App) -> u64 {
    let factory_contract = Box::new(ContractWrapper::new(
        terraswap_factory::contract::execute,
        terraswap_factory::contract::instantiate,
        terraswap_factory::contract::query,
    ));
    app.store_code(factory_contract)
}

fn factory_init(app: &mut App, owner: Addr) -> Addr {
    let token_code_id = token_code_id(app);
    let pair_code_id = pair_code_id(app);
    let factory_code_id = factory_code_id(app);
    let msg = InstantiateMsg {
        token_code_id,
        pair_code_id,
        owner: owner.to_string(),
        init_hook: None,
    };
    app.instantiate_contract(factory_code_id, owner.clone(), &msg, &[], "factory", None)
        .unwrap()
}

#[test]
fn factory_init_test() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let factory = factory_init(&mut app, owner.clone());

    let msg = QueryMsg::Config {};
    let config: ConfigResponse = app.wrap().query_wasm_smart(&factory, &msg).unwrap();

    assert_eq!(1, config.token_code_id);
    assert_eq!(2, config.pair_code_id);
    assert_eq!(owner, config.owner);
}

#[test]
fn create_pair() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let factory = factory_init(&mut app, owner.clone());

    let asset_infos = [
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            start_weight: Uint128::new(1),
            end_weight: Uint128::new(1),
        },
        WeightedAssetInfo {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            start_weight: Uint128::new(1),
            end_weight: Uint128::new(1),
        },
    ];

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        start_time,
        end_time,
        init_hook: None,
        description: Some(String::from("description")),
    };

    let res = app.execute_contract(owner, factory, &msg, &[]).unwrap();

    assert_eq!(res.events[1].attributes[1], attr("action", "create_pair"));
    assert_eq!(
        res.events[1].attributes[2],
        attr("pair", "asset0000-asset0001")
    );

    // TODO
    // assert_eq!(
    //     res.data,
    //     vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate {
    //         msg: to_binary(&PairInstantiateMsg {
    //             asset_infos: asset_infos.clone(),
    //             token_code_id: 123u64,
    //             init_hook: Some(InitHook {
    //                 contract_addr: Addr::unchecked(MOCK_CONTRACT_ADDR),
    //                 msg: to_binary(&ExecuteMsg::Register {
    //                     asset_infos: asset_infos.clone()
    //                 })
    //                     .unwrap(),
    //             }),
    //
    //             start_time,
    //             end_time,
    //             description: Some(String::from("description")),
    //         })
    //             .unwrap(),
    //         code_id: 321u64,
    //         funds: vec![],
    //         label: String::from(""),
    //         admin: Some(owner.to_string()),
    //     }))]
    // );
}
