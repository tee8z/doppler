### How to run
```
cargo run --bin doppler "parsetest.doppler"
```
- `parsetest.doppler` is the path to the `.doppler` file to use for the cluster
- Update the file called `parsetest.doppler` to be the setup of a cluster. 
The one part of the repo provides examples of how to use the grammar today

### How to view logs of container
```
docker logs doppler-<node name>
```

### Clear docker cluster
```
./scripts/clear_docker.sh
```

### Clear docker data
```
./scripts/clear_volumes.sh
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

