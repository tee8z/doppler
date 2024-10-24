#### Steps for using the simulation
1. Run "simple_network.doppler" -- Spins up a cluster of nodes with liquidity in the correct places for the invoice to work
2. Run "hold_invoice.doppler" -- Creates a hodl invoice from "sender" to "problem" which will act as our invoice that is "inflight" along the channels
3. Run "problem_offline.doppler" -- Kills the node at the receiving end of the payment and thus starts the clock until the htlc's expire and we have a problem
4. Run "expire_htlcs.doppler" -- Mines enough blocks to cause all of the htlcs in-flight to expire and force the channels involved in the hodl-invoice to resolve on chain, with one payment we watch as our whole cluster's channels are kicked off
