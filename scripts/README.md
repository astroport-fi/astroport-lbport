# Deploying a contracts to the Terra Station. #

## Bootstrap verifier
* This demo uses `package.json` to bootstrap all dependencies.
  ```shell
  $ cp sample.local.env .env
  $ npm install
  $ npm start
  ```

### Overview a scripts
* This script deploys all contracts to exists a TerraStation environment. 
  ```shell
  npm run build-app
  ```

### Output Result
* As a result, we will get a data file `<chain-id>.json` located in the root folder by default.
  ```json
  {
    "astroport_lbp_token": {
      "ID": 20116,
      "Addr": "terra1tkq9t7uveh8w97w3gukst6x0eyls7xemu3lc7f"
    },
    "astroport_lbp_factory": {
      "ID": 20117,
      "Addr": "terra1yknkheg3daqs3ugfppcsprsl09shxgju2rajw3"
    },
    "astroport_lbp_pair": {
      "ID": 20118,
      "Addr": "terra10tundj5757wtjzvs3g4h48kcvvpw59s6jsyvet"
    },
    "astroport_lbp_router": {
      "ID": 20119,
      "Addr": "terra1a8gkkxfzxdhazyaxnyj8taknkmumknd7c3u3w4"
    }
  }```