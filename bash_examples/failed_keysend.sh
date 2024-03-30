#!/bin/bash

# lnd.conf needs to have accept-keysend=false in it, under the [Application Options] section
# connect 3 nodes (alice, bob, carol) with "failed_keysend_setup.doppler" 
# where alice -> bob -> carol

# alice attempts to pay carol with a keysend
# result: 
# carol fails the payment with failure reason FAILURE_REASON_INCORRECT_PAYMENT_DETAILS 
# and failure code 1 (INCORRECT_OR_UNKNOWN_PAYMENT_DETAILS)
source ./scripts/aliases.sh

alice_pk=$(alice getinfo | jq '.identity_pubkey' -r)
carol_pk=$(carol getinfo | jq '.identity_pubkey' -r)
bob_pk=$(carol getinfo | jq '.identity_pubkey' -r)

alice sendpayment --keysend --amt 1000 --dest $carol_pk
carol sendpayment --keysend --amt 1000 --dest $alice_pk


bob_channels=$(bob listchannels)
remote_balance=$(echo "${bob_channels}" | jq -r --arg pubkey "$carol_pk" '.channels[] | select(.remote_pubkey == $pubkey) | .remote_balance')
chan_reserve_sat_remote=$(echo "${bob_channels}" | jq -r --arg pubkey "$carol_pk" '.channels[] | select(.remote_pubkey == $pubkey) | .remote_constraints.chan_reserve_sat')
chan_reserve_sat_local=$(echo "${bob_channels}" | jq -r --arg pubkey "$carol_pk" '.channels[] | select(.remote_pubkey == $pubkey) | .remote_constraints.chan_reserve_sat')
