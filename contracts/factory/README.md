# Factory

The factory contract can perform creation of astroport-lbp pair contract and also be used as directory contract for all pairs.

## InstantiateMsg

```json
{
  "pair_code_id": "123",
  "token_code_id": "123",
  "owner": "terra...",
}
```

## HandleMsg

### `update_config`

```json
{
  "update_config": {
    "owner": "terra...",
    "token_id": "123",
    "pair_code_id": "123"
  }
}
```

### `create_pair`

```json
{
  "create_pair": {
    "asset_infos": [
      {
        "info": {
          "token": {
            "contract_address": "terra..."
          }
        },
        "start_weight": 20,
        "end_weight": 30
      },
      {
        "info": {
          "native_token": {
            "denom": "uusd"
          }
        },
        "start_weight": 30,
        "end_weight": 20
      }
    ],
    "start_time": 1623337825,
    "end_time": 1623900000,
    "description": "this pair description is optional"
  }
}
```

### `register`

```json
{
  "register": {
    "asset_infos": [
      {
        "token": {
          "contract_address": "terra..."
        }
      },
      {
        "native_token": {
          "denom": "uusd"
        }
      }
    ]
  }
}
```

## QueryMsg

### `config`

```json
{
  "config": {}
}
```

### `pair`

```json
{
  "pair": {
    "asset_infos": [
      {
        "token": {
          "contract_address": "terra..."
        }
      },
      {
        "native_token": {
          "denom": "uusd"
        }
      }
    ]
  }
}
```

Register verified pair contract and token contract for pair contract creation. The sender will be the owner of the factory contract.

```rust
{
    /// Pair contract code ID, which is used to
    pub pair_code_id: u64,
    pub token_code_id: u64,
}
```

### UpdateConfig

The factory contract owner can change relevant code IDs for future pair contract creation.

```json
{
  "update_config": {
    "owner": Option<HumanAddr>,
    "pair_code_id": Option<u64>,
    "token_code_id": Option<u64>
  }
}
```

### Create Pair

When a user execute `CreatePair` operation, it creates `Pair` contract and `LP(liquidity provider)` token contract. It also creates not fully initialized `PairInfo`

```json
{
  "create_pair": {
    "asset_infos": [
      {
        "token": {
          "contract_addr": "terra1~~"
        }
      },
      {
        "native_token": {
          "denom": "uusd"
        }
      }
    ]
  }
}
```

### Register

When a user executes `CreatePair` operation, it passes `SubMsg` to `Pair` contract and `Pair` contract will invoke passed `SubMsg` registering created `Pair` contract to the factory. This operation is only allowed for a pair, which is not fully initialized.

Once a `Pair` contract invokes it, the sender address is registered as `Pair` contract address for the given asset_infos.

```json
{
  "register": {
    "asset_infos": [
      {
        "token": {
          "contract_addr": "terra1~~"
        }
      },
      {
        "native_token": {
          "denom": "uusd"
        }
      }
    ]
  }
}
```

### Unregister

The pair can be removed from factory using unregister function. Only the creator of pair is allowed to remove it.

```json
{
  "unregister": {
    "asset_infos": [
      {
        "token": {
          "contract_addr": "terra1~~"
        }
      },
      {
        "native_token": {
          "denom": "uusd"
        }
      }
    ]
  }
}
```
