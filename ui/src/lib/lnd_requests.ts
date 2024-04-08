export interface ConnectionConfig {
    macaroon: string;
    tls: string;
    host: string;
}

export interface Connections {
    [key: string]: ConnectionConfig;
}

export interface LndRequests {
    base_url: string;
    header: HeadersInit;
    tls: string;
    new(base_url: string, macaroon: string, tls: any): void;
    fetchGraph(url: string): Promise<any>;
}

export interface NodeData {
    graph: any;
    channels: any;
    info: any;
    balance: any;
}

export interface Nodes {
    [key: string]: NodeData;
}

export class LndRequests implements LndRequests {
    base_url: string;
    header: HeadersInit;
    tls: string;
    proxy: string;

    constructor(base_url: string, macaroon: string, tls: string) {
        this.base_url = base_url;
        this.header = {
            'Grpc-Metadata-macaroon': macaroon,
        };
        this.tls = tls;
        this.proxy = '/api/proxy';
    }

    async fetchGraph(): Promise<any> {
        let url = `${this.base_url}/v1/graph/info`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchChannels(): Promise<any> {
        let url = `${this.base_url}/v1/channels`
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
        let url = `${this.base_url}/v1/balance/blockchain`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }

    async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
        let url = `${this.base_url}/v1/graph/node/${pubkey}`
        let headers = { ...this.header, 'target': url };
        const response = await fetch(this.proxy, { headers });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    }
};

export async function getConnections(): Promise<Connections> {
    try {
        const response = await fetch('/api/connections');
        if (!response.ok) {
            throw new Error('Failed to fetch connections');
        }
        const connections = await response.json();
        return connections
    } catch (error) {
        console.error('Error fetching connections:', error);
    }
    return {}
}