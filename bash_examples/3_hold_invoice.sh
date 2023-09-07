source ./scripts/container_aliases.sh

customer_list=$(customer listpayments --include_incomplete | jq -r '.payments |sort_by(-(.creation_time_ns | tonumber))' )
customer_payment_hash=$(echo $customer_list  | jq -r ' .[0].payment_hash')
echo $customer_payment_hash
customer_r_preimage=$(customer lookupinvoice  $customer_payment_hash | jq -r '.r_preimage')
echo $customer_r_preimage
courier settleinvoice $customer_r_preimage
merchant settleinvoice $customer_r_preimage