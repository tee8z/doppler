source ./scripts/aliases.sh

customer_hash=$(customer addinvoice | jq -r '.r_hash')
echo $customer_hash
merchant_hold_invoice_payment_req=$(merchant addholdinvoice $customer_hash 4500 | jq -r '.payment_request')
customer payinvoice --pay_req $merchant_hold_invoice_payment_req -f --timeout 30m0s