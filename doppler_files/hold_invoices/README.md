#### Steps for using the simulation
1. Run "hold_invoice_setup.doppler" -- Spins up a cluster of nodes with liquidity in the correct places for the invoice to work
2. Run "create_hold_invoice.doppler" -- Creates a hodl invoice from "customer" to "merchant" which will act as our invoice that is "inflight" along the channels
3. Run "settle_hold_invoice.doppler" -- This provides the preimage from the "customer" to "merchant" allowing them to settle their hodl invoice and everything be handled gracefully
