import { BaseRequestHandler, type NodeRequests } from './nodes';

export interface EclairRequests {
	new (base_url: string, password: string): void;
}

export class EclairRequests implements EclairRequests, NodeRequests {
	requestHandler: BaseRequestHandler;

	constructor(base_url: string, password: string) {
		const encodedCredentials = btoa(':' + password);
		const headers = {
			Authorization: `Basic ${encodedCredentials}`
		};
		const proxy = '/api/proxy';
		this.requestHandler = new BaseRequestHandler(base_url, headers, proxy);
	}
	//API docs: https://acinq.github.io/eclair/#introduction

	async fetchChannels(): Promise<any> {
		let url = `${this.requestHandler.base_url}/channels`;
		return this.requestHandler.send_request(url, 'POST', true);
	}

	async fetchInfo(): Promise<any> {
		let url = `${this.requestHandler.base_url}/getinfo`;
		return this.requestHandler.send_request(url, 'POST', true);
	}

	async fetchBalance(): Promise<any> {
		let url = `${this.requestHandler.base_url}/globalbalance`;
		return this.requestHandler.send_request(url, 'POST', true);
	}

	async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
		let url = `${this.requestHandler.base_url}/node/${pubkey}`;
		return this.requestHandler.send_request(url, 'POST', true);
	}
}
