export const configDefault: Config = {
    terraswapFactoryConfig: {
        configInitMsg: {
            owner: '',
            pair_code_id: 0,
            token_code_id: 0,
        }
    },
    tokenConfig: {
        configInitMsg: {
            name: process.env.TOKEN_NAME!,
            symbol: process.env.TOKEN_SYMBOL!,
            decimals: Number(process.env.TOKEN_DECIMALS!),
            initial_balances: [
                {
                    address: process.env.TOKEN_INITIAL_AMOUNT_ADDRESS!,
                    amount: process.env.TOKEN_INITIAL_AMOUNT!
                },
            ],
            mint: {
                minter: process.env.TOKEN_MINTER!,
                cap: process.env.TOKEN_CAPACITY!
            }
        }
    },
    terraswapRouterConfig: {
        configInitMsg: {
            terraswap_factory: ''
        }
    },
    terraswapPairConfig: {
        configInitMsg: {
            asset_infos: [
                {
                    end_weight: '',
                    info: {
                        native_token: {
                            denom: ''
                        }
                    },
                    start_weight: ''
                },
                {
                    end_weight: '',
                    info: {
                        native_token: {
                            denom: ''
                        }
                    },
                    start_weight: ''
                },
            ],
            end_time: 0,
            start_time: 0,
            token_code_id: 0
        }
    },
}
