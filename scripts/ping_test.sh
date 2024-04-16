#!/bin/bash
# Script to ping for 'synced_to_chain', make sure to update 'alias_count' to be the total number of lnd nodes in the given cluster before running
# This is useful for performance/benchmark testing lnd pulling in on-chain data

SCRIPT_DIR=$(dirname "${BASH_SOURCE[0]}")

# Source the aliases file
source $SCRIPT_DIR/aliases.sh
alias_count=50

# Enable alias expansion
shopt -s expand_aliases

# Function to ping a single LND and check synced_to_chain status
ping_lnd() {
    local lnd_alias=$1
    local attempt_count=0
    local failure_count=0
    local total_attempts=0
    local start_time=$(date +%s)
    local end_time=$((start_time + 300)) # 5 minutes from now

    while [ $(date +%s) -lt $end_time ]; do
        attempt_count=$((attempt_count + 1))
        # Use the alias to get the synced_to_chain status
        response=$($lnd_alias getinfo | jq '.synced_to_chain')
        if [ "$response" == "false" ]; then
            failure_count=$((failure_count + 1))
            echo -n "x"
        else
            echo -n "."
        fi
        total_attempts=$((total_attempts + 1))
        sleep 1 # Adjust the sleep duration as needed
    done

    local percentage=$(( (failure_count * 100) / total_attempts ))
    echo "LND Alias: $lnd_alias"
    echo "Total Attempts: $total_attempts"
    echo "Failures: $failure_count"
    echo "Percentage of Failures: $percentage%"
}

# Ping each LND concurrently
for ((i=1; i<=$alias_count; i++)); do
    ping_lnd "lnd$i" &
done

# Wait for all background processes to finish
wait