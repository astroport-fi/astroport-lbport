use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use cosmwasm_std::Response;

use astroport_lbp::asset::PairInfo;
use astroport_lbp::pair::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolResponse, QueryMsg, ReverseSimulationResponse,
    SimulationResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(Cw20HookMsg), &out_dir);
    export_schema(&schema_for!(Response), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(PairInfo), &out_dir);
    export_schema(&schema_for!(PoolResponse), &out_dir);
    export_schema(&schema_for!(ReverseSimulationResponse), &out_dir);
    export_schema(&schema_for!(SimulationResponse), &out_dir);
}