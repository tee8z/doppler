#!/bin/bash

source ./scripts/container_aliases.sh

#pubkey open channel to
customer getinfo | jq -r 'identity_pubkey')

#funding_tx
router1 openchannel --node_key $pubkey --local_amt 900000 | jq -r 'funding_txid')

# repeate the following 2 step to create as many CPFP fee bumps as you'd like

#the outpoint is either 1 or 2
router1 wallet bumpfee --force --sat_per_vbyte 1000 $fundind_tx:(1|0)

#get list of chain txns sort by created time
# grab the tx_hash of the sweep if you want to make chain
# grab the tx_hash of the orignial open to make a new branch of txs
router1 listchaintxns | jq '.transactions | sort_by(.time_stamp | tonumber)'

#generate at least one block to have a transaction chain for the channel picked
#mine more blocks if you want to see the channel also confirmed
#after 1 block, fee bumping doesn't matter (it's a mempool concern)
bd1 -generate 1
