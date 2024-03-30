#!/bin/bash

source ./scripts/aliases.sh

alice pendingchannels

#get txId 
funding_tx=$(alice listchaintxns | jq -r '.transactions | sort_by(-(.time_stamp | tonumber)) | .[0] | .tx_hash')
outpoints=$(alice listchaintxns | jq -r '.transactions | sort_by(-(.time_stamp | tonumber)) | .[0] | .output_details')
selected_outpoint=$(echo "$outpoints" | jq -r '.[] | select(.is_our_address == true)')
output_index=$(echo "$selected_outpoint" | jq -r '.output_index')
first_transaction="$funding_tx:$output_index"
echo $first_transaction

sleep 2

alice wallet bumpfee --force --sat_per_vbyte 3000 $first_transaction

sleep 2

alice listchaintxns | jq -r '.transactions | sort_by(-(.time_stamp | tonumber)) | .[0]'


funding_tx_2=$(alice listchaintxns | jq -r '.transactions | sort_by(-(.time_stamp | tonumber)) | .[0] | .tx_hash')
outpoints_2=$(alice listchaintxns | jq -r '.transactions | sort_by(-(.time_stamp | tonumber)) | .[0] | .output_details')
selected_outpoint_2=$(echo "$outpoints_2" | jq -r '.[] | select(.is_our_address == true)')
output_index_2=$(echo "$selected_outpoint_2" | jq -r '.output_index')

second_transaction="$funding_tx_2:$output_index_2"
echo $second_transaction

alice wallet bumpfee --force --sat_per_vbyte 8000 $second_transaction

sleep 2

alice wallet bumpfee --force --sat_per_vbyte 10000 $first_transaction

sleep 2

alice listchaintxns | jq -r '.transactions | sort_by(-(.time_stamp | tonumber)) | .[0]'