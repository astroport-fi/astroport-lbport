interface TerraSwapFactoryConfig {
    configInitMsg: {
        owner: string,
        pair_code_id: number,
        token_code_id: number,
    }
}

interface TerraSwapRouterConfig {
    configInitMsg: {
        terraswap_factory: string
    }
}

interface TerraSwapPairConfig {
    configInitMsg: {
        asset_infos: [
            {
                end_weight: string,
                info: {
                    native_token: {
                        denom: string
                    }
                },
                start_weight: string,
            },
            {
                end_weight: string,
                info: {
                    native_token: {
                        denom: string
                    }
                },
                start_weight: string,
            },
        ],
        end_time: number,
        start_time: number,
        token_code_id: number,

    }
}

interface TokenConfig {
    configInitMsg: {
        name: string,
        symbol: string,
        decimals: number,
        initial_balances: [
            {
                address: string,
                amount: string
            }
        ],
        mint: {
            minter: string,
            cap: string
        }
    }
}

interface Config {
    tokenConfig: TokenConfig,
    terraswapFactoryConfig: TerraSwapFactoryConfig,
    terraswapRouterConfig: TerraSwapRouterConfig,
    terraswapPairConfig: TerraSwapPairConfig,
}
