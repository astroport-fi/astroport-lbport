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

async function uploadContracts(cl: Client) {
    const artifacts = readdirSync(process.env.ARTIFACTS_PATH!);
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    for(let i=0; i<artifacts.length; i++){
        if (artifacts[i].split('.').pop() == process.env.ARTIFACTS_EXTENSIONS!) {
            let codeID = await uploadContract(cl.terra, cl.wallet,
                join(process.env.ARTIFACTS_PATH!, artifacts[i])
            );
            console.log(`Contract: ${artifacts[i].split('.')[0]} was uploaded.\nStore code: ${codeID}`);
            networkConfig[`${artifacts[i].split('.')[0].split('_').pop()}`] = {}
            networkConfig[`${artifacts[i].split('.')[0].split('_').pop()}`][`ID`] = codeID;
        }
    }
    writeNetworkConfig(networkConfig, cl.terra.config.chainID)
    console.log('upload contracts ---> FINISH')
}

async function setupToken(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.token.Addr) {
        if (!cfg.tokenConfig.configInitMsg.initial_balances[0].address) {
            cfg.tokenConfig.configInitMsg.initial_balances[0].address = cl.wallet.key.accAddress
        }

        if (!cfg.tokenConfig.configInitMsg.mint.minter) {
            cfg.tokenConfig.configInitMsg.mint.minter = cl.wallet.key.accAddress
        }

        networkConfig.token.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.token.ID,
            cfg.tokenConfig.configInitMsg
        );

        let balance = await queryContract(cl.terra, networkConfig.token.Addr, {
            balance: {address: cl.wallet.key.accAddress}
        })

        // Validate token balance
        strictEqual(balance.balance, process.env.TOKEN_INITIAL_AMOUNT!)
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup token ---> FINISH')
    }
}

async function setupTerraSwapPair(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.pair.Addr) {
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
        console.log("curr time: ", Math.round(currTime));
        cfg.terraswapPairConfig.configInitMsg.end_time = Math.round(currTime) + 1000;
        cfg.terraswapPairConfig.configInitMsg.start_time = Math.round(currTime);
        cfg.terraswapPairConfig.configInitMsg.token_code_id = networkConfig.token.ID

        networkConfig.pair.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.pair.ID,
            cfg.terraswapPairConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup factory ---> FINISH')
    }
}

async function setupTerraSwapFactory(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.factory.Addr) {
        cfg.terraswapFactoryConfig.configInitMsg.owner = process.env.FACTORY_OWNER! || cl.wallet.key.accAddress;
        cfg.terraswapFactoryConfig.configInitMsg.token_code_id = networkConfig.token.ID
        cfg.terraswapFactoryConfig.configInitMsg.pair_code_id = networkConfig.pair.ID

        networkConfig.factory.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.factory.ID,
            cfg.terraswapFactoryConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup factory ---> FINISH')
    }
}

async function setupTerraSwapRouter(cl: Client, cfg: Config) {
    const networkConfig = readNetworkConfig(cl.terra.config.chainID);

    if (!networkConfig.router.Addr) {
        cfg.terraswapRouterConfig.configInitMsg.terraswap_factory = networkConfig.factory.Addr

        networkConfig.router.Addr = await instantiateContract(
            cl.terra,
            cl.wallet,
            networkConfig.router.ID,
            cfg.terraswapRouterConfig.configInitMsg
        );
        writeNetworkConfig(networkConfig, cl.terra.config.chainID)
        console.log('setup router ---> FINISH')
    }
}

async function main() {
    const client = newClient();
    let config: Config = configDefault

    await uploadContracts(client);
    await setupTerraSwapFactory(client, config);
    await setupTerraSwapRouter(client, config);
    // await setupToken(client, config);
    // await setupTerraSwapPair(client, config);
}
main().catch(console.log)
