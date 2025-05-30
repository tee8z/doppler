## How to create local docker image of mutinynet signet node
```
cd ./bitcoind_images
docker build -f "./Docker.mutinynet" -t mutinynet/bitcoind .
```
Then a docker image with the tag mutinynet/bitcoind should exist when doing this command:
```
docker images
```
## How to create local docker image of eclair that supports mutinynet signet
```
cd ./eclair_images
docker build -f "./Docker.eclair" -t mutinynet/eclair .
```
Then a docker image with the tag mutinynet/eclair should exist when doing this command:
```
docker images
```
## How to run image locally with doppler
From there, run the following script with doppler:
```
cargo run --bin doppler -- -f "examples/doppler_files/mutiny_cluster/setup.doppler" -l debug -n "signet"
```
And this will create 1 lnd and 1 coreln node backed by the one mutinynet node all controllable by doppler
Once you complete the setup.doppler file, time to go get coffee as it will take some time to download the current state of the signet
Feel free to follow along at: `watch -n 1 tail -n 100 data/bd1/.bitcoin/signet/debug.log`
Aliases for all the nodes can be found at: `scripts/aliases.sh`
