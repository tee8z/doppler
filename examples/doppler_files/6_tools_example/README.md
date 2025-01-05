### Esplora on bitcoind in cluster

Once the simulation has finished coming up after you've hit run, you will be able to access the esplora instance by going directly to the host url shown by clicking the "show connections" button on the left side of the screen. It will look something like:
```
"esp": {
  "host": "http://localhost:9102",
  "type": "esplora",
  "rpc_port": "localhost:9101"
}
```

Additionally, the hose url will also be the base at which the esplora API is hosted at, docs for it can be found here: https://github.com/Blockstream/esplora/blob/master/API.md

Example of calling out to it from curl:
request
```
 curl http://localhost:9102/regtest/api/blocks/tip/hash
```
response
```
 3f4f3ffae9ba83afb207198e6c05dde877afb7677e90439f7bd351f24634dfc5
```
