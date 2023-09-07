source ./scripts/container_aliases.sh

customer_list=$(customer listpayments --include_incomplete | jq -r '.payments |sort_by(-(.creation_time_ns | tonumber))' )
customer_payment_hash=$(echo $customer_list  | jq -r ' .[0].payment_hash')
echo $customer_payment_hash
courier_hold_invoice_payment_req=$(courier addholdinvoice $customer_payment_hash 4900 | jq -r '.payment_request')
merchant payinvoice --pay_req $courier_hold_invoice_payment_req -f --timeout 30m0s