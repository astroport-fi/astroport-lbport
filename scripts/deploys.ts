import 'dotenv/config'
import { strictEqual } from "assert"
import {
    Client,
    newClient,
    instantiateContract,
    queryContract,
    uploadContract, writeNetworkConfig, readNetworkConfig,
} from './helpers.js'
import {configDefault} from "./deploy_configs.js";
import {join} from "path";
import {
    readdirSync,
} from 'fs'

const ARTIFACT_PATH = String(process.env.ARTIFACTS_PATH! || String("../artifacts"))
const ARTIFACT_EXTENSIONS = String(process.env.ARTIFACTS_EXTENSIONS! || String("wasm"))

async function uploadContracts(cl: Client) {
    const artifacts = readdirSync(ARTIFACT_PATH);

    for(let i=0; i<artifacts.length; i++) {
        if (artifacts[i].split('.').pop() == ARTIFACT_EXTENSIONS) {
            await uploadContractByName(cl, artifacts[i]);
        }
    }
    console.log('upload contracts ---> FINISH')
}

async function uploadContractByName(cl: Client, file_name: string) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);
    let key = file_name.split('.')[0];

    if (!networkConfig[`${key}`]) {
        let codeID = await uploadContract(cl.terra, cl.wallet, join(ARTIFACT_PATH, file_name));
        networkConfig[`${key}`] = {}
        networkConfig[`${key}`][`ID`] = codeID;
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log(`Contract: ${key} was uploaded.\nStore code: ${codeID}`);
    } else {
        console.log('Contract is already stored. StoreID: ', networkConfig[`${key}`].ID)
    }
}

async function setupToken(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.terraswap_token.Addr) {
        if (!cfg.tokenConfig.configInitMsg.initial_balances[0].address) {
            cfg.tokenConfig.configInitMsg.initial_balances[0].address = cl.wallet.key.accAddress
        }

        if (!cfg.tokenConfig.configInitMsg.mint.minter) {
            cfg.tokenConfig.configInitMsg.mint.minter = cl.wallet.key.accAddress
        }

        networkConfig.terraswap_token.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.terraswap_token.ID,
            cfg.tokenConfig.configInitMsg
        );

        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup token ---> FINISH')
    } else {
        console.log('Token is already exists.\nAddr: ', networkConfig.terraswap_token.Addr);
    }
}

async function setupTerraSwapPair(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.terraswap_pair.Addr) {
        cfg.terraswapPairConfig.configInitMsg.asset_infos = [
            {
                info:{
                    native_token: {
                        denom: "uusd".toString()
                    }
                },
                start_weight: "1",
                end_weight: "1"
            },
            {
                info:{
                    native_token: {
                        denom: "uluna".toString()
                    }
                },
                start_weight: "1",
                end_weight: "1"
            }
        ]

        let currTime = new Date().getTime() / 1000;

        cfg.terraswapPairConfig.configInitMsg.end_time = Math.round(currTime) + 1000;
        cfg.terraswapPairConfig.configInitMsg.start_time = Math.round(currTime);
        cfg.terraswapPairConfig.configInitMsg.token_code_id = networkConfig.terraswap_token.ID;

        networkConfig.terraswap_pair.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.terraswap_pair.ID,
            cfg.terraswapPairConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup pair ---> FINISH')
    } else {
        console.log('Pair is already exists.\nAddr: ', networkConfig.terraswap_pair.Addr);
    }
}

async function setupTerraSwapFactory(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.terraswap_factory.Addr) {
        cfg.terraswapFactoryConfig.configInitMsg.owner = process.env.FACTORY_OWNER! || cl.wallet.key.accAddress;
        cfg.terraswapFactoryConfig.configInitMsg.token_code_id = networkConfig.terraswap_token.ID;
        cfg.terraswapFactoryConfig.configInitMsg.pair_code_id = networkConfig.terraswap_pair.ID;

        networkConfig.terraswap_factory.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.terraswap_factory.ID,
            cfg.terraswapFactoryConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup factory ---> FINISH')
    } else {
        console.log('Factory is already exists.\nAddr: ', networkConfig.terraswap_factory.Addr);
    }
}

async function setupTerraSwapRouter(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.terraswap_router.Addr) {
        cfg.terraswapRouterConfig.configInitMsg.terraswap_factory = networkConfig.terraswap_factory.Addr;

        networkConfig.terraswap_router.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.terraswap_router.ID,
            cfg.terraswapRouterConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup router ---> FINISH')
    } else {
        console.log('Router is already exists.\nAddr: ', networkConfig.terraswap_router.Addr);
    }
}

async function main() {
    const client = newClient();
    let config: Config = configDefault;

    await uploadContracts(client);
    await setupTerraSwapFactory(client, config);
    await setupTerraSwapRouter(client, config);
    await setupTerraSwapPair(client, config);
    await setupToken(client, config);
}
main().catch(console.log)
