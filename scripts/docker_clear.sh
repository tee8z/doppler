#!/bin/bash
# Remove all containers with names starting with "doppler-"
docker ps -a --format '{{.Names}}' | grep "^doppler-" | xargs -r docker rm -f

# Remove the doppler network
docker network rm doppler

# Remove any dangling volumes
docker volume prune -f
