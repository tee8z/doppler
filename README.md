<div style="text-align: center;">
<img src="logo.png" alt="Doppler Radar with Lightning Bolts" width="300"/>
</div>

## Doppler (A Lightning Domain-Specific Language)

- GOAL: Create a DSL for Lightning that enables users to easily experiment with and test against various Lightning implementations. Whether it's for discovery testing, inclusion in a comprehensive integration testing suite, or simply investigating the interactions between different components. Given the potential occurrence of intriguing and unique issues across different implementations of Lightning, this project aims to simplify the lives of application developers reliant on the network.

The DSL should empower developers to compose a concise script that configures an entire cluster of nodes running a single docker network, even if they are of different implementation types, to suit precise testing requirements. This should provide a sensation similar to working with a set of Lego blocks, where all the necessary components are at your fingertips, ready to be assembled based on the idea at hand.

Additionally, this DSL can be used against a cluster of remote LND nodes (more implementations will follow) to generate activity across them. This could be payments or channel related activity at the moment, but more work can be done to further expand this to include starting and stopping the remote nodes as well as funding them from a configured faucet. Still would not recommend running these doppler files on a mainnet cluster of nodes, but we wont stop you from being reckless :wink:

### Installing
##### Requires:
  - docker-compose:
    - https://docs.docker.com/compose/install/#scenario-one-install-docker-desktop
  - nvm:
    - https://github.com/nvm-sh/nvm

Download installing script
```sh
  curl --proto '=https' --tlsv1.2 -LsSf "https://raw.githubusercontent.com/tee8z/doppler/refs/heads/master/doppler-installer.sh" > doppler_installer.sh
```  
```sh
  chmod +x doppler_installer.sh
```
Execute installer with a [release tag](https://github.com/tee8z/doppler/releases) 
```sh
  ./doppler_installer.sh <release tag to install>
```
### Running after install:
To run with UI (navigate to `0.0.0.0:3000` in your browser to view the UI)
```sh
  cd $HOME/.doppler/<release tag> && node ./build
```
To run as cli
```sh
  cd $HOME/.doppler && doppler -h
```
More information on how to use this tool can be found here: [USAGE.md](./docs/USAGE.md)

#### Supports:
- [x] creating a cluster of bitcoind nodes
- [x] setting up one or many as miners on a provided time interval
- [x] setting up and funding a cluster of LND nodes, backed by a specified bitcoind node
- [x] outputing the cluster configuration as a docker-compose
- [x] making all the logs/data of the nodes available to the running of doppler
- [x] allowing to set values in LND's native configuration file
- [x] setup all the networking deterministically
- [x] OPEN_CHANNEL
- [x] SEND_LN (amp/keysend/bolt11 via subcommand)
- [x] SEND_ONCHAIN (only taproot addresses)
- [x] LOOP a set of commands over an optional interval
- [x] CLOSE_CHANNEL
- [X] support multiple node implementations (supports LND, CoreLN, Eclair)
- [x] add a cluster level UI to see how all the nodes connect (comes from https://github.com/litch/lightning-conformance/tree/master/operator)
- [x] SETTLE_HOLD_LN -- only works for LND nodes
- [x] SEND_HOLD_LN -- only works for LND nodes
- [x] SEND_COINS --  to send from a btc miner to any of the L2 node types (helpful in making sure there are enough funds for channels to open)
- [x] FORCE_CLOSE_CHANNEL - forces an L2 node to close a give channel
- [x] TAG - allows for hodl invoices payment hashes to be stored between doppler files being run, enable a shared state between files (these will be used in the future to enable closing a specific channel with another node instead of just picking one at random that the two nodes share)
- [x] STOP_BTC - stops a BTC container
- [x] START_BTC - starts a BTC container
- [x] STOP_LN - stops a LN container
- [x] START_LN - starts a LN container
- [x] WAIT BLOCKS - uses one of the hooked up LND nodes and waits to proceed in the script until a certain block height is reached

### Interesting Simulations
- Chain of force closures due to an inflight htlc: [force_closures](./examples/doppler_files/force_close/README.md)
- Executing a successful hodl invoice in LND nodes: [hodl_invoice](./examples/doppler_files/hold_invoices/README.md)
- Using external LND nodes and generating payment activity: [external_nodes](./examples/doppler_files/external_nodes/README.md)
- Running with multiple versions of lightning implementations: [different_versions](./examples/doppler_files/different_images/different_images.doppler)

### Acknowledgments
* Thank you [polar](https://github.com/jamaljsr/polar) for having such easy docker images to work with
* Thank you litch for creating [lightning-conformance](https://github.com/litch/lightning-conformance) which helped inspire this project
* Thank you w3irdrobot for getting this project started and helping pick the tool to build the grammar in rust

### Disclaimer
* This is very much a work in progress, but feel free to use what works today and PR's are welcome!
