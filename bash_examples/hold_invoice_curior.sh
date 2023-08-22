#!/bin/bash

<<COMMENT
What happens:
1) customer makes a payment payment_hash (generate an invoice with no amount to make simple)
2) customer gives the payment_hash to the merchant (not the payment_request or invoice, just the r_hash)
3) merchant makes a hold invoice using that customer payment_hash, the hold invoice's payment_hash will be the same as customer's
4) customer pays that holdinvoice created by the merchant -- money is in flight
5) merchant contacts courier and provides the payment_hash of that hold_invoice
6) courier creates their own hold_invoice from the merchant payment_hash
7) merchant then pays the courier hold_invoice
8) customer recieves the goods, and provides the preimage to the courier 
9) courier settles invoice with merchant and is paid
10) merchant settles invoice with customer and is paid
11) all parties are happy

NOTE: major risk is around how long the flow takes, the longer it takes for the customer to provide 
the courier with the preimag the high liklihood of htlcs timing out
COMMENT

# copy to terminal 1
# customer creates r_hash, merchant creates hold_invoice, customer pays it
source ./scripts/container_aliases.sh
customer_invoice=$(customer addinvoice)
r_hash=$(echo "$customer_invoice" | jq -r '.r_hash')
merchant_hold_invoice=$(merchant addholdinvoice $r_hash 450000)
merchant_hold_invoice_payment_req=$(echo "$merchant_hold_invoice" | jq -r '.payment_request')
customer payinvoice --pay_req $merchant_hold_invoice_payment_req -f --timeout 30m0s


# copy to terminal 2
# customer creates r_hash, merchant creates hold_invoice, customer pays it
source ./scripts/container_aliases.sh
courier addholdinvoice $r_hash 490000)
courier_hold_invoice_payment_req=$(echo "$courier_hold_invoice" | jq -r '.payment_request')
merchant payinvoice --pay_req $courier_hold_invoice_payment_req -f --timeout 30m0s


# copy to terminal 2
# out-of-band
# customer provides the courier with the payment_hash
source ./scripts/container_aliases.sh
customer_pre_image=$(echo customer lookupinvoice  $r_hash | jq -r '.r_preimage')
courier settleinvoice $customer_pre_image
merchant settleinvoice $customer_pre_image
