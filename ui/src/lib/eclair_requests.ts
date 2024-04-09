import type { NodeRequests } from "./nodes";

export interface EclairRequests {
    base_url: string;
    header: HeadersInit;
    new(base_url: string, password: string): void;
}

export class EclairRequests implements EclairRequests, NodeRequests {
    base_url: string;
    header: HeadersInit;
    proxy: string;

    constructor(base_url: string, password: string) {
        this.base_url = base_url;
        const encodedCredentials = btoa(":" + password);
        this.header = {
            'Authorization': `Basic ${encodedCredentials}`
        };
        this.proxy = '/api/proxy';
    }
    //API docs: https://acinq.github.io/eclair/#introduction

    async fetchChannels(): Promise<any> {
        let url = `${this.base_url}/channels`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { method: "POST", headers, credentials: "include" });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchInfo(): Promise<any> {
        let url = `${this.base_url}/getinfo`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { method: "POST", headers, credentials: "include" });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchBalance(): Promise<any> {
        let url = `${this.base_url}/globalbalance`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { method: "POST", headers, credentials: "include" });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
        let url = `${this.base_url}/node/${pubkey}`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { method: "POST", headers, credentials: "include" });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }
};