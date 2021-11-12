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
    "factory": {
      "ID": 176,
      "Addr": "terra1njg0ed835rzt2ee9yw0ek0kezadzv5zzqrwad6"
    },
    "pair": {
      "ID": 181
    },
    "router": {
      "ID": 183,
      "Addr": "terra12nz0lf4sg8lu8agxej8tjfmmecy5hy562kp9h4"
    },
    "token": {
      "ID": 185,
      "Addr": "terra1tv73ust2prgnp9njmzhy0g94sly2y5956ttm3m"
    }
  }
  ```