export const bombay_testnet: Config = { 

    terraswap_factory_InitMsg: {
        "config" : { 
            "owner": undefined,
            "token_code_id": 0,
            "pair_code_id": 0,
        } 
    },

    terraswap_pair_InitMsg: {
        "config" : { 
            "asset_infos": undefined,
            "token_code_id": undefined,
            "start_time": undefined,
            "end_time": undefined,
            "description": undefined,
        }
    },

    // lockdrop_InitMsg: {
    //     "config" : { 
    //         "owner": "",
    //         "init_timestamp": 0,
    //         "deposit_window": 86400,         
    //         "withdrawal_window": 86400,      
    //         "min_lock_duration": 1,         
    //         "max_lock_duration": 52,
    //         "weekly_multiplier": 1,    
    //         "weekly_divider": 12,    
    //     }
    // },



    // lockdropUpdateMsg: {
    //     "config" : { 
    //         "owner": undefined,
    //         "astro_token_address": undefined,
    //         "auction_contract_address": undefined,         
    //         "generator_address": undefined,      
    //         "lockdrop_incentives": undefined
    //     }
    // }
}







// interface LockdropInitMsg {
//     config : { 
//         owner?: string
//         init_timestamp: number
//         deposit_window: number 
//         withdrawal_window: number 
//         min_lock_duration: number 
//         max_lock_duration: number
//         weekly_multiplier: number
//         weekly_divider: number
//     }
// }

// interface LockdropUpdateMsg {
//     config : { 
//         owner?: string
//         astro_token_address?: string
//         auction_contract_address?: string 
//         generator_address?: string 
//         lockdrop_incentives?: string 
//     }
// }


interface FactoryInitMsg {
    config : { 
        owner?: string
        token_code_id?: number
        pair_code_id?: number
    }
}
interface PairInitMsg {
    config : { 
        asset_infos?: any
        token_code_id?: number
        start_time?: number
        end_time?: number
        description?: string 
    }
}

interface Config {
    terraswap_factory_InitMsg: FactoryInitMsg
    terraswap_pair_InitMsg: PairInitMsg
    // lockdrop_InitMsg: LockdropInitMsg
    // lockdropUpdateMsg: LockdropUpdateMsg
}