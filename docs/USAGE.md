### How to run
```
RUST_LOG=TRACE cargo run --bin doppler
```

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

### How to test parse grammar
```
RUST_LOG=TRACE cargo run --bin parsetest
```