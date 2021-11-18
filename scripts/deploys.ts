import 'dotenv/config'
import {
    Client,
    newClient,
    instantiateContract,
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

    if (!networkConfig.astroport_lbp_token.Addr) {
        if (!cfg.tokenConfig.configInitMsg.initial_balances[0].address) {
            cfg.tokenConfig.configInitMsg.initial_balances[0].address = cl.wallet.key.accAddress
        }

        if (!cfg.tokenConfig.configInitMsg.mint.minter) {
            cfg.tokenConfig.configInitMsg.mint.minter = cl.wallet.key.accAddress
        }

        networkConfig.astroport_lbp_token.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.astroport_lbp_token.ID,
            cfg.tokenConfig.configInitMsg
        );

        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup token ---> FINISH')
    } else {
        console.log('Token is already exists.\nAddr: ', networkConfig.astroport_lbp_token.Addr);
    }
}

async function setupAstroportPair(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.astroport_lbp_pair.Addr) {
        cfg.astroportPairConfig.configInitMsg.asset_infos = [
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

        cfg.astroportPairConfig.configInitMsg.end_time = Math.round(currTime) + 1000;
        cfg.astroportPairConfig.configInitMsg.start_time = Math.round(currTime);
        cfg.astroportPairConfig.configInitMsg.token_code_id = networkConfig.astroport_lbp_token.ID;

        networkConfig.astroport_lbp_pair.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.astroport_lbp_pair.ID,
            cfg.astroportPairConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup pair ---> FINISH')
    } else {
        console.log('Pair is already exists.\nAddr: ', networkConfig.astroport_lbp_pair.Addr);
    }
}

async function setupAstroportFactory(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.astroport_lbp_factory.Addr) {
        cfg.astroportFactoryConfig.configInitMsg.owner = process.env.FACTORY_OWNER! || cl.wallet.key.accAddress;
        cfg.astroportFactoryConfig.configInitMsg.token_code_id = networkConfig.astroport_lbp_token.ID;
        cfg.astroportFactoryConfig.configInitMsg.pair_code_id = networkConfig.astroport_lbp_pair.ID;

        networkConfig.astroport_lbp_factory.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.astroport_lbp_factory.ID,
            cfg.astroportFactoryConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup factory ---> FINISH')
    } else {
        console.log('Factory is already exists.\nAddr: ', networkConfig.astroport_lbp_factory.Addr);
    }
}

async function setupAstroportRouter(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.astroport_lbp_router.Addr) {
        cfg.astroportRouterConfig.configInitMsg.astroport_lbp_factory = networkConfig.astroport_lbp_factory.Addr;

        networkConfig.astroport_lbp_router.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.astroport_lbp_router.ID,
            cfg.astroportRouterConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup router ---> FINISH')
    } else {
        console.log('Router is already exists.\nAddr: ', networkConfig.astroport_lbp_router.Addr);
    }
}

async function main() {
    const client = newClient();
    let config: Config = configDefault;

    await uploadContracts(client);
    await setupAstroportFactory(client, config);
    await setupAstroportRouter(client, config);
    await setupAstroportPair(client, config);
    await setupToken(client, config);
}
main().catch(console.log)
