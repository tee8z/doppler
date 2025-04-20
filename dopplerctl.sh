#!/bin/bash

SCRIPT=${BASH_SOURCE[0]}
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

# Host environment
DOPPLER_PREREQUISITES="docker doppler"
DOPPLER_HOME="${DOPPLER_HOME:-$(pwd)}"
DOPPLER_UID="${DOPPLER_UID:-1000}"
DOPPLER_GID="${DOPPLER_GID:-1000}"

# Doppler environment
DOPPLER_CLUSTER="${DOPPLER_HOME}/doppler-cluster.yaml"
DOPPLER_NETWORK=signet
DOPPLER_ALIAS_COUNT=8


# Enable alias expansion
shopt -s expand_aliases


###############################
## BEGIN dopplerctl subcommands
##
## Functions named: doppler_<subcommand>()
## each with preceeding two lines as comments used for 'help' output.
## For blank help output, leave '##' with no comment (needs two chars)
##
## Self-generated list of subcommands, in the order they appear in this script:
SUBCOMMANDS=$(grep '^doppler_.*{' $SCRIPT | sed -e 's/^doppler_//' -e 's/() {$//')

# Execute lncli command
# <node_index> <lncli args>
doppler_lnd() {
    # First argument is the node index
    index=$1; shift
    # Remaining arguments are passed to lncli
    docker compose -f $DOPPLER_CLUSTER exec --user $DOPPLER_UID:$DOPPLER_GID doppler-lnd-lnd$index lncli --lnddir=/home/lnd/.lnd --network=$DOPPLER_NETWORK --macaroonpath=/home/lnd/.lnd/data/chain/bitcoin/$DOPPLER_NETWORK/admin.macaroon --rpcserver=localhost:10000 "$@"
}

# Execute bitcoin-cli command
# <node_index> <bitcoin-cli args>
doppler_bd() {
    # First argument is the node index
    index=$1; shift
    # Remaining arguments are passed to bitcoin-cli
    docker compose -f $DOPPLER_CLUSTER exec --user $DOPPLER_UID:$DOPPLER_GID doppler-bitcoind-miner-bd$index bitcoin-cli "$@"
}

# Useful for performance/benchmark testing lnd pulling in on-chain data
# Ping a single LND and check synced_to_chain status
# <node_index>
doppler_ping_lnd() {
    if [ $# -lt 1 ]; then
        echo "Usage: dopplerctl ping_lnd <node_index>"
        return 1
    fi
    local lnd_alias=$1
    local attempt_count=0
    local failure_count=0
    local total_attempts=0
    local start_time=$(date +%s)
    local end_time=$((start_time + 300)) # 5 minutes from now

    while [ $(date +%s) -lt $end_time ]; do
        attempt_count=$((attempt_count + 1))
        # Use the alias to get the synced_to_chain status
        response=$(doppler_lnd $lnd_alias getinfo | jq '.synced_to_chain')
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
##
doppler_ping_all() {
    for ((i=1; i<=$DOPPLER_ALIAS_COUNT; i++)); do
        doppler_ping_lnd $i &
    done
    
    # Wait for all background processes to finish
    wait
}

# Delete all doppler containers
##
doppler_clear_containers() {
    echo "Deleting all doppler docker containers..."
    docker ps -a --format '{{.ID}} {{.Names}}' | grep "doppler-" | awk '{print $1}' | xargs -r docker rm -f -v
}

# Delete all doppler volumes data
##
doppler_clear_volumes() {
    echo "Deleting all doppler volumes data..."
    rm -rf "$DOPPLER_HOME/data"
}

# Delete all doppler containers & volumes data
##
doppler_reset() {
    doppler_clear_containers
    doppler_clear_volumes
    #[ -e "$DOPPLER_HOME/doppler.db" ] && rm "$DOPPLER_HOME/doppler.db"
    rm "$DOPPLER_HOME/doppler.db" 2>/dev/null
}

# Convert hexadecimal to binary and then decode Base64 to ASCII
# <hex_string>
doppler_hex_to_ascii() {
    [ -z "$1" ] && { echo "Usage: convert_hex_to_ascii <hex_string>"; return 1; }

    hex_string="$1"
    for byte in $(echo $hex_string | fold -w2); do
        echo -n -e "\x$byte\n"
    done
}

# Install dopplerctl shell completion
# <shell_type>
doppler_completion() {
  local shell_type="$1"
####  bash-completion BEGIN  ####
  local bash_completion_script='#!/usr/bin/env bash
_dopplerctl_completions()
{
  local cur prev opts
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"
  opts="'"$(echo $SUBCOMMANDS)"'"

  # Only complete the first arg, to prevent repeating the same one forever
  if [ "${#COMP_WORDS[@]}" -lt "3" ]; then
    COMPREPLY=($(compgen -W "${opts}" -- ${cur}))
  fi
}
complete -F _dopplerctl_completions dopplerctl'
####  bash-completion END  ####
  chmod +x $SCRIPT
  if [ "$shell_type" == 'bash' ]; then
    echo "$bash_completion_script"
  else
    echo "Usage: source <(dopplerctl completion bash)"
    return 0
  fi
}

# Print this help message
##
doppler_help() {
    echo "Usage: dopplerctl <subcommand> [<args>]"

    # Print generated list of: subcommand <args> <description>
    printf "\n%-18s %-32s %-60s\n" "Subcommands:" "ARGS" "DESCRIPTION"
    for cmd in $SUBCOMMANDS; do
        blob=$(grep -B2 "^doppler_$cmd" "$SCRIPT")
        description=$(echo "$blob" | head -1 | sed 's/^#.//')
        args=$(echo "$blob" | head -2 | tail -1 | sed 's/^#.//')
        printf "%18s %-32s %-60s\n" "$cmd" "$args" "$description"
    done

    echo -e "\nEnvironment variables, override to configure:"
    echo "  DOPPLER_HOME=$DOPPLER_HOME"
    echo "  DOPPLER_UID=$DOPPLER_UID"
    echo "  DOPPLER_GID=$DOPPLER_GID"
    echo "  DOPPLER_CLUSTER=$DOPPLER_CLUSTER"
    echo "  DOPPLER_NETWORK=$DOPPLER_NETWORK"
    echo "  DOPPLER_ALIAS_COUNT=$DOPPLER_ALIAS_COUNT"
}

## END dopplerctl subcommands
#############################


## dopplerctl main entrypoint
dopplerctl() {
    for cmd in $DOPPLER_PREREQUISITES; do
        if ! command -v $cmd &>/dev/null; then
            echo "Missing prerequisite, could not find '$cmd' installed"
            return 1
        fi
    done
    
    # If no args are provided, display usage instructions then exit
    [ $# -lt 1 ] && { doppler_help; return 0; }
    
    # Otherwise treat the first arg as subcommand
    SUBCOMMAND=$1
    shift
    
    # Pass any remaining args through to the subcommand
    doppler_$SUBCOMMAND "$@"
}

## If script was executed instead of source, print intended usage
if [ -n "$ZSH_VERSION" ]; then 
    case $ZSH_EVAL_CONTEXT in *:file:*) return 0;; esac
else  # Add additional POSIX-compatible shell names here, if needed.
    case ${0##*/} in dash|-dash|bash|-bash|ksh|-ksh|sh|-sh) return 0;; esac
fi
echo "This script is intended to be sourced instead of executed directly."
echo
echo "Usage: source $0 && dopplerctl && source <(dopplerctl completion bash)"

