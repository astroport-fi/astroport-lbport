use cosmwasm_std::testing::{
    mock_env as mock_env_std, MockApi as MockApi_std, MockQuerier as MockQuerier_std,
    MockStorage as MockStorage_std,
};

use std::time::{SystemTime, UNIX_EPOCH};
use terraswap::asset::{AssetInfo, PairInfo, WeightedAssetInfo};
use terraswap::factory::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

use cosmwasm_std::{Addr, Uint128};
use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};

fn mock_app() -> App {
    let env = mock_env_std();
    let api = MockApi_std::default();
    let bank = BankKeeper::new();
    let storage = MockStorage_std::new();
    let terra_mock_querier = TerraMockQuerier::new(MockQuerier_std::new(&[]));

    App::new(api, env.block, bank, storage, terra_mock_querier)
}

fn store_factory_code(app: &mut App) -> u64 {
    let pair_contract = Box::new(
        ContractWrapper::new(
            terraswap_factory::contract::execute,
            terraswap_factory::contract::instantiate,
            terraswap_factory::contract::query,
        )
        .with_reply(terraswap_factory::contract::reply),
    );

    app.store_code(pair_contract)
}

fn store_pair_code(app: &mut App) -> u64 {
    let pair_contract = Box::new(
        ContractWrapper::new(
            terraswap_pair::contract::execute,
            terraswap_pair::contract::instantiate,
            terraswap_pair::contract::query,
        )
        .with_reply(terraswap_pair::contract::reply),
    );

    app.store_code(pair_contract)
}

fn store_token_code(app: &mut App) -> u64 {
    let terra_swap_token_contract = Box::new(ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    ));

    app.store_code(terra_swap_token_contract)
}

#[test]
fn create_and_register_pair_with_reply() {
    let mut app = mock_app();

    let factory_code_id = store_factory_code(&mut app);
    let pair_code_id = store_pair_code(&mut app);
    let token_code_id = store_token_code(&mut app);

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let end_time = start_time + 1000;
    let owner = "owner0000";

    let msg = InstantiateMsg {
        pair_code_id,
        token_code_id,
        owner: owner.to_string(),
    };

    // we can just call .unwrap() to assert this was a success
    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            Addr::unchecked(owner),
            &msg,
            &[],
            "TerraSwapFactory",
            None,
        )
        .unwrap();

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

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        start_time,
        end_time,
        description: Some(String::from("description")),
    };

    app.execute_contract(
        Addr::unchecked("addr0000"),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap();

    let res: PairInfo = app
        .wrap()
        .query_wasm_smart(
            factory_instance.clone(),
            &QueryMsg::Pair {
                asset_infos: [asset_infos[0].info.clone(), asset_infos[1].info.clone()],
            },
        )
        .unwrap();

    assert_eq!("Contract #0", factory_instance.to_string());
    assert_eq!("Contract #1", res.contract_addr.to_string());
    assert_eq!("Contract #2", res.liquidity_token.to_string());
    assert_eq!(start_time, res.start_time);
    assert_eq!(end_time, res.end_time);
    assert_eq!(asset_infos, res.asset_infos);
}

#[test]
fn update_config() {
    let mut app = mock_app();

    let factory_code_id = store_factory_code(&mut app);
    let pair_code_id = store_pair_code(&mut app);
    let token_code_id = store_token_code(&mut app);

    let owner = Addr::unchecked("owner");
    let new_owner = Addr::unchecked("new_owner");

    let msg = InstantiateMsg {
        pair_code_id,
        token_code_id,
        owner: owner.to_string(),
    };

    // we can just call .unwrap() to assert this was a success
    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            owner.clone(),
            &msg,
            &[],
            "TerraSwapFactory",
            None,
        )
        .unwrap();

    // update owner
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(new_owner.clone()),
        token_code_id: None,
        pair_code_id: None,
    };

    app.execute_contract(owner.clone(), factory_instance.clone(), &msg, &[])
        .unwrap();

    let msg = QueryMsg::Config {};
    let config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(token_code_id, config_res.token_code_id);
    assert_eq!(new_owner.clone(), config_res.owner);

    // update left items
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        token_code_id: Some(200u64),
        pair_code_id: Some(300u64),
    };

    app.execute_contract(new_owner, factory_instance.clone(), &msg, &[])
        .unwrap();

    let msg = QueryMsg::Config {};
    let config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(300u64, config_res.pair_code_id);

    // Unauthorized err
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        token_code_id: None,
        pair_code_id: None,
    };

    let res = app
        .execute_contract(owner, factory_instance, &msg, &[])
        .unwrap_err();
    assert_eq!(res.to_string(), "Unauthorized");
}
