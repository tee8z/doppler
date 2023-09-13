### How to run

```
cargo run --bin doppler -- -f "doppler_files/only_setup_network.doppler" -d
```

- add a new .doppler file to create the cluster how you want
- examples of the possible valid grammar for the doppler files can be found in [doppler_files](../doppler_files/)

### Start Blockly editor UI

```
cd ui
yarn
yarn dev
```

App will be running on `localhost:5173`

### How to view logs of container

```
docker logs doppler-<node name>
```

### Clear docker cluster

```
./scripts/docker_clear.sh
```

### Clear docker data

```
./scripts/volumes_clear.sh
```

### Reset cluster with current script

```
./scripts/reset.sh
```

### How to test parse grammar

```
RUST_LOG=TRACE cargo run --bin parsetest
```

### Permissions

- If on linux, make sure your user has permission to group 1000 and user 1000, if they are different, update the varaibles in the .env file
- If you are running rootless docker on linux, make sure the user the rootless docker daemon is under matches your current user, otherwise it will create folders you do not have access to

### Call via local lncli

`lncli --lnddir=/path/to/repo/doppler/data/lnd1/.lnd --network=regtest --rpcserver=10.5.0.6:10000 --macaroonpath=/path/to/repo/doppler/data/lnd1/.lnd/da
ta/chain/bitcoin/regtest/admin.macaroon getinfo`

### Call via local curl

```
MACAROON_HEADER="Grpc-Metadata-macaroon: $(xxd -ps -u -c 1000 /path/to/repo/doppler/data/lnd1/.lnd/data/chain/bitcoin/regtest/admin.macaroon)"
curl --cacert /path/to/repo/doppler/data/lnd1/.lnd/tls.cert  --header "$MACAROON_HEADER"  https://10.5.0.6:8080/v1/graph
```

### To use alias for the containers

run the doppler with the following flag `-a` in one of the files that sets up the network

```
cargo run --bin doppler -- -f "doppler_files/only_setup_network.doppler" -l "debug" -d
```

once the cluster as gotten past the `up` command, you'll see a new file which contains the aliases and can be run like below

```
source ./scripts/container_aliases.sh
```

this allows you to call the containers like this from your current command prompt session:

```
lnd1 --help
```

### Config options for eclair nodes:

https://github.com/ACINQ/eclair/blob/master/eclair-core/src/main/resources/reference.conf
