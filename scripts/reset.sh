#!/bin/bash
./scripts/docker_clear.sh
./scripts/volumes_clear.sh

RUST_LOG=debug cargo run --bin=doppler