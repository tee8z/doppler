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
			const url = `${this.requestHandler.base_url}/v1/listpeerchannels`;
			const data = await this.requestHandler.send_request(url, 'POST', false);
			if (!data || !data.channels) {
				return [];
			}
			return data.channels;
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
			return data;
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
