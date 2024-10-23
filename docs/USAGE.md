### How to run the daemon

if using docker-compose
```
doppler -f "examples/doppler_files/many_lnd_channels/only_setup_network.doppler" -d
```
if using docker compose
```
doppler -f "examples/doppler_files/many_lnd_channels/only_setup_network.doppler"
```
- add a new .doppler file to create the cluster how you want
- examples of the possible valid grammar for the doppler files can be found in [doppler_files](../doppler_files/)

### How to use the UI (Script builder and node visualizer)

```
cd $HOME/.doppler/<release tag> && node ./build
```

App will be running on `localhost:3000`


### Example on how to hook up to remote LND nodes
- [remote_nodes](../examples/doppler_files/external_nodes/README.md)

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
RUST_LOG=TRACE parsetest -f "<path>/<to>/<file.doppler>"
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
doppler -f "doppler_files/only_setup_network.doppler" -l "debug" -d
```

once the cluster as gotten past the `up` command, you'll see a new file which contains the aliases and can be run like below

```
source ./scripts/aliases.sh
```

this allows you to call the containers like this from your current command prompt session:

```
lnd1 --help
```


### How to create a custom image:
We use the same docker images as polar, follow along with their instruction for lightning node implementations:
[custom lightning node](https://github.com/jamaljsr/polar/blob/master/docs/custom-nodes.md)
Here we create a custom docker file for bitcoind that should work with polar or doppler:
1. `git clone `
2. `cd bitcoind`
3. Create a new `Dockerfile` at the root of the repo:
```
FROM debian:stable-slim

ARG BITCOIN_VERSION
ENV PATH=/opt/bitcoin-${BITCOIN_VERSION}/bin:$PATH

RUN apt-get update -y \
  && apt-get install -y curl gosu \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN SYS_ARCH="$(uname -m)" \
  && curl -SLO https://bitcoincore.org/bin/bitcoin-core-${BITCOIN_VERSION}/bitcoin-${BITCOIN_VERSION}-${SYS_ARCH}-linux-gnu.tar.gz \
  && tar -xzf *.tar.gz -C /opt \
  && rm *.tar.gz

RUN curl -SLO https://raw.githubusercontent.com/bitcoin/bitcoin/master/contrib/bitcoin-cli.bash-completion \
  && mkdir /etc/bash_completion.d \
  && mv bitcoin-cli.bash-completion /etc/bash_completion.d/ \
  && curl -SLO https://raw.githubusercontent.com/scop/bash-completion/master/bash_completion \
  && mv bash_completion /usr/share/bash-completion/

COPY docker-entrypoint.sh /entrypoint.sh
COPY bashrc /home/bitcoin/.bashrc

RUN chmod a+x /entrypoint.sh

VOLUME ["/home/bitcoin/.bitcoin"]

EXPOSE 18443 18444 28334 28335

ENTRYPOINT ["/entrypoint.sh"]

CMD ["bitcoind"]
```
4. Create a new file named `docker-entrypoint.sh` with the following contents
```
#!/bin/sh
set -e

# containers on linux share file permissions with hosts.
# assigning the same uid/gid from the host user
# ensures that the files can be read/write from both sides
if ! id bitcoin > /dev/null 2>&1; then
  USERID=${USERID:-1000}
  GROUPID=${GROUPID:-1000}

  echo "adding user bitcoin ($USERID:$GROUPID)"
  groupadd -f -g $GROUPID bitcoin
  useradd -r -u $USERID -g $GROUPID bitcoin
  chown -R $USERID:$GROUPID /home/bitcoin
fi

if [ $(echo "$1" | cut -c1) = "-" ]; then
  echo "$0: assuming arguments for bitcoind"

  set -- bitcoind "$@"
fi

if [ "$1" = "bitcoind" ] || [ "$1" = "bitcoin-cli" ] || [ "$1" = "bitcoin-tx" ]; then
  echo "Running as bitcoin user: $@"
  exec gosu bitcoin "$@"
fi

echo "$@"
exec "$@"
```
5.
```
docker build -t bitcoind-master .
```
Use this doppler file as an example for using a custom image:
[example doppler](./examples/doppler_files/different_images.doppler)
