# TerraSwap LBP

Uniswap-inspired automated market-maker (AMM) protocol powered by Smart Contracts on the [Terra](https://terra.money) blockchain with LBP support.

## Contracts

| Name                                               | Description                                  |
| -------------------------------------------------- | -------------------------------------------- |
| [`terraswap_factory`](contracts/terraswap_factory) | Pool creation factory                        |
| [`terraswap_pair`](contracts/terraswap_pair)       | Pair with x\*y=k curve                       |
| [`terraswap_router`](contracts/terraswap_router)   | Multi-hop trade router                       |
| [`terraswap_token`](contracts/terraswap_token)     | CW20 (ERC20 equivalent) token implementation |

## Running this contract

You will need Rust 1.44.1+ with wasm32-unknown-unknown target installed.

You can run unit tests on this on each contracts directory via :

```
cargo unit-test
cargo integration-test
```

Once you are happy with the content, you can compile it to wasm on each contracts directory via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

Or for a production-ready (compressed) build, run the following from the repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.3
```

The optimized contracts are generated in the artifacts/ directory.

```
export WALLET="<mnemonic seed>"
export LCD_CLIENT_URL="https://bombay-lcd.terra.dev"
export CHAIN_ID="bombay-12"
node --loader ts-node/esm deploy_script.ts
```

## Liquidity Migration post LBP Completion :

- The `owner` of the LBP Pair can migrate Liquidity from the LBP Pool to either Astroport / Terraswap / Loop DEX via the Pool's `MigrateLiquidity`function.
- Pool address of Astroport / Terraswap / Loop DEX to which liquidity is to be migrated needs to be provided when calling the `MigrateLiquidity`function.
- Liquidity Providers to the LBP Pool can claim the equivalent LP shares of the new pool via calling the `ClaimNewShares` CW20Hook Msg via the LP token's `Send` function.
- When claiming the LP shares for the new pool, equivalent LP shares of the LBP pool are burnt.

`MigrateLiquidity`function can only be called after the completion of the LBP.
