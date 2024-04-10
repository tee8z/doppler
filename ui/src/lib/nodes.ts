export interface NodeData {
    channels: any;
    info: any;
    balance: any;
    type: string;
}

export interface Nodes {
    [key: string]: NodeData;
}

export interface NodeRequests {
    fetchChannels(): Promise<any>
    fetchInfo(): Promise<any>
    fetchBalance(): Promise<any>
    fetchSpecificNodeInfo(pubkey: String): Promise<any>
    requestHandler: BaseRequestHandler
}


export class BaseRequestHandler {
    base_url: string;
    header: HeadersInit;
    proxy: string;

    constructor(base_url: string, headers: HeadersInit, proxy: string) {
        this.base_url = base_url;
        this.header = headers
        this.proxy = proxy;
    }

    async send_request(url: string, method: string, include_creds: boolean): Promise<any> {
        if (this.base_url.includes("localhost")) {
            let headers = { ...this.header, 'target': url };
            const response = await fetch(this.proxy, { headers, method: method, credentials: include_creds ? "include" : "same-origin" });
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            return await response.json();
        } else {
            let headers = { ...this.header };
            const response = await fetch(url, { headers, method: method, credentials: include_creds ? "include" : "same-origin" });
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            return await response.json();
        }
    }
}