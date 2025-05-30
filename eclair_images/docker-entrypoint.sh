#!/bin/bash
set -e

# If the first argument is a flag, assume we want to run eclair-node
if [ "${1:0:1}" = '-' ]; then
    set -- eclair-node "$@"
fi

# If running eclair-node, set up the command properly
if [ "$1" = 'eclair-node' ]; then
    shift
    exec eclair-node/bin/eclair-node.sh \
        "-Declair.datadir=${ECLAIR_DATADIR}" \
        ${JAVA_OPTS} \
        "$@"
fi

# If running eclair-cli, just pass through
if [ "$1" = 'eclair-cli' ]; then
    exec /sbin/eclair-cli "$@"
fi

# Otherwise, execute whatever command was passed
exec "$@"
