import { BaseRequestHandler, type NodeRequests } from './nodes';

export interface CorelnRequests {
	new (base_url: string, macaroon: string): void;
}

export class CorelnRequests implements CorelnRequests, NodeRequests {
	requestHandler: BaseRequestHandler;

	constructor(base_url: string, rune: string) {
		const header = {
			Rune: rune
		};
		const proxy = '/api/proxy';
		this.requestHandler = new BaseRequestHandler(base_url, header, proxy);
	}
	//API docs: https://docs.corelightning.org/reference/get_list_methods_resource
	// look at the commands in "JSON-RPC API Reference" and pass them via /v1/{command}

	async fetchChannels(): Promise<any> {
		try {
			const url = `${this.requestHandler.base_url}/v1/listpeers`;
			const data = await this.requestHandler.send_request(url, 'POST', false);
			console.log(data);
			if (!data || !data.peers) {
				return [];
			}
			const filteredPeers = data.peers.filter(
				(peer: any) => peer.channels && peer.channels.length > 0
			);

			const chanList = await Promise.all(
				filteredPeers.map((peer: any) => getAliasForChannels(peer, this.fetchSpecificNodeInfo))
			);
			console.log(chanList);
			return chanList.flatMap((chan) => chan);
		} catch (err: any) {
			console.error(err);
			return {
				status: 500,
				error: err.message || 'An error occurred while fetching the channels'
			};
		}
	}

	async fetchInfo(): Promise<any> {
		let url = `${this.requestHandler.base_url}/v1/getinfo`;
		return await this.requestHandler.send_request(url, 'POST', false);
	}

	async fetchBalance(): Promise<any> {
		try {
			const url = `${this.requestHandler.base_url}/v1/listfunds`;
			const data = await this.requestHandler.send_request(url, 'POST', false);

			const opArray = data.outputs;
			let confBalance = 0;
			let unconfBalance = 0;

			for (const output of opArray) {
				if (output.status === 'confirmed') {
					confBalance += output.amount_msat;
				} else if (output.status === 'unconfirmed') {
					unconfBalance += output.amount_msat / 1000;
				}
			}

			const totalBalance = confBalance + unconfBalance;

			return {
				totalBalance,
				confBalance,
				unconfBalance
			};
		} catch (err: any) {
			console.error(err);
			return {
				status: 500,
				error: err.message || 'An error occurred while fetching the balance'
			};
		}
	}
	async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
		let url = `${this.requestHandler.base_url}/v1/listnodes?id=${pubkey}`;
		return await this.requestHandler.send_request(url, 'POST', false);
	}
}

const getAliasForChannels = (peer: any, fetchSpecificNodeInfo: (id: string) => Promise<any>) => {
	return new Promise(function (resolve, reject) {
		fetchSpecificNodeInfo(peer.id)
			.then((data: any) => {
				resolve(
					peer.channels
						.filter((c: any) => c.state !== 'ONCHAIN' && c.state !== 'CLOSED')
						.reduce((acc: any, channel: any) => {
							const TO_US_MSATS = channel.msatoshi_to_us || channel.to_us_msat;
							const TOTAL_MSATS = channel.msatoshi_total || channel.total_msat;
							acc.push({
								id: peer.id,
								alias: data.nodes[0] ? data.nodes[0].alias : peer.id,
								connected: peer.connected,
								state: channel.state,
								short_channel_id: channel.short_channel_id,
								channel_id: channel.channel_id,
								funding_txid: channel.funding_txid,
								private: channel.private,
								msatoshi_to_us: TO_US_MSATS,
								msatoshi_total: TOTAL_MSATS,
								msatoshi_to_them: TOTAL_MSATS - TO_US_MSATS,
								their_channel_reserve_satoshis:
									channel.their_channel_reserve_satoshis || channel.their_reserve_msat,
								our_channel_reserve_satoshis:
									channel.our_channel_reserve_satoshis || channel.our_reserve_msat,
								spendable_msatoshi: channel.spendable_msatoshi || channel.spendable_msat,
								funding_allocation_msat: channel.funding_allocation_msat,
								opener: channel.opener,
								direction: channel.direction,
								htlcs: channel.htlcs
							});
							return acc;
						}, [])
				);
			})
			.catch((err: any) => {
				console.error(err);
				resolve(
					peer.channels
						.filter((c: any) => c.state !== 'ONCHAIN' && c.state !== 'CLOSED')
						.reduce((acc: any, channel: any) => {
							const TO_US_MSATS = channel.msatoshi_to_us || channel.to_us_msat;
							const TOTAL_MSATS = channel.msatoshi_total || channel.total_msat;
							acc.push({
								id: peer.id,
								alias: peer.id,
								connected: peer.connected,
								state: channel.state,
								short_channel_id: channel.short_channel_id,
								channel_id: channel.channel_id,
								funding_txid: channel.funding_txid,
								private: channel.private,
								msatoshi_to_us: TO_US_MSATS,
								msatoshi_total: TOTAL_MSATS,
								msatoshi_to_them: TOTAL_MSATS - TO_US_MSATS,
								their_channel_reserve_satoshis:
									channel.their_channel_reserve_satoshis || channel.their_reserve_msat,
								our_channel_reserve_satoshis:
									channel.our_channel_reserve_satoshis || channel.our_reserve_msat,
								spendable_msatoshi: channel.spendable_msatoshi || channel.spendable_msat,
								funding_allocation_msat: channel.funding_allocation_msat,
								opener: channel.opener,
								direction: channel.direction,
								htlcs: channel.htlcs
							});
							return acc;
						}, [])
				);
			});
	});
};
