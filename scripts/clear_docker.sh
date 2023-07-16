#!/bin/bash
docker ps -a --format '{{.ID}} {{.Names}}' | grep "doppler-" | awk '{print $1}' | xargs -r docker rm -f -v