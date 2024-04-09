import type { NodeRequests } from "./nodes";

export interface CorelnRequests {
    base_url: string;
    header: HeadersInit;
    new(base_url: string, macaroon: string): void;
}

export class CorelnRequests implements CorelnRequests, NodeRequests {
    base_url: string;
    header: HeadersInit;
    proxy: string;

    constructor(base_url: string, macaroon: string) {
        this.base_url = base_url;
        this.header = {
            'macaroon': macaroon,
            'encodingtype': 'hex'
        };
        this.proxy = '/api/proxy';
    }
    //API docs: https://github.com/Ride-The-Lightning/c-lightning-REST

    async fetchChannels(): Promise<any> {
        let url = `${this.base_url}/v1/channel/listChannels`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchInfo(): Promise<any> {
        let url = `${this.base_url}/v1/getinfo`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchBalance(): Promise<any> {
        let url = `${this.base_url}/v1/getBalance`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
        let url = `${this.base_url}/v1/network/listNode/${pubkey}`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }
};