## Doppler (A Lightning Domain-Specific Language)

- GOAL: Create a DSL for Lightning that enables users to easily experiment with and test against various Lightning implementations. Whether it's for discovery testing, inclusion in a comprehensive integration testing suite, or simply investigating the interactions between different components. Given the potential occurrence of intriguing and unique issues across different implementations of Lightning, this project aims to simplify the lives of application developers reliant on the network.

The DSL should empower developers to compose a concise script that configures an entire cluster of nodes running a single docker network, even if they are of different implementation types, to suit precise testing requirements. This should provide a sensation similar to working with a set of Lego blocks, where all the necessary components are at your fingertips, ready to be assembled based on the idea at hand.

#### How to use:
- More information on how to use this tool can be found here: [USAGE.md](./docs/USAGE.md)

#### Supports:
- [x] creating a cluster of bitcoind nodes
- [x] setting up one or many as miners on a provided time interval
- [x] setting up and funding a cluster of lnd nodes, backed by a specified bitcoind node
- [x] outputing the cluster configuration as a docker-compose
- [x] making all the logs/data of the nodes available to the running of doppler
- [x] allowing to set values in LND's native configuration file
- [x] setup all the networking deterministically
- [x] OPEN_CHANNEL
- [x] SEND_LN (amp/keysend/bolt11 via subcommand)
- [x] SEND_ONCHAIN (only taproot addresses)
- [x] LOOP a set of commands over an optional interval
- [x] CLOSE_CHANNEL

##### TODO:
- [ ] support more than LND (should support coreln, eclair, and LDK)
- [ ] create new ldk image for [ldk-node](https://github.com/lightningdevkit/ldk-node)
- [ ] add a UI to each nodes to view in the browser (thunderhub, ride-the-lightning, etc.)
- [ ] add ability to have mutliple types of images/version and even allow user to use their own custom that aline with these instructions: [out-of-band](https://github.com/jamaljsr/polar/tree/master/docker#out-of-band-image-updates)
- [ ] make mining in the background optional after funding the lightning nodes
- [ ] allow to set how many funds individual lightning nodes get at start up (make optional)
- [ ] BUMP_FEE - need some way to identify the transaction created earlier in the script to bump
- [ ] CLOSE_CHANNEL - come up with grammar allowing scripts to close a specific channel
- [ ] BATCH_OPEN - build grammar for creating batch opening of channels
- [ ] add documentation and more examples showing how the grammar works/how to use doppler
- [ ] add a cluster level UI to see how all the nodes  connect (started, but need to finish in ./visualizer)
- [ ] add instructions on how to view the visualizer
- [ ] add "get_info" and some additional data to be shown in the visualizer
- [ ] improve ip address allocation

### Acknowledgments
* Thank you [polar](https://github.com/jamaljsr/polar) for having such easy docker images to work with
* Thank you litch for creating [lightning-conformance](https://github.com/litch/lightning-conformance) which helped inspire this project
* Thank you w3irdrobot for getting this project started and helping pick the tool to build the grammar in rust

### Disclaimer
* This is very much a work in progress, but feel free to use what works today and PR's are welcome!
