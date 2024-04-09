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
}
