import 'dotenv/config'
import {
  uploadContract,instantiateContract,
    deployContract,
    executeContract,
    newClient,
    readArtifact,
    writeArtifact,toEncodedBinary,
    Client
  } from "./helpers/helpers.js";
import { bombay_testnet } from "./deploy_configs.js";
import { join } from "path"
import { Coin } from '@terra-money/terra.js';


const ARTIFACTS_PATH = "../artifacts"


const FROM_TIMESTAMP = parseInt((Date.now()/1000).toFixed(0)) +  150

async function transferAmount(cl: Client, sender: string, recipient: string, amount: String) {
  let out: any, msg: any
  msg = { transfer: { recipient: recipient, amount: amount } }
  console.log('execute', sender, JSON.stringify(msg))
  out = await executeContract(cl.terra, cl.wallet, sender, msg)
  console.log(out.txhash)
}

async function main() {

  const {terra, wallet} = newClient()
  console.log(`chainID: ${terra.config.chainID} wallet: ${wallet.key.accAddress}`)
  console.log(`FROM_TIMESTAMP: ${FROM_TIMESTAMP} `)
  FROM_TIMESTAMP

  const network = readArtifact(terra.config.chainID)
  console.log('network:', network)

  network.terraswap_pair_address = ""
  /*************************************** DEPLOYMENT :: TERRASWAP FACTORY CONTRACT  *****************************************/


  // let terraswap_token_id = await uploadContract( terra, wallet, join(ARTIFACTS_PATH, 'terraswap_token.wasm'))
  // let terraswap_pair_id =  await uploadContract( terra, wallet, join(ARTIFACTS_PATH, 'terraswap_pair.wasm'))

  // bombay_testnet.terraswap_factory_InitMsg.config.owner = wallet.key.accAddress;
  // bombay_testnet.terraswap_factory_InitMsg.config.token_code_id = terraswap_token_id;
  // bombay_testnet.terraswap_factory_InitMsg.config.pair_code_id = terraswap_pair_id;
  // console.log(` ${bombay_testnet.terraswap_factory_InitMsg.config}`)


  // network.terraswap_factory_address = await deployContract(terra, wallet, join(ARTIFACTS_PATH, 'terraswap_factory.wasm'),  bombay_testnet.terraswap_factory_InitMsg.config)
  // console.log(`Factory Contract Address : ${network.terraswap_factory_address}`)

  /*************************************** Deploy CW20 (WHALE Token) Contract *****************************************/

    // Deploy WHALE Token
    // let whale_token_config = { "name": "WHALE",
    //                         "symbol": "WHALE",
    //                         "decimals": 6,
    //                         "initial_balances": [ {"address":wallet.key.accAddress, "amount":"1000000000000000"}], 
    //                         "mint": { "minter":wallet.key.accAddress, "cap":"1000000000000000"}
    //                        }
    // network.whale_token_address = await deployContract(terra, wallet, join(ARTIFACTS_PATH, 'cw20_token.wasm'),  whale_token_config )
    // console.log(`WHALE Token deployed successfully, address : ${network.whale_token_address}`);
    


  /*************************************** CREATE PAIR :: TERRASWAP PAIR CONTRACT  *****************************************/

  // let create_pair = { "create_pair" : {
  //                           "owner": wallet.key.accAddress,
  //                           "asset_infos": [
  //                             {   "info":{ "token": {"contract_addr":network.whale_token_address} }, "start_weight": "20", "end_weight":"50" } ,
  //                             {   "info":{"native_token": {"denom": "uusd"} }, "start_weight": "80", "end_weight":"50"  } ,
  //                           ],
  //                           "start_time": FROM_TIMESTAMP,
  //                           "end_time": FROM_TIMESTAMP + 59,
  //                           "description": "testing pair creation"
  //                         }                        
  //                   }

  // await executeContract(terra, wallet, network.terraswap_factory_address,  create_pair)
  // console.log(`PAIR Contract Address : ${network.terraswap_pair_address}`)

  /*************************************** PROVIDE LIQUIDITY  *****************************************/

  let inital_whale_liquidity_to_lbp = 1000000000000
  let inital_ust_liquidity_to_lbp = 1000000000  
  // await executeContract( terra, wallet, network.whale_token_address ,   { "increase_allowance": { "spender": network.terraswap_pair_address, amount: String(inital_whale_liquidity_to_lbp) } } )

  // await executeContract( terra, wallet, network.terraswap_pair_address ,   { "provide_liquidity": { "assets": [ { "info": { "native_token": { "denom": "uusd" }  }, "amount": String(inital_ust_liquidity_to_lbp)   },
  //                                                                                                                       { "info": { "token": { "contract_addr": network.whale_token_address }  }   , "amount": String(inital_whale_liquidity_to_lbp)   },
  //                                                                                                                     ],
  //                                                                                                             "slippage_tolerance": undefined,
                                                                                                              
  //                                                                                                   }
  //                                                                                   }, 
  //                                                                                   [new Coin("uusd", inital_ust_liquidity_to_lbp)]
  //                       )

  /*************************************** CREATE PAIR (to which liquidity is to be migrated) ON DEX  *****************************************/
  // await executeContract( terra, wallet, "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf" ,   { "create_pair": {
  //                                                                                               "asset_infos": [  {  "token": {"contract_addr":network.whale_token_address} } ,
  //                                                                                                                 {  "native_token": {"denom": "uusd"} } ,
  //                                                                                                             ],
  //                                                                                             } 
  //                                                                                         } )



  /*************************************** MIGRATE LIQUIDITY  *****************************************/
  // await executeContract( terra, wallet, network.terraswap_pair_address ,   { "migrate_liquidity": { "pool_address" : "terra1ls9hrg5f370v4gjc52wzljunlyyj3tmq9jyghp" } } )

  /*************************************** CLAIM NEW LP TOKENS  *****************************************/

  // await executeContract( terra, wallet, "terra149m3hx4we4k9ddahwted83adp9knss3sq4n33z" ,   { "send" : { "contract": network.terraswap_pair_address,
  //                                                                                                       "amount" : String(31622576601),        
  //                                                                                                       "msg":  toEncodedBinary({ "claim_new_shares": {} })
  //                                                                                                     } 
  //                                                                                           } )


  writeArtifact(network, terra.config.chainID)
  console.log('FINISH')
}

main().catch(console.log)

