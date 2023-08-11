### Doppler (A Lightning Domain-Specific Language)

- GOAL: Create a DSL grammar for Lightning that enables users to easily experiment with and test against various Lightning implementations. Whether it's for discovery testing, inclusion in a comprehensive integration testing suite, or simply investigating the interactions between different components. Given the potential occurrence of intriguing and unique issues across different implementations of Lightning, this project aims to simplify the lives of application developers reliant on the network.

The DSL should empower developers to compose a concise script that configures an entire cluster of nodes, even if they are of different implementation types, to suit precise testing requirements. This should provide a sensation similar to working with a set of Lego blocks, where all the necessary components are at your fingertips, ready to be assembled based on the idea at hand."

- More information on how to use this tool can be found here: [USAGE.md](./docs/USAGE.md)

#### Supports:
- [x] creating a cluster of bitcoind nodes
- [x] setting up one or many as miners on a provided time interval
- [x] setting up and funding a cluster of lnd nodes, backed by a specified bitcoind node
- [x] outputing the cluster configuration as a docker-compose
- [x] making all the logs/data of the nodes available to the running of doppler
- [x] allowing to set values in LND's native configuration file
- [x] setup all the networking deterministically
- [ ] Lightning node actions:
        - [x] OPEN_CHANNEL
        - [x] SEND_LN
        - [ ] SEND_ONCHAIN
        - [ ] CLOSE_CHANNEL

##### TODO:
- [ ] support more than LND (should support coreln, eclair, and some LDK implementation)
- [ ] add a UI to the nodes to view in the browser (thunderhub, ride-the-lightning, etc.)
- [ ] allow for repeate calls of commands every x interval (a user defined amount of time)
- [ ] make mining in the background optional after fundning the lightning nodes
- [ ] allow to set how many funds individual lightning nodes get at start up
- [ ] allow to re-use existing cluster to see the same pattern of events
- [ ] add pause between when the network gets stood up and the rest of the actions occur so an external application can be connected