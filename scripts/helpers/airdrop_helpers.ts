import {executeContract} from "./helpers.js";
import { LCDClient, Wallet, LocalTerra} from "@terra-money/terra.js";

//-----------------------------------------------------

// ------ ExecuteContract :: Function signatures ------
// - updateAirdropConfig
// - claimAirdrop
// - transferAstroByAdminFromAirdropContract
//------------------------------------------------------
//------------------------------------------------------
// ----------- Queries :: Function signatures ----------
// - getAirdropConfig(terra, airdropContractAdr) --> Returns configuration
// - isAirdropClaimed(terra, airdropContractAdr, address) --> Returns true if airdrop already claimed, else false
//------------------------------------------------------


// UPDATE TERRA MERKLE ROOTS : EXECUTE TX
export async function updateAirdropConfig( terra: LocalTerra | LCDClient, wallet:Wallet, airdropContractAdr: string, new_config: any) {
    let resp = await executeContract(terra, wallet, airdropContractAdr, new_config );
}
  

// AIRDROP CLAIM BY TERRA USER : EXECUTE TX
export async function claimAirdrop( terra: LocalTerra | LCDClient, wallet:Wallet, airdropContractAdr: string,  claim_amount: number, merkle_proof: any, root_index: number  ) {
    if ( merkle_proof.length > 1 ) {
      let claim_for_terra_msg = { "claim_by_terra_user": {'claim_amount': claim_amount.toString(), 'merkle_proof': merkle_proof, "root_index": root_index }};
        let resp = await executeContract(terra, wallet, airdropContractAdr, claim_for_terra_msg );
        return resp;        
    } else {
        console.log("AIRDROP TERRA CLAIM :: INVALID MERKLE PROOF");
    }
}
  
  

// TRANSFER ASTRO TOKENS : EXECUTE TX
export async function transferAstroByAdminFromAirdropContract( terra: LocalTerra | LCDClient, wallet:Wallet, airdropContractAdr: string, recepient: string, amount: number) {
    try {
        let transfer_astro_msg = { "transfer_astro_tokens": {'recepient': recepient, 'amount': amount.toString() }};
        let resp = await executeContract(terra, wallet, airdropContractAdr, transfer_astro_msg );
        return resp;        
    }
    catch {
        console.log("ERROR IN transferAstroByAdminFromAirdropContract function")
    }        
}


// GET CONFIG : CONTRACT QUERY
export async function getAirdropConfig(  terra: LocalTerra | LCDClient, airdropContractAdr: string) {
    try {
        let res = await terra.wasm.contractQuery(airdropContractAdr, { "config": {} })
        return res;
    }
    catch {
        console.log("ERROR IN getAirdropConfig QUERY")
    }    
}

// IS CLAIMED : CONTRACT QUERY
export async function isAirdropClaimed(  terra: LocalTerra | LCDClient, airdropContractAdr: string, address: string ) {
    let is_claimed_msg = { "is_claimed": {'address': address }};
    try {
        let res = await terra.wasm.contractQuery(airdropContractAdr, is_claimed_msg)
        return res;
    }
    catch {
        console.log("ERROR IN isAirdropClaimed QUERY")
    }
    
}
  

  


// // GET NATIVE TOKEN BALANCE
// export async function getUserNativeAssetBalance(terra, native_asset, wallet_addr) {
//     let res = await terra.bank.balance(  wallet_addr );
//     let balances = JSON.parse(JSON.parse(JSON.stringify( res )));
//     for (let i=0; i<balances.length;i++) {
//         if ( balances[i].denom == native_asset ) {
//             return balances[i].amount;
//         }
//     }    
//     return 0;
// }


// function print_events(response) {
//     if (response.height > 0) {
//       let events_array = JSON.parse(response["raw_log"])[0]["events"];
//       let attributes = events_array[1]["attributes"];
//       for (let i=0; i < attributes.length; i++ ) {
//         console.log(attributes[i]);
//       }
//     }
//   }


